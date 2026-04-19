use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use base64::Engine;
use hmanga_core::{
    download::ExportRunner, AppConfig, Comic, DynPlugin, HostApi, HttpMethod, HttpRequest,
    HttpResponse, NiuhuanCompat, SiteConfig,
};
use hmanga_host::HostRuntime;
use hmanga_plugin_jm::{JmPlugin, JmUserProfile, JmWeeklyInfo, ProcessedImage};
use hmanga_plugin_wnacg::{WnacgPlugin, WnacgUserProfile};
use tokio::sync::Semaphore;
use tokio::task::JoinSet;
use tokio::time::sleep;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LocalChapterEntry {
    pub comic_id: String,
    pub comic_title: String,
    pub chapter_id: String,
    pub chapter_title: String,
    pub chapter_dir: PathBuf,
    pub pages: Vec<PathBuf>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LocalComicEntry {
    pub comic: Comic,
    pub comic_dir: PathBuf,
    pub chapters: Vec<LocalChapterEntry>,
    pub platform_tag: Option<String>,
    pub download_time: Option<u64>,
    pub update_time: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FavoritePage {
    pub comics: Vec<Comic>,
    pub current_page: u32,
    pub total_pages: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SearchPage {
    pub comics: Vec<Comic>,
    pub current_page: u32,
    pub total_pages: u32,
}

#[derive(Clone)]
pub struct AppServices {
    host: Arc<Mutex<HostRuntime>>,
    jm: Arc<Mutex<JmPlugin>>,
    wnacg: Arc<Mutex<WnacgPlugin>>,
    wnacg_session: Arc<Mutex<Option<hmanga_core::Session>>>,
    plugins: Arc<Mutex<HashMap<String, Arc<dyn DynPlugin>>>>,
    config: Arc<Mutex<AppConfig>>,
    config_path: PathBuf,
    chapter_gate: Arc<Mutex<Arc<Semaphore>>>,
    task_controls: Arc<Mutex<HashMap<String, Arc<DownloadControl>>>>,
}

impl AppServices {
    pub fn new() -> Self {
        let config_dir = resolve_config_dir();
        Self::new_with_paths(
            config_dir,
            PathBuf::from("/Users/fwmbam4/Downloads/books/下载"),
        )
        .expect("failed to initialize app services")
    }

    pub fn new_with_paths(
        config_dir: PathBuf,
        default_download_dir: PathBuf,
    ) -> Result<Self, String> {
        fs::create_dir_all(&config_dir).map_err(|err| err.to_string())?;
        let config_path = config_dir.join("config.json");
        let config = load_or_init_config(&config_path, default_download_dir)?;
        fs::create_dir_all(&config.download_dir).map_err(|err| err.to_string())?;
        fs::create_dir_all(&config.export_dir).map_err(|err| err.to_string())?;
        let chapter_concurrency = config.chapter_concurrency.max(1);

        Ok(Self {
            host: Arc::new(Mutex::new(build_host_runtime(&config)?)),
            jm: Arc::new(Mutex::new(build_jm_plugin(&config))),
            wnacg: Arc::new(Mutex::new(build_wnacg_plugin(&config))),
            wnacg_session: Arc::new(Mutex::new(None)),
            plugins: Arc::new(Mutex::new(build_plugin_registry(&config))),
            config: Arc::new(Mutex::new(config)),
            config_path,
            chapter_gate: Arc::new(Mutex::new(Arc::new(Semaphore::new(chapter_concurrency)))),
            task_controls: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    pub fn config_path(&self) -> &Path {
        &self.config_path
    }

    pub fn config(&self) -> AppConfig {
        self.config.lock().unwrap().clone()
    }

    fn host(&self) -> HostRuntime {
        self.host.lock().unwrap().clone()
    }

    fn jm(&self) -> JmPlugin {
        self.jm.lock().unwrap().clone()
    }

    fn wnacg(&self) -> WnacgPlugin {
        self.wnacg.lock().unwrap().clone()
    }

    pub fn save_jm_credentials(&self, username: &str, password: &str) -> Result<(), String> {
        let mut config = self.config();
        config.jm_username = username.to_string();
        config.jm_password = password.to_string();
        self.save_config(&config)
    }

    pub fn save_config(&self, config: &AppConfig) -> Result<(), String> {
        fs::create_dir_all(&config.download_dir).map_err(|err| err.to_string())?;
        fs::create_dir_all(&config.export_dir).map_err(|err| err.to_string())?;
        persist_config(&self.config_path, config)?;
        *self.config.lock().unwrap() = config.clone();
        *self.host.lock().unwrap() = build_host_runtime(config)?;
        *self.jm.lock().unwrap() = build_jm_plugin(config);
        *self.wnacg.lock().unwrap() = build_wnacg_plugin(config);
        *self.plugins.lock().unwrap() = build_plugin_registry(config);
        *self.chapter_gate.lock().unwrap() =
            Arc::new(Semaphore::new(config.chapter_concurrency.max(1)));
        Ok(())
    }

    pub async fn search_aggregate(&self, query: &str) -> Result<SearchPage, String> {
        let enabled_plugins = self.config().enabled_plugins;
        let plugins = self.plugins.lock().unwrap().clone();
        let host = self.host();
        search_aggregate_plugins(&plugins, &host, &enabled_plugins, query, 1).await
    }

    pub async fn search_aggregate_page(
        &self,
        query: &str,
        page: u32,
    ) -> Result<SearchPage, String> {
        let enabled_plugins = self.config().enabled_plugins;
        let plugins = self.plugins.lock().unwrap().clone();
        let host = self.host();
        search_aggregate_plugins(&plugins, &host, &enabled_plugins, query, page).await
    }

    pub async fn search_wnacg(&self, query: &str) -> Result<SearchPage, String> {
        let wnacg = self.wnacg();
        let host = self.host();
        wnacg
            .search(&host, query, 1, hmanga_core::SearchSort::Latest)
            .await
            .map(|result| SearchPage {
                comics: result.comics,
                current_page: result.current_page,
                total_pages: result.total_pages,
            })
            .map_err(|err| err.to_string())
    }

    pub async fn search_wnacg_page(&self, query: &str, page: u32) -> Result<SearchPage, String> {
        let wnacg = self.wnacg();
        let host = self.host();
        wnacg
            .search(&host, query, page, hmanga_core::SearchSort::Latest)
            .await
            .map(|result| SearchPage {
                comics: result.comics,
                current_page: result.current_page,
                total_pages: result.total_pages,
            })
            .map_err(|err| err.to_string())
    }

    pub async fn load_wnacg_comic(&self, comic_id: &str) -> Result<Comic, String> {
        let wnacg = self.wnacg();
        let host = self.host();
        wnacg
            .get_comic(&host, comic_id)
            .await
            .map_err(|err| err.to_string())
    }

    pub async fn login_wnacg(
        &self,
        username: &str,
        password: &str,
    ) -> Result<WnacgUserProfile, String> {
        let wnacg = self.wnacg();
        let host = self.host();
        let session = wnacg
            .login(&host, username, password)
            .await
            .map_err(|err| err.to_string())?;
        *self.wnacg_session.lock().unwrap() = Some(session.clone());
        let wnacg = self.wnacg();
        let host = self.host();
        wnacg
            .get_user_profile(&host, &session)
            .await
            .map_err(|err| err.to_string())
    }

    #[allow(dead_code)]
    pub async fn get_wnacg_favorites_page(
        &self,
        folder_id: i64,
        page: u32,
    ) -> Result<FavoritePage, String> {
        let session = self.wnacg_session.lock().unwrap().clone();
        let session = session.ok_or_else(|| "未登录wnacg".to_string())?;
        let wnacg = self.wnacg();
        let host = self.host();
        wnacg
            .get_favorites(&host, &session, folder_id, page)
            .await
            .map(|result| FavoritePage {
                comics: result.comics,
                current_page: result.current_page,
                total_pages: result.total_pages,
            })
            .map_err(|err| err.to_string())
    }

    pub async fn search_jm(&self, query: &str) -> Result<SearchPage, String> {
        let jm = self.jm();
        let host = self.host();
        jm.search(&host, query, 1, hmanga_core::SearchSort::Latest)
            .await
            .map(|result| SearchPage {
                comics: result.comics,
                current_page: result.current_page,
                total_pages: result.total_pages,
            })
            .map_err(|err| err.to_string())
    }

    pub async fn search_jm_page(&self, query: &str, page: u32) -> Result<SearchPage, String> {
        let jm = self.jm();
        let host = self.host();
        jm.search(&host, query, page, hmanga_core::SearchSort::Latest)
            .await
            .map(|result| SearchPage {
                comics: result.comics,
                current_page: result.current_page,
                total_pages: result.total_pages,
            })
            .map_err(|err| err.to_string())
    }

    pub async fn load_jm_comic(&self, comic_id: &str) -> Result<Comic, String> {
        let jm = self.jm();
        let host = self.host();
        jm.get_comic(&host, comic_id)
            .await
            .map_err(|err| err.to_string())
    }

    pub async fn load_comic(&self, source: &str, comic_id: &str) -> Result<Comic, String> {
        match source {
            "wnacg" => self.load_wnacg_comic(comic_id).await,
            _ => self.load_jm_comic(comic_id).await,
        }
    }

    pub async fn read_chapter_online(
        &self,
        source: &str,
        chapter: &hmanga_core::ChapterInfo,
        mut on_progress: impl FnMut(u32, u32, &str),
    ) -> Result<Vec<String>, String> {
        match source {
            "wnacg" => {
                self.read_wnacg_chapter_online(chapter, &mut on_progress)
                    .await
            }
            _ => self.read_jm_chapter_online(chapter, &mut on_progress).await,
        }
    }

    pub async fn login_jm(&self, username: &str, password: &str) -> Result<JmUserProfile, String> {
        let jm = self.jm();
        let host = self.host();
        jm.login(&host, username, password)
            .await
            .map_err(|err| err.to_string())?;
        let jm = self.jm();
        let host = self.host();
        jm.get_user_profile(&host)
            .await
            .map_err(|err| err.to_string())
    }

    pub async fn get_jm_favorites_page(&self, page: u32) -> Result<FavoritePage, String> {
        let jm = self.jm();
        let host = self.host();
        jm.get_favorites(&host, 0, page)
            .await
            .map(|result| FavoritePage {
                comics: result.comics,
                current_page: result.current_page,
                total_pages: result.total_pages,
            })
            .map_err(|err| err.to_string())
    }

    pub async fn get_jm_weekly_info(&self) -> Result<JmWeeklyInfo, String> {
        let jm = self.jm();
        let host = self.host();
        jm.get_weekly_info(&host)
            .await
            .map_err(|err| err.to_string())
    }

    pub async fn get_jm_weekly(
        &self,
        category_id: &str,
        type_id: &str,
    ) -> Result<Vec<Comic>, String> {
        let jm = self.jm();
        let host = self.host();
        jm.get_weekly(&host, category_id, type_id)
            .await
            .map(|result| result.comics)
            .map_err(|err| err.to_string())
    }

    async fn read_jm_chapter_online(
        &self,
        chapter: &hmanga_core::ChapterInfo,
        on_progress: &mut impl FnMut(u32, u32, &str),
    ) -> Result<Vec<String>, String> {
        let config = self.config();
        let host = self.host();
        let jm = self.jm();
        let images = jm
            .get_chapter_images(&host, &chapter.id)
            .await
            .map_err(|err| err.to_string())?;
        if images.is_empty() {
            return Err("章节没有可用图片。".to_string());
        }

        let image_concurrency = config.image_concurrency.max(1);
        let total_pages = images.len() as u32;
        let mut pages = vec![String::new(); images.len()];
        let mut next_to_spawn = 0usize;
        let mut completed = 0u32;
        let mut join_set = JoinSet::new();

        while next_to_spawn < image_concurrency.min(images.len()) {
            let image = images[next_to_spawn].clone();
            spawn_jm_reader_task(
                &mut join_set,
                host.clone(),
                jm.clone(),
                image,
                next_to_spawn,
            );
            next_to_spawn += 1;
        }

        while let Some(joined) = join_set.join_next().await {
            let (index, current_name, page_src) = joined.map_err(|err| err.to_string())??;
            pages[index] = page_src;
            completed += 1;
            on_progress(completed, total_pages, &current_name);

            if config.image_download_interval_sec > 0 {
                sleep(Duration::from_secs(config.image_download_interval_sec)).await;
            }

            if next_to_spawn < images.len() {
                let image = images[next_to_spawn].clone();
                spawn_jm_reader_task(
                    &mut join_set,
                    host.clone(),
                    jm.clone(),
                    image,
                    next_to_spawn,
                );
                next_to_spawn += 1;
            }
        }

        Ok(pages)
    }

    async fn read_wnacg_chapter_online(
        &self,
        chapter: &hmanga_core::ChapterInfo,
        on_progress: &mut impl FnMut(u32, u32, &str),
    ) -> Result<Vec<String>, String> {
        let config = self.config();
        let host = self.host();
        let wnacg = self.wnacg();
        let images = wnacg
            .get_chapter_images(&host, &chapter.id)
            .await
            .map_err(|err| err.to_string())?;
        if images.is_empty() {
            return Err("章节没有可用图片。".to_string());
        }

        let image_concurrency = config.image_concurrency.max(1);
        let total_pages = images.len() as u32;
        let mut pages = vec![String::new(); images.len()];
        let mut next_to_spawn = 0usize;
        let mut completed = 0u32;
        let mut join_set = JoinSet::new();

        while next_to_spawn < image_concurrency.min(images.len()) {
            let image = images[next_to_spawn].clone();
            spawn_passthrough_reader_task(&mut join_set, host.clone(), image, next_to_spawn);
            next_to_spawn += 1;
        }

        while let Some(joined) = join_set.join_next().await {
            let (index, current_name, page_src) = joined.map_err(|err| err.to_string())??;
            pages[index] = page_src;
            completed += 1;
            on_progress(completed, total_pages, &current_name);

            if config.image_download_interval_sec > 0 {
                sleep(Duration::from_secs(config.image_download_interval_sec)).await;
            }

            if next_to_spawn < images.len() {
                let image = images[next_to_spawn].clone();
                spawn_passthrough_reader_task(&mut join_set, host.clone(), image, next_to_spawn);
                next_to_spawn += 1;
            }
        }

        Ok(pages)
    }

    pub async fn download_jm_chapter(
        &self,
        comic: &Comic,
        chapter: &hmanga_core::ChapterInfo,
        mut on_progress: impl FnMut(u32, u32, &str),
    ) -> Result<LocalChapterEntry, String> {
        let control = self.download_control(&chapter.id);
        let config = self.config();
        let host = self.host();
        let jm = self.jm();
        let chapter_gate = self.chapter_gate.lock().unwrap().clone();
        let _permit = chapter_gate
            .acquire_owned()
            .await
            .map_err(|err| err.to_string())?;
        let (comic_dir, chapter_dir) = build_jm_download_dirs(&config.download_dir, comic, chapter);
        fs::create_dir_all(&chapter_dir).map_err(|err| err.to_string())?;

        let images = jm
            .get_chapter_images(&host, &chapter.id)
            .await
            .map_err(|err| err.to_string())?;
        let total_pages = images.len() as u32;
        let image_concurrency = config.image_concurrency.max(1);
        let mut next_to_spawn = 0usize;
        let mut completed = 0u32;
        let mut join_set = JoinSet::new();

        while next_to_spawn < image_concurrency.min(images.len()) {
            let image = images[next_to_spawn].clone();
            spawn_image_task(
                &mut join_set,
                host.clone(),
                jm.clone(),
                image,
                next_to_spawn,
            );
            next_to_spawn += 1;
        }

        while let Some(joined) = join_set.join_next().await {
            control.wait_until_active().await?;
            let (index, _current_name, processed) = joined.map_err(|err| err.to_string())??;
            let filename = format!("{:04}.{}", index + 1, processed.extension);
            fs::write(chapter_dir.join(&filename), processed.bytes)
                .map_err(|err| err.to_string())?;
            completed += 1;
            on_progress(completed, total_pages, &filename);

            if config.image_download_interval_sec > 0 {
                sleep(Duration::from_secs(config.image_download_interval_sec)).await;
            }

            if next_to_spawn < images.len() {
                let image = images[next_to_spawn].clone();
                spawn_image_task(
                    &mut join_set,
                    host.clone(),
                    jm.clone(),
                    image,
                    next_to_spawn,
                );
                next_to_spawn += 1;
            }
        }

        fs::create_dir_all(&comic_dir).map_err(|err| err.to_string())?;
        fs::write(
            comic_dir.join("metadata.json"),
            serde_json::to_string_pretty(comic).map_err(|err| err.to_string())?,
        )
        .map_err(|err| err.to_string())?;

        if let Some(strict) = NiuhuanCompat::from_comic(comic) {
            fs::write(
                comic_dir.join("元数据.json"),
                serde_json::to_string_pretty(&strict).map_err(|err| err.to_string())?,
            )
            .map_err(|err| err.to_string())?;
        }

        if config.chapter_download_interval_sec > 0 {
            sleep(Duration::from_secs(config.chapter_download_interval_sec)).await;
        }

        let result = self.local_chapter_from_disk(comic, &comic_dir, chapter);
        self.task_controls.lock().unwrap().remove(&chapter.id);
        result
    }

    pub fn pause_download(&self, chapter_id: &str) {
        if let Some(control) = self.task_controls.lock().unwrap().get(chapter_id).cloned() {
            control.pause();
        }
    }

    pub fn resume_download(&self, chapter_id: &str) {
        if let Some(control) = self.task_controls.lock().unwrap().get(chapter_id).cloned() {
            control.resume();
        }
    }

    pub fn cancel_download(&self, chapter_id: &str) {
        if let Some(control) = self.task_controls.lock().unwrap().get(chapter_id).cloned() {
            control.cancel();
        }
    }

    pub fn read_library(&self) -> Result<Vec<LocalComicEntry>, String> {
        let config = self.config();
        let mut entries = self.read_zone_library(&config.download_dir, None)?;
        for (subdir, platform_tag) in known_platform_subdirs() {
            let platform_root = config.download_dir.join(subdir);
            entries.extend(self.read_zone_library(&platform_root, Some(platform_tag.to_string()))?);
        }
        entries.sort_by(|left, right| left.comic.title.cmp(&right.comic.title));
        entries.dedup_by(|left, right| left.comic.id == right.comic.id);
        Ok(entries)
    }

    pub fn delete_local_comic(&self, comic_dir: &Path) -> Result<(), String> {
        if comic_dir.exists() {
            fs::remove_dir_all(comic_dir).map_err(|err| err.to_string())?;
        }
        Ok(())
    }

    pub fn export_local_chapter_cbz(&self, chapter: &LocalChapterEntry) -> Result<PathBuf, String> {
        let config = self.config();
        fs::create_dir_all(&config.export_dir).map_err(|err| err.to_string())?;
        let export_name = format!(
            "{}-{}.cbz",
            sanitize_filename(&chapter.comic_title),
            sanitize_filename(&chapter.chapter_title)
        );
        let output_path = config.export_dir.join(export_name);
        let runner = ExportRunner::new();
        let callback: Box<dyn Fn(hmanga_core::DownloadEvent) + Send + Sync> = Box::new(|_| {});
        runner.run_cbz(0, &chapter.chapter_dir, &output_path, &callback)?;
        Ok(output_path)
    }

    pub async fn update_library_queue(
        &self,
    ) -> Result<Vec<(Comic, hmanga_core::ChapterInfo)>, String> {
        let config = self.config();
        let library = self.read_library()?;
        let mut queue = Vec::new();

        for (index, item) in library.iter().enumerate() {
            if item.comic.source != "jm" {
                continue;
            }

            let latest = self.load_jm_comic(&item.comic.id).await?;
            for chapter in latest.chapters.clone() {
                let exists_locally = item
                    .chapters
                    .iter()
                    .any(|local| local.chapter_id == chapter.id);
                if !exists_locally {
                    queue.push((latest.clone(), chapter));
                }
            }

            if config.update_downloaded_comics_interval_sec > 0 && index + 1 < library.len() {
                sleep(Duration::from_secs(
                    config.update_downloaded_comics_interval_sec,
                ))
                .await;
            }
        }

        Ok(queue)
    }

    fn local_chapter_from_disk(
        &self,
        comic: &Comic,
        comic_dir: &Path,
        chapter: &hmanga_core::ChapterInfo,
    ) -> Result<LocalChapterEntry, String> {
        let chapter_dir = comic_dir.join(sanitize_filename(&chapter.title));
        let mut pages = if chapter_dir.exists() {
            fs::read_dir(&chapter_dir)
                .map_err(|err| err.to_string())?
                .filter_map(Result::ok)
                .map(|entry| entry.path())
                .filter(|path| {
                    path.extension()
                        .and_then(|ext| ext.to_str())
                        .map(|ext| matches!(ext, "png" | "gif" | "jpg" | "jpeg" | "webp"))
                        .unwrap_or(false)
                })
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        pages.sort();

        Ok(LocalChapterEntry {
            comic_id: comic.id.clone(),
            comic_title: comic.title.clone(),
            chapter_id: chapter.id.clone(),
            chapter_title: chapter.title.clone(),
            chapter_dir,
            pages,
        })
    }

    fn read_zone_library(
        &self,
        source_root: &Path,
        platform_tag: Option<String>,
    ) -> Result<Vec<LocalComicEntry>, String> {
        if !source_root.exists() {
            return Ok(Vec::new());
        }

        let mut entries = Vec::new();
        for comic_entry in fs::read_dir(source_root).map_err(|err| err.to_string())? {
            let comic_dir = comic_entry.map_err(|err| err.to_string())?.path();
            if !comic_dir.is_dir() {
                continue;
            }
            let (comic, metadata_path, needs_write) = match read_comic_metadata(&comic_dir) {
                Ok((c, p, w)) => (c, p, w),
                Err(_) => continue,
            };

            // If we read from 元数据.json, generate metadata.json for faster future reads
            if needs_write {
                let new_metadata_path = comic_dir.join("metadata.json");
                if !new_metadata_path.exists() {
                    let _ = fs::write(
                        &new_metadata_path,
                        serde_json::to_string_pretty(&comic).unwrap_or_default(),
                    );
                }
            }

            let mut chapters = Vec::new();
            for chapter in &comic.chapters {
                if let Ok(local_chapter) = self.local_chapter_from_disk(&comic, &comic_dir, chapter)
                {
                    if !local_chapter.pages.is_empty() {
                        chapters.push(local_chapter);
                    }
                }
            }

            let download_time = metadata_path
                .metadata()
                .ok()
                .and_then(|m| m.modified().ok())
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs());
            let update_time = chapters
                .iter()
                .filter_map(|c| {
                    c.chapter_dir
                        .metadata()
                        .ok()
                        .and_then(|m| m.modified().ok())
                        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                        .map(|d| d.as_secs())
                })
                .max();

            entries.push(LocalComicEntry {
                comic,
                comic_dir,
                chapters,
                platform_tag: platform_tag.clone(),
                download_time,
                update_time,
            });
        }

        let mut metadata_files = Vec::new();
        collect_named_files(source_root, "元数据.json", &mut metadata_files)?;

        for metadata_path in metadata_files {
            let Some(comic_dir) = metadata_path.parent() else {
                continue;
            };
            let download_root = self.config().download_dir;
            if known_platform_subdirs().iter().any(|(subdir, _)| {
                source_root == download_root && comic_dir.starts_with(download_root.join(subdir))
            }) {
                continue;
            }

            let legacy = serde_json::from_str::<LegacyComicMetadata>(
                &fs::read_to_string(&metadata_path).map_err(|err| err.to_string())?,
            )
            .map_err(|err| err.to_string())?;
            let comic = legacy.to_comic();
            let chapters = self.legacy_chapters_from_disk(&legacy, comic_dir)?;

            let download_time = metadata_path
                .metadata()
                .ok()
                .and_then(|m| m.modified().ok())
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs());
            let update_time = chapters
                .iter()
                .filter_map(|c| {
                    c.chapter_dir
                        .metadata()
                        .ok()
                        .and_then(|m| m.modified().ok())
                        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                        .map(|d| d.as_secs())
                })
                .max();

            entries.push(LocalComicEntry {
                comic,
                comic_dir: comic_dir.to_path_buf(),
                chapters,
                platform_tag: platform_tag.clone(),
                download_time,
                update_time,
            });
        }

        Ok(entries)
    }

    fn legacy_chapters_from_disk(
        &self,
        legacy: &LegacyComicMetadata,
        comic_dir: &Path,
    ) -> Result<Vec<LocalChapterEntry>, String> {
        let chapter_metadata = collect_legacy_chapter_metadata(comic_dir)?;
        let mut entries = Vec::new();

        for chapter in &legacy.chapter_infos {
            let chapter_dir = chapter_metadata
                .get(&chapter.chapter_id)
                .cloned()
                .unwrap_or_else(|| comic_dir.join(&chapter.chapter_title));

            let mut pages = if chapter_dir.exists() {
                fs::read_dir(&chapter_dir)
                    .map_err(|err| err.to_string())?
                    .filter_map(Result::ok)
                    .map(|entry| entry.path())
                    .filter(|path| {
                        path.extension()
                            .and_then(|ext| ext.to_str())
                            .map(|ext| matches!(ext, "png" | "gif" | "jpg" | "jpeg" | "webp"))
                            .unwrap_or(false)
                    })
                    .collect::<Vec<_>>()
            } else {
                Vec::new()
            };
            pages.sort();

            if !pages.is_empty() {
                entries.push(LocalChapterEntry {
                    comic_id: legacy.id.to_string(),
                    comic_title: legacy.name.clone(),
                    chapter_id: chapter.chapter_id.to_string(),
                    chapter_title: chapter.chapter_title.clone(),
                    chapter_dir,
                    pages,
                });
            }
        }

        Ok(entries)
    }

    fn download_control(&self, chapter_id: &str) -> Arc<DownloadControl> {
        let mut controls = self.task_controls.lock().unwrap();
        controls
            .entry(chapter_id.to_string())
            .or_insert_with(|| Arc::new(DownloadControl::default()))
            .clone()
    }
}

#[derive(Default)]
struct DownloadControl {
    state: AtomicU8,
}

impl DownloadControl {
    fn pause(&self) {
        self.state.store(1, Ordering::SeqCst);
    }

    fn resume(&self) {
        self.state.store(0, Ordering::SeqCst);
    }

    fn cancel(&self) {
        self.state.store(2, Ordering::SeqCst);
    }

    async fn wait_until_active(&self) -> Result<(), String> {
        loop {
            match self.state.load(Ordering::SeqCst) {
                0 => return Ok(()),
                1 => sleep(Duration::from_millis(150)).await,
                2 => return Err("下载已取消".to_string()),
                _ => return Ok(()),
            }
        }
    }
}

pub fn to_browser_src(path: &Path) -> String {
    let mime = match path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_ascii_lowercase())
        .as_deref()
    {
        Some("png") => "image/png",
        Some("gif") => "image/gif",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("webp") => "image/webp",
        _ => "application/octet-stream",
    };

    match fs::read(path) {
        Ok(bytes) => bytes_to_data_url(&bytes, mime),
        Err(_) => String::new(),
    }
}

fn bytes_to_data_url(bytes: &[u8], mime: &str) -> String {
    let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
    format!("data:{mime};base64,{encoded}")
}

fn mime_from_extension(extension: &str) -> &'static str {
    match extension.to_ascii_lowercase().as_str() {
        "png" => "image/png",
        "gif" => "image/gif",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        _ => "application/octet-stream",
    }
}

fn response_mime(response: &HttpResponse, url: &str) -> String {
    response
        .header("content-type")
        .and_then(|value| value.split(';').next())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| {
            let extension = url
                .split('?')
                .next()
                .and_then(|value| value.rsplit('.').next())
                .unwrap_or_default();
            mime_from_extension(extension).to_string()
        })
}

fn remote_image_to_data_url(response: &HttpResponse, url: &str) -> String {
    bytes_to_data_url(&response.body, &response_mime(response, url))
}

fn sanitize_filename(value: &str) -> String {
    value
        .chars()
        .map(|ch| match ch {
            '\\' | '/' | '\n' => ' ',
            ':' => '：',
            '*' => '⭐',
            '?' => '？',
            '"' => '\'',
            '<' => '《',
            '>' => '》',
            '|' => '丨',
            _ => ch,
        })
        .collect::<String>()
        .trim()
        .trim_end_matches('.')
        .trim()
        .to_string()
}

fn build_jm_download_dirs(
    root: &Path,
    comic: &Comic,
    chapter: &hmanga_core::ChapterInfo,
) -> (PathBuf, PathBuf) {
    let comic_dir = root.join(sanitize_filename(&comic.title));
    let chapter_dir = comic_dir.join(sanitize_filename(&chapter.title));
    (comic_dir, chapter_dir)
}

fn build_host_runtime(config: &AppConfig) -> Result<HostRuntime, String> {
    HostRuntime::new_with_proxy(config.proxy.as_deref())
}

fn build_jm_plugin(config: &AppConfig) -> JmPlugin {
    let api_domain = resolve_jm_api_domain(config);
    JmPlugin::default()
        .with_api_domain(api_domain)
        .with_download_format(resolve_site_download_format(config, &config.sites.jm))
}

fn build_wnacg_plugin(config: &AppConfig) -> WnacgPlugin {
    WnacgPlugin::default()
        .with_api_domain(resolve_site_api_domain(&config.sites.wnacg))
        .with_download_format(resolve_site_download_format(config, &config.sites.wnacg))
}

fn build_plugin_registry(config: &AppConfig) -> HashMap<String, Arc<dyn DynPlugin>> {
    let mut plugins: HashMap<String, Arc<dyn DynPlugin>> = HashMap::new();
    plugins.insert(
        "jm".to_string(),
        Arc::new(build_jm_plugin(config)) as Arc<dyn DynPlugin>,
    );
    plugins.insert(
        "wnacg".to_string(),
        Arc::new(build_wnacg_plugin(config)) as Arc<dyn DynPlugin>,
    );
    plugins
}

async fn search_aggregate_plugins(
    plugins: &HashMap<String, Arc<dyn DynPlugin>>,
    host: &dyn HostApi,
    enabled_plugins: &[String],
    query: &str,
    page: u32,
) -> Result<SearchPage, String> {
    let mut all_comics = Vec::new();
    let mut total_pages = 1u32;
    for plugin_id in enabled_plugins {
        let Some(plugin) = plugins.get(plugin_id) else {
            continue;
        };
        let result = plugin
            .search(host, query, page, hmanga_core::SearchSort::Latest)
            .await
            .map_err(|err| err.to_string())?;
        total_pages = total_pages.max(result.total_pages);
        all_comics.extend(result.comics);
    }
    Ok(SearchPage {
        comics: all_comics,
        current_page: page,
        total_pages,
    })
}

fn resolve_jm_api_domain(config: &AppConfig) -> String {
    let site_domain = resolve_site_api_domain(&config.sites.jm);
    if site_domain == "www.cdnhth.cc"
        && (config.api_domain != "www.cdnhth.cc" || !config.custom_api_domain.trim().is_empty())
    {
        if config.custom_api_domain.trim().is_empty() {
            config.api_domain.clone()
        } else {
            config.custom_api_domain.clone()
        }
    } else {
        site_domain
    }
}

fn resolve_site_api_domain(site: &SiteConfig) -> String {
    site.api_domain.trim().to_string()
}

fn resolve_site_download_format<'a>(config: &'a AppConfig, site: &'a SiteConfig) -> &'a str {
    if site.use_global_download_format {
        &config.download_format
    } else {
        &site.download_format
    }
}

#[cfg_attr(not(test), allow(dead_code))]
fn resolve_site_cover_preference(config: &AppConfig, site: &SiteConfig) -> bool {
    if site.use_global_cover_preference {
        config.should_download_cover
    } else {
        site.should_download_cover
    }
}

fn spawn_image_task(
    join_set: &mut JoinSet<Result<(usize, String, ProcessedImage), String>>,
    host: HostRuntime,
    jm: JmPlugin,
    image: hmanga_core::ImageUrl,
    index: usize,
) {
    join_set.spawn(async move {
        let current_name = image
            .headers
            .get("x-hmanga-jm-file-name")
            .cloned()
            .unwrap_or_else(|| format!("{:04}", index + 1));
        let response = host
            .http_request(HttpRequest {
                url: image.url.clone(),
                method: HttpMethod::Get,
                headers: HashMap::new(),
                body: None,
            })
            .await
            .map_err(|err| err.to_string())?;
        if response.status != 200 {
            return Err(format!("下载图片失败: {}", response.status));
        }

        let processed = jm
            .process_image(&image, response.body)
            .map_err(|err| err.to_string())?;
        Ok((index, current_name, processed))
    });
}

fn spawn_jm_reader_task(
    join_set: &mut JoinSet<Result<(usize, String, String), String>>,
    host: HostRuntime,
    jm: JmPlugin,
    image: hmanga_core::ImageUrl,
    index: usize,
) {
    join_set.spawn(async move {
        let current_name = image
            .headers
            .get("x-hmanga-jm-file-name")
            .cloned()
            .unwrap_or_else(|| format!("{:04}", index + 1));
        let response = host
            .http_request(HttpRequest {
                url: image.url.clone(),
                method: HttpMethod::Get,
                headers: HashMap::new(),
                body: None,
            })
            .await
            .map_err(|err| err.to_string())?;
        if response.status != 200 {
            return Err(format!("载入图片失败: {}", response.status));
        }

        let processed = jm
            .process_image(&image, response.body)
            .map_err(|err| err.to_string())?;
        let page_src =
            bytes_to_data_url(&processed.bytes, mime_from_extension(processed.extension));
        Ok((index, current_name, page_src))
    });
}

fn spawn_passthrough_reader_task(
    join_set: &mut JoinSet<Result<(usize, String, String), String>>,
    host: HostRuntime,
    image: hmanga_core::ImageUrl,
    index: usize,
) {
    join_set.spawn(async move {
        let current_name = image
            .url
            .rsplit('/')
            .next()
            .map(str::to_string)
            .unwrap_or_else(|| format!("{:04}", index + 1));
        let response = host
            .http_request(HttpRequest {
                url: image.url.clone(),
                method: HttpMethod::Get,
                headers: image.headers.clone(),
                body: None,
            })
            .await
            .map_err(|err| err.to_string())?;
        if response.status != 200 {
            return Err(format!("载入图片失败: {}", response.status));
        }

        let page_src = remote_image_to_data_url(&response, &image.url);
        Ok((index, current_name, page_src))
    });
}

fn resolve_config_dir() -> PathBuf {
    if let Ok(path) = std::env::var("HMANGA_CONFIG_DIR") {
        return PathBuf::from(path);
    }

    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(".config")
        .join("hmanga")
}

fn load_or_init_config(
    config_path: &Path,
    default_download_dir: PathBuf,
) -> Result<AppConfig, String> {
    if config_path.exists() {
        let mut config = serde_json::from_str::<AppConfig>(
            &fs::read_to_string(config_path).map_err(|err| err.to_string())?,
        )
        .map_err(|err| err.to_string())?;
        migrate_legacy_site_settings(&mut config);
        if config.export_dir.as_os_str().is_empty() || config.export_dir == Path::new("Exports") {
            config.export_dir = config.download_dir.join("_exports");
        }
        return Ok(config);
    }

    let export_dir = default_download_dir.join("_exports");
    let config = AppConfig {
        download_dir: default_download_dir,
        export_dir,
        ..AppConfig::default()
    };
    persist_config(config_path, &config)?;
    Ok(config)
}

fn persist_config(config_path: &Path, config: &AppConfig) -> Result<(), String> {
    fs::write(
        config_path,
        serde_json::to_string_pretty(config).map_err(|err| err.to_string())?,
    )
    .map_err(|err| err.to_string())
}

fn migrate_legacy_site_settings(config: &mut AppConfig) {
    let legacy_jm_api_domain = if config.custom_api_domain.trim().is_empty() {
        config.api_domain.trim()
    } else {
        config.custom_api_domain.trim()
    };

    if config.sites.jm.api_domain == "www.cdnhth.cc"
        && !legacy_jm_api_domain.is_empty()
        && legacy_jm_api_domain != "www.cdnhth.cc"
    {
        config.sites.jm.api_domain = legacy_jm_api_domain.to_string();
    }
}

fn collect_named_files(
    root: &Path,
    target_name: &str,
    output: &mut Vec<PathBuf>,
) -> Result<(), String> {
    if !root.exists() {
        return Ok(());
    }

    for entry in fs::read_dir(root).map_err(|err| err.to_string())? {
        let path = entry.map_err(|err| err.to_string())?.path();
        if path.is_dir() {
            collect_named_files(&path, target_name, output)?;
        } else if path.file_name().and_then(|name| name.to_str()) == Some(target_name) {
            output.push(path);
        }
    }

    Ok(())
}

/// Reads comic metadata, trying files in order of preference:
/// 1. metadata.json (full Hmanga format)
/// 2. 元数据.json - tried as LegacyComicMetadata first (camelCase, old Hmanga format),
///    then as NiuhuanCompat (Yeats33/jmcomic-downloader compatible, snake_case)
///
/// Returns (comic, metadata_path, needs_write_metadata_json)
/// where `needs_write_metadata_json` is true if only 元数据.json existed and metadata.json should be generated
fn read_comic_metadata(comic_dir: &Path) -> Result<(Comic, PathBuf, bool), String> {
    // Try metadata.json first (full Hmanga format)
    let metadata_path = comic_dir.join("metadata.json");
    if metadata_path.exists() {
        let comic: Comic = serde_json::from_str(
            &fs::read_to_string(&metadata_path).map_err(|err| err.to_string())?,
        )
        .map_err(|err| err.to_string())?;
        return Ok((comic, metadata_path, false));
    }

    // Fall back to 元数据.json
    let booker_path = comic_dir.join("元数据.json");
    if booker_path.exists() {
        let content = fs::read_to_string(&booker_path).map_err(|err| err.to_string())?;

        // First try LegacyComicMetadata (old Hmanga format with camelCase chapterInfos)
        if let Ok(legacy) = serde_json::from_str::<LegacyComicMetadata>(&content) {
            return Ok((legacy.to_comic(), booker_path, true));
        }

        // Then try NiuhuanCompat (Yeats33/jmcomic-downloader compatible format)
        if let Ok(strict) = serde_json::from_str::<NiuhuanCompat>(&content) {
            return Ok((strict.to_comic(), booker_path, true));
        }
    }

    Err("no metadata file found".to_string())
}

fn collect_legacy_chapter_metadata(comic_dir: &Path) -> Result<HashMap<i64, PathBuf>, String> {
    let mut metadata_files = Vec::new();
    collect_named_files(comic_dir, "章节元数据.json", &mut metadata_files)?;
    let mut output = HashMap::new();

    for metadata_path in metadata_files {
        let metadata = serde_json::from_str::<LegacyChapterMetadata>(
            &fs::read_to_string(&metadata_path).map_err(|err| err.to_string())?,
        )
        .map_err(|err| err.to_string())?;
        if let Some(parent) = metadata_path.parent() {
            output.insert(metadata.chapter_id, parent.to_path_buf());
        }
    }

    Ok(output)
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct LegacyComicMetadata {
    id: i64,
    name: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    author: Vec<String>,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    chapter_infos: Vec<LegacyChapterMetadata>,
}

impl LegacyComicMetadata {
    fn to_comic(&self) -> Comic {
        Comic {
            id: self.id.to_string(),
            source: "jm".to_string(),
            title: self.name.clone(),
            author: self.author.join(", "),
            cover_url: String::new(),
            description: self.description.clone(),
            tags: self.tags.clone(),
            chapters: self
                .chapter_infos
                .iter()
                .map(|chapter| hmanga_core::ChapterInfo {
                    id: chapter.chapter_id.to_string(),
                    title: chapter.chapter_title.clone(),
                    page_count: None,
                })
                .collect(),
            extra: HashMap::new(),
            ..Default::default()
        }
    }
}

#[derive(Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct LegacyChapterMetadata {
    chapter_id: i64,
    chapter_title: String,
}

fn known_platform_subdirs() -> [(&'static str, &'static str); 3] {
    [("jm", "JM"), ("wnacg", "WNACG"), ("copymanga", "拷贝漫画")]
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use hmanga_core::{
        Capabilities, FavoriteResult, HttpRequest, HttpResponse, ImageUrl, LogLevel, PluginError,
        PluginMetaInfo, SearchResult, SearchSort, Session, WeeklyResult,
    };
    use std::pin::Pin;
    use tempfile::TempDir;

    #[derive(Default)]
    struct NoopHost;

    impl HostApi for NoopHost {
        fn http_request(
            &self,
            _request: HttpRequest,
        ) -> Pin<
            Box<dyn std::future::Future<Output = hmanga_core::PluginResult<HttpResponse>> + Send>,
        > {
            Box::pin(async { Err(PluginError::Other("unused".to_string())) })
        }

        fn log(&self, _level: LogLevel, _message: &str) {}
    }

    #[derive(Clone)]
    struct FakePlugin {
        id: String,
        comics: Vec<Comic>,
    }

    impl FakePlugin {
        fn new(id: &str, titles: &[&str]) -> Self {
            Self {
                id: id.to_string(),
                comics: titles
                    .iter()
                    .enumerate()
                    .map(|(index, title)| Comic {
                        id: format!("{id}-{index}"),
                        source: id.to_string(),
                        title: (*title).to_string(),
                        author: String::new(),
                        cover_url: String::new(),
                        description: String::new(),
                        tags: Vec::new(),
                        chapters: Vec::new(),
                        extra: HashMap::new(),
                        ..Default::default()
                    })
                    .collect(),
            }
        }
    }

    #[async_trait]
    impl DynPlugin for FakePlugin {
        fn id(&self) -> &str {
            &self.id
        }

        fn meta(&self) -> PluginMetaInfo {
            PluginMetaInfo {
                id: self.id.clone(),
                name: self.id.clone(),
                version: "test".to_string(),
                sdk_version: 1,
                icon: Vec::new(),
                description: "test".to_string(),
                capabilities: Capabilities {
                    search: true,
                    login: false,
                    favorites: false,
                    ranking: false,
                    weekly: false,
                    tags_browsing: false,
                },
            }
        }

        async fn search(
            &self,
            _host: &dyn HostApi,
            _query: &str,
            _page: u32,
            _sort: SearchSort,
        ) -> hmanga_core::PluginResult<SearchResult> {
            Ok(SearchResult {
                comics: self.comics.clone(),
                current_page: 1,
                total_pages: 1,
            })
        }

        async fn get_comic(
            &self,
            _host: &dyn HostApi,
            _comic_id: &str,
        ) -> hmanga_core::PluginResult<Comic> {
            Err(PluginError::NotSupported)
        }

        async fn get_chapter_images(
            &self,
            _host: &dyn HostApi,
            _chapter_id: &str,
        ) -> hmanga_core::PluginResult<Vec<ImageUrl>> {
            Err(PluginError::NotSupported)
        }

        async fn login(
            &self,
            _host: &dyn HostApi,
            _username: &str,
            _password: &str,
        ) -> hmanga_core::PluginResult<Session> {
            Err(PluginError::NotSupported)
        }

        async fn get_favorites(
            &self,
            _host: &dyn HostApi,
            _session: Option<&Session>,
            _page: u32,
        ) -> hmanga_core::PluginResult<FavoriteResult> {
            Err(PluginError::NotSupported)
        }

        async fn get_weekly(&self, _host: &dyn HostApi) -> hmanga_core::PluginResult<WeeklyResult> {
            Err(PluginError::NotSupported)
        }
    }

    #[test]
    fn initializes_and_persists_config_file_with_download_dir() {
        let config_dir = TempDir::new().unwrap();
        let download_dir = TempDir::new().unwrap();

        let services = AppServices::new_with_paths(
            config_dir.path().to_path_buf(),
            download_dir.path().to_path_buf(),
        )
        .unwrap();

        assert_eq!(services.config().download_dir, download_dir.path());
        assert_eq!(
            services.config().export_dir,
            download_dir.path().join("_exports")
        );
        assert!(config_dir.path().join("config.json").exists());
    }

    #[test]
    fn reads_legacy_jmcomic_downloader_library_layout() {
        let config_dir = TempDir::new().unwrap();
        let download_dir = TempDir::new().unwrap();

        let comic_dir = download_dir.path().join("旧漫画");
        let chapter_dir = comic_dir.join("第1话 开始");
        fs::create_dir_all(&chapter_dir).unwrap();

        fs::write(
            comic_dir.join("元数据.json"),
            r#"{
              "id": 123,
              "name": "旧漫画",
              "description": "旧版元数据",
              "author": ["老作者"],
              "tags": ["怀旧"],
              "chapterInfos": [
                { "chapterId": 456, "chapterTitle": "第1话 开始", "order": 1 }
              ]
            }"#,
        )
        .unwrap();
        fs::write(
            chapter_dir.join("章节元数据.json"),
            r#"{
              "chapterId": 456,
              "chapterTitle": "第1话 开始",
              "order": 1
            }"#,
        )
        .unwrap();
        fs::write(chapter_dir.join("0001.png"), b"fake").unwrap();

        let services = AppServices::new_with_paths(
            config_dir.path().to_path_buf(),
            download_dir.path().to_path_buf(),
        )
        .unwrap();
        let library = services.read_library().unwrap();

        assert_eq!(library.len(), 1);
        assert_eq!(library[0].comic.id, "123");
        assert_eq!(library[0].comic.title, "旧漫画");
        assert_eq!(library[0].chapters.len(), 1);
        assert_eq!(library[0].platform_tag, None);
        assert_eq!(library[0].chapters[0].chapter_id, "456");
        assert_eq!(library[0].chapters[0].pages.len(), 1);
    }

    #[test]
    fn browser_src_uses_data_url_for_local_images() {
        let dir = TempDir::new().unwrap();
        let image_path = dir.path().join("cover.png");
        fs::write(&image_path, b"png-bytes").unwrap();

        let src = to_browser_src(&image_path);

        assert!(src.starts_with("data:image/png;base64,"));
    }

    #[test]
    fn response_mime_prefers_http_header() {
        let response = HttpResponse {
            status: 200,
            headers: HashMap::from([("content-type".to_string(), vec!["image/webp".to_string()])]),
            body: b"img".to_vec(),
        };

        assert_eq!(
            response_mime(&response, "https://example.com/image.jpg"),
            "image/webp"
        );
    }

    #[test]
    fn remote_image_to_data_url_falls_back_to_url_extension() {
        let response = HttpResponse {
            status: 200,
            headers: HashMap::new(),
            body: b"png-bytes".to_vec(),
        };

        let src = remote_image_to_data_url(&response, "https://example.com/cover.png");

        assert!(src.starts_with("data:image/png;base64,"));
    }

    #[test]
    fn persists_jm_credentials_into_config_file() {
        let config_dir = TempDir::new().unwrap();
        let download_dir = TempDir::new().unwrap();
        let services = AppServices::new_with_paths(
            config_dir.path().to_path_buf(),
            download_dir.path().to_path_buf(),
        )
        .unwrap();

        services.save_jm_credentials("demo", "secret").unwrap();

        let config = serde_json::from_str::<AppConfig>(
            &fs::read_to_string(config_dir.path().join("config.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(config.jm_username, "demo");
        assert_eq!(config.jm_password, "secret");
    }

    #[test]
    fn save_config_persists_multithread_fields() {
        let config_dir = TempDir::new().unwrap();
        let download_dir = TempDir::new().unwrap();
        let services = AppServices::new_with_paths(
            config_dir.path().to_path_buf(),
            download_dir.path().to_path_buf(),
        )
        .unwrap();

        let mut config = services.config().clone();
        config.chapter_concurrency = 6;
        config.chapter_download_interval_sec = 3;
        config.image_concurrency = 24;
        config.image_download_interval_sec = 1;
        config.download_all_favorites_interval_sec = 5;
        config.update_downloaded_comics_interval_sec = 7;
        config.download_dir = download_dir.path().join("新目录");
        config.export_dir = download_dir.path().join("导出目录");
        services.save_config(&config).unwrap();

        let persisted = serde_json::from_str::<AppConfig>(
            &fs::read_to_string(config_dir.path().join("config.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(persisted.chapter_concurrency, 6);
        assert_eq!(persisted.chapter_download_interval_sec, 3);
        assert_eq!(persisted.image_concurrency, 24);
        assert_eq!(persisted.image_download_interval_sec, 1);
        assert_eq!(persisted.download_all_favorites_interval_sec, 5);
        assert_eq!(persisted.update_downloaded_comics_interval_sec, 7);
        assert_eq!(persisted.download_dir, download_dir.path().join("新目录"));
        assert_eq!(persisted.export_dir, download_dir.path().join("导出目录"));
    }

    #[test]
    fn exports_local_chapter_as_cbz() {
        let config_dir = TempDir::new().unwrap();
        let download_dir = TempDir::new().unwrap();
        let chapter_dir = download_dir.path().join("章节A");
        fs::create_dir_all(&chapter_dir).unwrap();
        fs::write(chapter_dir.join("0001.png"), b"png-a").unwrap();
        fs::write(chapter_dir.join("0002.png"), b"png-b").unwrap();

        let services = AppServices::new_with_paths(
            config_dir.path().to_path_buf(),
            download_dir.path().to_path_buf(),
        )
        .unwrap();

        let chapter = LocalChapterEntry {
            comic_id: "1".to_string(),
            comic_title: "漫画A".to_string(),
            chapter_id: "2".to_string(),
            chapter_title: "章节A".to_string(),
            chapter_dir: chapter_dir.clone(),
            pages: vec![chapter_dir.join("0001.png"), chapter_dir.join("0002.png")],
        };

        let exported = services.export_local_chapter_cbz(&chapter).unwrap();
        assert!(exported.exists());
        assert_eq!(
            exported.extension().and_then(|ext| ext.to_str()),
            Some("cbz")
        );
    }

    #[tokio::test]
    async fn download_control_reacts_to_pause_resume_and_cancel() {
        let control = Arc::new(DownloadControl::default());
        control.pause();

        let resumed_control = control.clone();
        let waiter = tokio::spawn(async move { resumed_control.wait_until_active().await });
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        control.resume();
        assert!(waiter.await.unwrap().is_ok());

        let delayed = Arc::new(DownloadControl::default());
        delayed.pause();
        let delayed_handle = delayed.clone();
        let resume_task = tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            delayed_handle.resume();
        });
        let delayed_wait = delayed.clone();
        let waiter = tokio::spawn(async move { delayed_wait.wait_until_active().await });
        resume_task.await.unwrap();
        assert!(waiter.await.unwrap().is_ok());

        let cancelled = DownloadControl::default();
        cancelled.cancel();
        assert_eq!(
            cancelled.wait_until_active().await.unwrap_err(),
            "下载已取消"
        );
    }

    #[test]
    fn discovers_platform_subdirectories_and_tags_them() {
        let config_dir = TempDir::new().unwrap();
        let download_dir = TempDir::new().unwrap();

        let root_comic_dir = download_dir.path().join("根目录漫画");
        fs::create_dir_all(root_comic_dir.join("第1话")).unwrap();
        fs::write(
            root_comic_dir.join("metadata.json"),
            r#"{
              "id": "root-1",
              "source": "jm",
              "title": "根目录漫画",
              "author": "作者A",
              "cover_url": "",
              "description": "",
              "tags": [],
              "chapters": [{"id": "c1", "title": "第1话", "page_count": null}],
              "extra": {}
            }"#,
        )
        .unwrap();
        fs::write(root_comic_dir.join("第1话").join("0001.png"), b"img").unwrap();

        let tagged_comic_dir = download_dir.path().join("jm").join("子目录漫画");
        fs::create_dir_all(tagged_comic_dir.join("第2话")).unwrap();
        fs::write(
            tagged_comic_dir.join("metadata.json"),
            r#"{
              "id": "tagged-1",
              "source": "jm",
              "title": "子目录漫画",
              "author": "作者B",
              "cover_url": "",
              "description": "",
              "tags": [],
              "chapters": [{"id": "c2", "title": "第2话", "page_count": null}],
              "extra": {}
            }"#,
        )
        .unwrap();
        fs::write(tagged_comic_dir.join("第2话").join("0001.png"), b"img").unwrap();

        let services = AppServices::new_with_paths(
            config_dir.path().to_path_buf(),
            download_dir.path().to_path_buf(),
        )
        .unwrap();
        let library = services.read_library().unwrap();

        let root_entry = library
            .iter()
            .find(|entry| entry.comic.id == "root-1")
            .unwrap();
        let tagged_entry = library
            .iter()
            .find(|entry| entry.comic.id == "tagged-1")
            .unwrap();

        assert_eq!(root_entry.platform_tag, None);
        assert_eq!(tagged_entry.platform_tag.as_deref(), Some("JM"));
    }

    #[test]
    fn jm_download_paths_follow_reference_dir_fmt() {
        let comic = Comic {
            id: "123".to_string(),
            source: "jm".to_string(),
            title: "漫画:A/测试".to_string(),
            author: "作者".to_string(),
            cover_url: String::new(),
            description: String::new(),
            tags: Vec::new(),
            chapters: vec![hmanga_core::ChapterInfo {
                id: "456".to_string(),
                title: "第1话 特别篇".to_string(),
                page_count: None,
            }],
            extra: HashMap::new(),
            ..Default::default()
        };
        let chapter = comic.chapters[0].clone();
        let root = PathBuf::from("/tmp/books");

        let (comic_dir, chapter_dir) = build_jm_download_dirs(&root, &comic, &chapter);

        assert_eq!(comic_dir, root.join("漫画：A 测试"));
        assert_eq!(chapter_dir, root.join("漫画：A 测试").join("第1话 特别篇"));
    }

    #[test]
    fn loading_existing_config_migrates_legacy_jm_domain_into_site_settings() {
        let config_dir = TempDir::new().unwrap();
        let download_dir = TempDir::new().unwrap();
        fs::write(
            config_dir.path().join("config.json"),
            format!(
                r#"{{
                  "version": 1,
                  "donation_unlocked": false,
                  "download_dir": "{}",
                  "export_dir": "{}",
                  "chapter_concurrency": 3,
                  "chapter_download_interval_sec": 0,
                  "image_concurrency": 5,
                  "image_download_interval_sec": 0,
                  "download_all_favorites_interval_sec": 0,
                  "update_downloaded_comics_interval_sec": 0,
                  "api_domain": "legacy.jm.example",
                  "custom_api_domain": "",
                  "should_download_cover": true,
                  "download_format": "webp",
                  "proxy": null,
                  "enabled_plugins": ["jm"],
                  "jm_username": "",
                  "jm_password": "",
                  "theme": "Auto"
                }}"#,
                download_dir.path().display(),
                download_dir.path().join("_exports").display()
            ),
        )
        .unwrap();

        let services = AppServices::new_with_paths(
            config_dir.path().to_path_buf(),
            download_dir.path().to_path_buf(),
        )
        .unwrap();

        assert_eq!(services.config().sites.jm.api_domain, "legacy.jm.example");
    }

    #[test]
    fn jm_site_settings_can_override_global_defaults() {
        let mut config = AppConfig {
            download_format: "webp".to_string(),
            should_download_cover: true,
            ..AppConfig::default()
        };
        config.sites.jm.api_domain = "jm.example".to_string();
        config.sites.jm.use_global_download_format = false;
        config.sites.jm.download_format = "png".to_string();
        config.sites.jm.use_global_cover_preference = false;
        config.sites.jm.should_download_cover = false;

        assert_eq!(resolve_site_api_domain(&config.sites.jm), "jm.example");
        assert_eq!(
            resolve_site_download_format(&config, &config.sites.jm),
            "png"
        );
        assert!(!resolve_site_cover_preference(&config, &config.sites.jm));
    }

    #[test]
    fn wnacg_site_settings_can_follow_global_defaults() {
        let mut config = AppConfig {
            download_format: "jpg".to_string(),
            should_download_cover: false,
            ..AppConfig::default()
        };
        config.sites.wnacg.api_domain = "wnacg.example".to_string();
        config.sites.wnacg.use_global_download_format = true;
        config.sites.wnacg.download_format = "png".to_string();
        config.sites.wnacg.use_global_cover_preference = true;
        config.sites.wnacg.should_download_cover = true;

        assert_eq!(
            resolve_site_api_domain(&config.sites.wnacg),
            "wnacg.example"
        );
        assert_eq!(
            resolve_site_download_format(&config, &config.sites.wnacg),
            "jpg"
        );
        assert!(!resolve_site_cover_preference(&config, &config.sites.wnacg));
    }

    #[tokio::test]
    async fn aggregate_search_combines_results_in_enabled_plugin_order() {
        let host = NoopHost;
        let plugins: HashMap<String, Arc<dyn DynPlugin>> = HashMap::from([
            (
                "jm".to_string(),
                Arc::new(FakePlugin::new("jm", &["JM-A"])) as Arc<dyn DynPlugin>,
            ),
            (
                "wnacg".to_string(),
                Arc::new(FakePlugin::new("wnacg", &["WN-1", "WN-2"])) as Arc<dyn DynPlugin>,
            ),
        ]);

        let comics = search_aggregate_plugins(
            &plugins,
            &host,
            &["wnacg".to_string(), "jm".to_string()],
            "demo",
            1,
        )
        .await
        .unwrap();

        assert_eq!(comics.comics.len(), 3);
        assert_eq!(comics.comics[0].source, "wnacg");
        assert_eq!(comics.comics[1].source, "wnacg");
        assert_eq!(comics.comics[2].source, "jm");
    }

    #[tokio::test]
    async fn aggregate_search_skips_plugins_not_enabled_in_config() {
        let host = NoopHost;
        let plugins: HashMap<String, Arc<dyn DynPlugin>> = HashMap::from([
            (
                "jm".to_string(),
                Arc::new(FakePlugin::new("jm", &["JM-A"])) as Arc<dyn DynPlugin>,
            ),
            (
                "wnacg".to_string(),
                Arc::new(FakePlugin::new("wnacg", &["WN-1"])) as Arc<dyn DynPlugin>,
            ),
        ]);

        let comics = search_aggregate_plugins(&plugins, &host, &["jm".to_string()], "demo", 1)
            .await
            .unwrap();

        assert_eq!(comics.comics.len(), 1);
        assert_eq!(comics.comics[0].source, "jm");
    }
}
