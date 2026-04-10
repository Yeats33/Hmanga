use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use base64::Engine;
use hmanga_core::{download::ExportRunner, AppConfig, Comic, HostApi, HttpMethod, HttpRequest};
use hmanga_host::HostRuntime;
use hmanga_plugin_jm::{JmPlugin, JmUserProfile, JmWeeklyInfo, ProcessedImage};
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
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FavoritePage {
    pub comics: Vec<Comic>,
    pub current_page: u32,
    pub total_pages: u32,
}

#[derive(Clone)]
pub struct AppServices {
    host: Arc<Mutex<HostRuntime>>,
    jm: Arc<Mutex<JmPlugin>>,
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
        *self.chapter_gate.lock().unwrap() =
            Arc::new(Semaphore::new(config.chapter_concurrency.max(1)));
        Ok(())
    }

    pub async fn search_aggregate(&self, query: &str) -> Result<Vec<Comic>, String> {
        self.search_jm(query).await
    }

    pub async fn search_jm(&self, query: &str) -> Result<Vec<Comic>, String> {
        let jm = self.jm();
        let host = self.host();
        jm.search(&host, query, 1, hmanga_core::SearchSort::Latest)
            .await
            .map(|result| result.comics)
            .map_err(|err| err.to_string())
    }

    pub async fn load_jm_comic(&self, comic_id: &str) -> Result<Comic, String> {
        let jm = self.jm();
        let host = self.host();
        jm.get_comic(&host, comic_id)
            .await
            .map_err(|err| err.to_string())
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
            let metadata_path = comic_dir.join("metadata.json");
            if !metadata_path.exists() {
                continue;
            }

            let comic = serde_json::from_str::<Comic>(
                &fs::read_to_string(&metadata_path).map_err(|err| err.to_string())?,
            )
            .map_err(|err| err.to_string())?;

            let mut chapters = Vec::new();
            for chapter in &comic.chapters {
                if let Ok(local_chapter) = self.local_chapter_from_disk(&comic, &comic_dir, chapter)
                {
                    if !local_chapter.pages.is_empty() {
                        chapters.push(local_chapter);
                    }
                }
            }

            entries.push(LocalComicEntry {
                comic,
                comic_dir,
                chapters,
                platform_tag: platform_tag.clone(),
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

            entries.push(LocalComicEntry {
                comic,
                comic_dir: comic_dir.to_path_buf(),
                chapters,
                platform_tag: platform_tag.clone(),
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
        Ok(bytes) => {
            let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
            format!("data:{mime};base64,{encoded}")
        }
        Err(_) => String::new(),
    }
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
    let api_domain = if config.custom_api_domain.trim().is_empty() {
        config.api_domain.clone()
    } else {
        config.custom_api_domain.clone()
    };
    JmPlugin::default().with_api_domain(api_domain)
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
    use tempfile::TempDir;

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
        };
        let chapter = comic.chapters[0].clone();
        let root = PathBuf::from("/tmp/books");

        let (comic_dir, chapter_dir) = build_jm_download_dirs(&root, &comic, &chapter);

        assert_eq!(comic_dir, root.join("漫画：A 测试"));
        assert_eq!(chapter_dir, root.join("漫画：A 测试").join("第1话 特别篇"));
    }
}
