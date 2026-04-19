use std::collections::HashMap;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use async_trait::async_trait;

use aes::cipher::generic_array::GenericArray;
use aes::cipher::{BlockDecrypt, KeyInit};
use aes::Aes256;
use base64::Engine;
use hmanga_core::{
    Capabilities, ChapterInfo, Comic, DynPlugin, FavoriteResult, HostApi, HttpMethod, HttpRequest,
    HttpResponse, ImageUrl, PluginError, PluginMetaInfo, PluginResult, SearchResult, SearchSort,
    Session, WeeklyResult,
};
use image::ImageFormat;
use serde::Deserialize;
use serde_json::Value;

const APP_TOKEN_SECRET: &str = "18comicAPP";
const APP_TOKEN_SECRET_2: &str = "18comicAPPContent";
const APP_DATA_SECRET: &str = "185Hcomic3PAPP7R";
const APP_VERSION: &str = "2.0.13";
const DEFAULT_API_DOMAIN: &str = "www.cdnhth.cc";
const DEFAULT_IMAGE_DOMAIN: &str = "cdn-msp2.jmapiproxy2.cc";
const DEFAULT_ALBUM_DOMAIN: &str = "cdn-msp3.18comic.vip";
const DEFAULT_USER_AGENT: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/128.0.0.0 Safari/537.36";
const PAGE_SIZE: u32 = 80;

#[derive(Debug, Clone)]
pub struct JmPlugin {
    api_domain: String,
    image_domain: String,
    album_domain: String,
    fixed_timestamp: Option<u64>,
    download_format: String,
}

#[derive(Debug, Clone)]
pub struct ProcessedImage {
    pub bytes: Vec<u8>,
    pub extension: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JmUserProfile {
    pub username: String,
    pub photo: String,
    pub level_name: String,
    pub favorites_count: i64,
    pub favorites_max: i64,
    pub extra: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JmWeeklyInfo {
    pub categories: Vec<JmWeeklyCategory>,
    pub types: Vec<JmWeeklyType>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JmWeeklyCategory {
    pub id: String,
    pub title: String,
    pub time: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JmWeeklyType {
    pub id: String,
    pub title: String,
}

impl Default for JmPlugin {
    fn default() -> Self {
        Self {
            api_domain: DEFAULT_API_DOMAIN.to_string(),
            image_domain: DEFAULT_IMAGE_DOMAIN.to_string(),
            album_domain: DEFAULT_ALBUM_DOMAIN.to_string(),
            fixed_timestamp: None,
            download_format: "webp".to_string(),
        }
    }
}

impl JmPlugin {
    pub fn with_fixed_timestamp(mut self, timestamp: u64) -> Self {
        self.fixed_timestamp = Some(timestamp);
        self
    }

    pub fn with_api_domain(mut self, api_domain: impl Into<String>) -> Self {
        self.api_domain = api_domain.into();
        self
    }

    pub fn with_download_format(mut self, format: impl Into<String>) -> Self {
        self.download_format = format.into();
        self
    }

    pub fn meta(&self) -> PluginMetaInfo {
        PluginMetaInfo {
            id: plugin_id().to_string(),
            name: "禁漫天堂".to_string(),
            version: "0.1.0".to_string(),
            sdk_version: 1,
            icon: Vec::new(),
            description: "JM / 18comic site adapter".to_string(),
            capabilities: Capabilities {
                search: true,
                login: true,
                favorites: true,
                ranking: false,
                weekly: true,
                tags_browsing: false,
            },
        }
    }

    pub async fn login(
        &self,
        host: &dyn HostApi,
        username: &str,
        password: &str,
    ) -> Result<Session, PluginError> {
        let timestamp = self.timestamp();
        let request = self.build_signed_post(
            "/login",
            &[],
            &[
                ("username", username.to_string()),
                ("password", password.to_string()),
            ],
            timestamp,
            false,
        );
        let response = host.http_request(request).await?;
        let data = self.decode_encrypted_payload(timestamp, response)?;
        let profile = serde_json::from_str::<GetUserProfileRespData>(&data)
            .map_err(|err| PluginError::Parse(err.to_string()))?;

        Ok(Session {
            token: profile.s.clone(),
            username: profile.username,
            extra: HashMap::from([
                ("uid".to_string(), profile.uid),
                (
                    "photo".to_string(),
                    self.normalize_user_photo(&profile.photo),
                ),
                ("level_name".to_string(), profile.level_name),
            ]),
        })
    }

    pub async fn get_user_profile(&self, host: &dyn HostApi) -> Result<JmUserProfile, PluginError> {
        let timestamp = self.timestamp();
        let request = self.build_signed_post("/login", &[], &[], timestamp, false);
        let response = host.http_request(request).await?;
        let data = self.decode_encrypted_payload(timestamp, response)?;
        let profile = serde_json::from_str::<GetUserProfileRespData>(&data)
            .map_err(|err| PluginError::Parse(err.to_string()))?;

        Ok(self.map_user_profile(profile))
    }

    pub async fn get_favorites(
        &self,
        host: &dyn HostApi,
        folder_id: i64,
        page: u32,
    ) -> Result<FavoriteResult, PluginError> {
        let timestamp = self.timestamp();
        let request = self.build_signed_get(
            "/favorite",
            &[
                ("page", page.to_string()),
                ("o", "mr".to_string()),
                ("folder_id", folder_id.to_string()),
            ],
            timestamp,
            false,
        );
        let response = host.http_request(request).await?;
        let data = self.decode_encrypted_payload(timestamp, response)?;
        let favorites = serde_json::from_str::<GetFavoriteRespData>(&data)
            .map_err(|err| PluginError::Parse(err.to_string()))?;

        let total_pages = favorites.total.parse::<u32>().unwrap_or(1).max(1);
        let folder_name = favorites
            .folder_list
            .first()
            .map(|folder| folder.name.clone());

        Ok(FavoriteResult {
            comics: favorites
                .list
                .into_iter()
                .map(|comic| self.map_favorite_comic(comic))
                .collect(),
            current_page: page,
            total_pages,
            folder_name,
        })
    }

    pub async fn get_weekly_info(&self, host: &dyn HostApi) -> Result<JmWeeklyInfo, PluginError> {
        let timestamp = self.timestamp();
        let request = self.build_signed_get("/week", &[], timestamp, false);
        let response = host.http_request(request).await?;
        let data = self.decode_encrypted_payload(timestamp, response)?;
        let weekly = serde_json::from_str::<GetWeeklyInfoRespData>(&data)
            .map_err(|err| PluginError::Parse(err.to_string()))?;

        Ok(JmWeeklyInfo {
            categories: weekly
                .categories
                .into_iter()
                .map(|category| JmWeeklyCategory {
                    id: category.id,
                    title: category.title,
                    time: category.time,
                })
                .collect(),
            types: weekly
                .type_field
                .into_iter()
                .map(|ty| JmWeeklyType {
                    id: ty.id,
                    title: ty.title,
                })
                .collect(),
        })
    }

    pub async fn get_weekly(
        &self,
        host: &dyn HostApi,
        category_id: &str,
        type_id: &str,
    ) -> Result<WeeklyResult, PluginError> {
        let timestamp = self.timestamp();
        let info = self.get_weekly_info(host).await?;
        let request = self.build_signed_get(
            "/week/filter",
            &[
                ("id", category_id.to_string()),
                ("type", type_id.to_string()),
            ],
            timestamp,
            false,
        );
        let response = host.http_request(request).await?;
        let data = self.decode_encrypted_payload(timestamp, response)?;
        let weekly = serde_json::from_str::<GetWeeklyRespData>(&data)
            .map_err(|err| PluginError::Parse(err.to_string()))?;

        let category_title = info
            .categories
            .iter()
            .find(|category| category.id == category_id)
            .map(|category| category.title.clone())
            .unwrap_or_else(|| category_id.to_string());
        let type_title = info
            .types
            .iter()
            .find(|weekly_type| weekly_type.id == type_id)
            .map(|weekly_type| weekly_type.title.clone())
            .unwrap_or_else(|| type_id.to_string());

        Ok(WeeklyResult {
            title: format!("{category_title} / {type_title}"),
            comics: weekly
                .list
                .into_iter()
                .map(|comic| self.map_weekly_comic(comic))
                .collect(),
        })
    }

    pub async fn search(
        &self,
        host: &dyn HostApi,
        keyword: &str,
        page: u32,
        sort: SearchSort,
    ) -> Result<SearchResult, PluginError> {
        let timestamp = self.timestamp();
        let request = self.build_signed_get(
            "/search",
            &[
                ("main_tag", "0".to_string()),
                ("search_query", keyword.to_string()),
                ("page", page.to_string()),
                ("o", self.search_sort(sort).to_string()),
            ],
            timestamp,
            false,
        );
        let response = host.http_request(request).await?;
        let data = self.decode_encrypted_payload(timestamp, response)?;

        if let Ok(redirect) = serde_json::from_str::<RedirectRespData>(&data) {
            let comic = self.get_comic(host, &redirect.redirect_aid).await?;
            return Ok(SearchResult {
                comics: vec![comic],
                current_page: page,
                total_pages: 1,
            });
        }

        let search = serde_json::from_str::<SearchRespData>(&data)
            .map_err(|err| PluginError::Parse(err.to_string()))?;
        let total_pages = (search.total.max(1) as u32).div_ceil(PAGE_SIZE);

        Ok(SearchResult {
            comics: search
                .content
                .into_iter()
                .map(|comic| self.map_search_comic(comic))
                .collect(),
            current_page: page,
            total_pages,
        })
    }

    pub async fn get_comic(
        &self,
        host: &dyn HostApi,
        comic_id: &str,
    ) -> Result<Comic, PluginError> {
        let timestamp = self.timestamp();
        let request =
            self.build_signed_get("/album", &[("id", comic_id.to_string())], timestamp, false);
        let response = host.http_request(request).await?;
        let data = self.decode_encrypted_payload(timestamp, response)?;
        let comic = serde_json::from_str::<GetComicRespData>(&data)
            .map_err(|err| PluginError::Parse(err.to_string()))?;
        Ok(self.map_comic(comic))
    }

    pub async fn get_chapter_images(
        &self,
        host: &dyn HostApi,
        chapter_id: &str,
    ) -> Result<Vec<ImageUrl>, PluginError> {
        let timestamp = self.timestamp();
        let scramble_request = self.build_signed_get(
            "/chapter_view_template",
            &[
                ("id", chapter_id.to_string()),
                ("v", timestamp.to_string()),
                ("mode", "vertical".to_string()),
                ("page", "0".to_string()),
                ("app_img_shunt", "1".to_string()),
                ("express", "off".to_string()),
            ],
            timestamp,
            true,
        );
        let scramble_response = host.http_request(scramble_request).await?;
        let scramble_id = self.parse_scramble_id(scramble_response)?;

        let chapter_request = self.build_signed_get(
            "/chapter",
            &[("id", chapter_id.to_string())],
            timestamp,
            false,
        );
        let chapter_response = host.http_request(chapter_request).await?;
        let data = self.decode_encrypted_payload(timestamp, chapter_response)?;
        let chapter = serde_json::from_str::<GetChapterRespData>(&data)
            .map_err(|err| PluginError::Parse(err.to_string()))?;

        Ok(chapter
            .images
            .into_iter()
            .filter_map(|filename| self.map_chapter_image(chapter_id, scramble_id, filename))
            .enumerate()
            .map(|(index, mut image)| {
                image.index = index as u32;
                image
            })
            .collect())
    }

    pub fn process_image(
        &self,
        image: &ImageUrl,
        bytes: Vec<u8>,
    ) -> Result<ProcessedImage, PluginError> {
        let block_num = image
            .headers
            .get("x-hmanga-jm-block-num")
            .and_then(|value| value.parse::<u32>().ok())
            .unwrap_or_default();

        let format =
            image::guess_format(&bytes).map_err(|err| PluginError::Parse(err.to_string()))?;
        if format == ImageFormat::Gif {
            return Ok(ProcessedImage {
                bytes,
                extension: "gif",
            });
        }

        let mut source = image::load_from_memory(&bytes)
            .map_err(|err| PluginError::Parse(err.to_string()))?
            .to_rgb8();
        let output = if block_num == 0 {
            source
        } else {
            stitch_image(&mut source, block_num)
        };

        let mut encoded = Vec::new();
        let (img_format, ext) = match self.download_format.as_str() {
            "jpg" => (ImageFormat::Jpeg, "jpg"),
            "png" => (ImageFormat::Png, "png"),
            _ => (ImageFormat::WebP, "webp"),
        };
        image::DynamicImage::ImageRgb8(output)
            .write_to(&mut std::io::Cursor::new(&mut encoded), img_format)
            .map_err(|err| PluginError::Other(err.to_string()))?;

        Ok(ProcessedImage {
            bytes: encoded,
            extension: ext,
        })
    }

    fn map_search_comic(&self, comic: ComicInSearchRespData) -> Comic {
        let mut tags = Vec::new();
        if let Some(title) = comic.category.title {
            tags.push(title);
        }
        if let Some(title) = comic.category_sub.title {
            tags.push(title);
        }

        let mut extra = HashMap::new();
        extra.insert("liked".to_string(), comic.liked.to_string());
        extra.insert("is_favorite".to_string(), comic.is_favorite.to_string());
        extra.insert("update_at".to_string(), comic.update_at.to_string());

        Comic {
            id: comic.id.clone(),
            source: plugin_id().to_string(),
            title: comic.name,
            author: comic.author,
            // Use comic_id to generate cover URL, same as map_comic
            cover_url: comic
                .id
                .parse::<i64>()
                .map(|id| self.normalize_cover_url(None, Some(id)))
                .unwrap_or_default(),
            description: String::new(),
            tags,
            chapters: Vec::new(),
            extra,
            ..Default::default()
        }
    }

    fn map_comic(&self, comic: GetComicRespData) -> Comic {
        let mut extra = HashMap::new();
        extra.insert("addtime".to_string(), comic.addtime.clone());
        extra.insert("total_views".to_string(), comic.total_views.clone());
        extra.insert("likes".to_string(), comic.likes.clone());
        extra.insert("series_id".to_string(), comic.series_id.clone());
        extra.insert("comment_total".to_string(), comic.comment_total.clone());
        extra.insert("liked".to_string(), comic.liked.to_string());
        extra.insert("is_favorite".to_string(), comic.is_favorite.to_string());
        extra.insert("is_aids".to_string(), comic.is_aids.to_string());

        let mut tags = comic.tags;
        tags.extend(comic.works);
        tags.extend(comic.actors);

        let mut chapters = comic
            .series
            .into_iter()
            .enumerate()
            .map(|(index, chapter)| ChapterInfo {
                id: chapter.id,
                title: chapter_title(index + 1, &chapter.name),
                page_count: None,
            })
            .collect::<Vec<_>>();
        if chapters.is_empty() {
            chapters.push(ChapterInfo {
                id: comic.id.to_string(),
                title: "第1话".to_string(),
                page_count: None,
            });
        }

        Comic {
            id: comic.id.to_string(),
            source: plugin_id().to_string(),
            title: comic.name,
            author: comic.author.join(", "),
            cover_url: self.normalize_cover_url(None, Some(comic.id)),
            description: comic.description,
            tags,
            chapters,
            extra,
            ..Default::default()
        }
    }

    fn map_chapter_image(
        &self,
        chapter_id: &str,
        scramble_id: i64,
        filename: String,
    ) -> Option<ImageUrl> {
        let path = Path::new(&filename);
        let extension = path.extension()?.to_str()?.to_ascii_lowercase();
        let file_stem = path.file_stem()?.to_str()?.to_string();
        let chapter_id_num = chapter_id.parse::<i64>().ok()?;

        let block_num = match extension.as_str() {
            "gif" => 0,
            "webp" => calculate_block_num(scramble_id, chapter_id_num, &file_stem),
            _ => return None,
        };

        let mut headers = HashMap::new();
        headers.insert(
            "x-hmanga-jm-scramble-id".to_string(),
            scramble_id.to_string(),
        );
        headers.insert("x-hmanga-jm-file-name".to_string(), file_stem);
        headers.insert("x-hmanga-jm-block-num".to_string(), block_num.to_string());

        Some(ImageUrl {
            url: format!(
                "https://{}/media/photos/{}/{}",
                self.image_domain, chapter_id, filename
            ),
            headers,
            index: 0,
        })
    }

    fn map_user_profile(&self, profile: GetUserProfileRespData) -> JmUserProfile {
        let mut extra = HashMap::new();
        extra.insert("uid".to_string(), profile.uid);
        extra.insert("email".to_string(), profile.email);
        extra.insert("coin".to_string(), profile.coin.to_string());
        extra.insert("exp".to_string(), profile.exp);
        extra.insert("s".to_string(), profile.s);

        JmUserProfile {
            username: profile.username,
            photo: self.normalize_user_photo(&profile.photo),
            level_name: profile.level_name,
            favorites_count: profile.album_favorites,
            favorites_max: profile.album_favorites_max,
            extra,
        }
    }

    fn map_favorite_comic(&self, comic: ComicInFavoriteRespData) -> Comic {
        let mut tags = Vec::new();
        if let Some(title) = comic.category.title {
            tags.push(title);
        }
        if let Some(title) = comic.category_sub.title {
            tags.push(title);
        }

        let mut extra = HashMap::new();
        if let Some(description) = comic.description.clone() {
            extra.insert("description".to_string(), description);
        }
        if let Some(latest_ep) = comic.latest_ep.clone() {
            extra.insert("latest_ep".to_string(), latest_ep);
        }
        if let Some(latest_ep_aid) = comic.latest_ep_aid.clone() {
            extra.insert("latest_ep_aid".to_string(), latest_ep_aid);
        }

        Comic {
            id: comic.id,
            source: plugin_id().to_string(),
            title: comic.name,
            author: comic.author,
            cover_url: self.normalize_cover_url(Some(&comic.image), None),
            description: comic.description.unwrap_or_default(),
            tags,
            chapters: Vec::new(),
            extra,
            ..Default::default()
        }
    }

    fn map_weekly_comic(&self, comic: ComicInWeeklyRespData) -> Comic {
        let mut tags = Vec::new();
        if let Some(title) = comic.category.title {
            tags.push(title);
        }
        if let Some(title) = comic.category_sub.title {
            tags.push(title);
        }

        let mut extra = HashMap::new();
        extra.insert("liked".to_string(), comic.liked.to_string());
        extra.insert("is_favorite".to_string(), comic.is_favorite.to_string());
        extra.insert("update_at".to_string(), comic.update_at.to_string());

        Comic {
            id: comic.id,
            source: plugin_id().to_string(),
            title: comic.name,
            author: comic.author,
            cover_url: self.normalize_cover_url(Some(&comic.image), None),
            description: comic.description,
            tags,
            chapters: Vec::new(),
            extra,
            ..Default::default()
        }
    }

    fn build_signed_get(
        &self,
        path: &str,
        query: &[(&str, String)],
        timestamp: u64,
        content_secret: bool,
    ) -> HttpRequest {
        let token_secret = if content_secret {
            APP_TOKEN_SECRET_2
        } else {
            APP_TOKEN_SECRET
        };
        let token = md5_hex(&format!("{timestamp}{token_secret}"));
        let tokenparam = format!("{timestamp},{APP_VERSION}");
        let query_string = build_query(query);
        let url = if query_string.is_empty() {
            format!("https://{}{}", self.api_domain, path)
        } else {
            format!("https://{}{}?{}", self.api_domain, path, query_string)
        };

        let mut headers = HashMap::new();
        headers.insert("token".to_string(), token);
        headers.insert("tokenparam".to_string(), tokenparam);
        headers.insert("user-agent".to_string(), DEFAULT_USER_AGENT.to_string());

        HttpRequest {
            url,
            method: HttpMethod::Get,
            headers,
            body: None,
        }
    }

    fn build_signed_post(
        &self,
        path: &str,
        query: &[(&str, String)],
        form: &[(&str, String)],
        timestamp: u64,
        content_secret: bool,
    ) -> HttpRequest {
        let token_secret = if content_secret {
            APP_TOKEN_SECRET_2
        } else {
            APP_TOKEN_SECRET
        };
        let token = md5_hex(&format!("{timestamp}{token_secret}"));
        let tokenparam = format!("{timestamp},{APP_VERSION}");
        let query_string = build_query(query);
        let form_body = build_query(form).into_bytes();
        let url = if query_string.is_empty() {
            format!("https://{}{}", self.api_domain, path)
        } else {
            format!("https://{}{}?{}", self.api_domain, path, query_string)
        };

        let mut headers = HashMap::new();
        headers.insert("token".to_string(), token);
        headers.insert("tokenparam".to_string(), tokenparam);
        headers.insert("user-agent".to_string(), DEFAULT_USER_AGENT.to_string());
        headers.insert(
            "content-type".to_string(),
            "application/x-www-form-urlencoded".to_string(),
        );

        HttpRequest {
            url,
            method: HttpMethod::Post,
            headers,
            body: Some(form_body),
        }
    }

    fn timestamp(&self) -> u64 {
        self.fixed_timestamp.unwrap_or_else(|| {
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time before unix epoch")
                .as_secs()
        })
    }

    fn search_sort(&self, sort: SearchSort) -> &'static str {
        match sort {
            SearchSort::Latest => "mr",
            SearchSort::Popular => "mv",
            SearchSort::Relevance => "tf",
        }
    }

    fn decode_encrypted_payload(
        &self,
        timestamp: u64,
        response: HttpResponse,
    ) -> Result<String, PluginError> {
        if response.status != 200 {
            return Err(PluginError::Network(format!(
                "unexpected status: {}",
                response.status
            )));
        }

        let body =
            String::from_utf8(response.body).map_err(|err| PluginError::Parse(err.to_string()))?;
        let payload = serde_json::from_str::<JmResp>(&body)
            .map_err(|err| PluginError::Parse(err.to_string()))?;

        if payload.code != 200 {
            return Err(PluginError::Other(if payload.error_msg.is_empty() {
                format!("jm api returned code {}", payload.code)
            } else {
                payload.error_msg
            }));
        }

        let encrypted = payload
            .data
            .as_str()
            .ok_or_else(|| PluginError::Parse("jm api data is not string".to_string()))?;

        decrypt_data(timestamp, encrypted)
    }

    fn parse_scramble_id(&self, response: HttpResponse) -> Result<i64, PluginError> {
        if response.status != 200 {
            return Err(PluginError::Network(format!(
                "unexpected status: {}",
                response.status
            )));
        }
        let body =
            String::from_utf8(response.body).map_err(|err| PluginError::Parse(err.to_string()))?;
        Ok(body
            .split("var scramble_id = ")
            .nth(1)
            .and_then(|rest| rest.split(';').next())
            .and_then(|value| value.trim().parse::<i64>().ok())
            .unwrap_or(220_980))
    }

    fn normalize_cover_url(&self, image: Option<&str>, comic_id: Option<i64>) -> String {
        if let Some(image) = image {
            if image.starts_with("http://") || image.starts_with("https://") {
                return image.to_string();
            }
            return format!("https://{}/media/albums/{}", self.album_domain, image);
        }

        match comic_id {
            Some(id) => format!("https://{}/media/albums/{}_3x4.jpg", self.album_domain, id),
            None => String::new(),
        }
    }

    fn normalize_user_photo(&self, photo: &str) -> String {
        format!("https://{}/media/users/{}", self.image_domain, photo)
    }
}

#[async_trait]
impl DynPlugin for JmPlugin {
    fn id(&self) -> &str {
        plugin_id()
    }

    fn meta(&self) -> PluginMetaInfo {
        self.meta()
    }

    async fn search(
        &self,
        host: &dyn HostApi,
        query: &str,
        page: u32,
        sort: SearchSort,
    ) -> PluginResult<SearchResult> {
        self.search(host, query, page, sort).await
    }

    async fn get_comic(&self, host: &dyn HostApi, comic_id: &str) -> PluginResult<Comic> {
        self.get_comic(host, comic_id).await
    }

    async fn get_chapter_images(
        &self,
        host: &dyn HostApi,
        chapter_id: &str,
    ) -> PluginResult<Vec<ImageUrl>> {
        self.get_chapter_images(host, chapter_id).await
    }

    async fn login(
        &self,
        host: &dyn HostApi,
        username: &str,
        password: &str,
    ) -> PluginResult<Session> {
        self.login(host, username, password).await
    }

    async fn get_favorites(
        &self,
        host: &dyn HostApi,
        _session: Option<&Session>,
        page: u32,
    ) -> PluginResult<FavoriteResult> {
        self.get_favorites(host, 0, page).await
    }

    async fn get_weekly(&self, host: &dyn HostApi) -> PluginResult<WeeklyResult> {
        let info = self.get_weekly_info(host).await?;
        let first_category = info
            .categories
            .first()
            .ok_or_else(|| PluginError::Other("no weekly categories".to_string()))?;
        let first_type = info
            .types
            .first()
            .ok_or_else(|| PluginError::Other("no weekly types".to_string()))?;
        self.get_weekly(host, &first_category.id, &first_type.id)
            .await
    }
}

pub fn plugin_id() -> &'static str {
    "jm"
}

pub fn calculate_block_num(scramble_id: i64, chapter_id: i64, filename: &str) -> u32 {
    if chapter_id < scramble_id {
        0
    } else if chapter_id < 268_850 {
        10
    } else {
        let modulus = if chapter_id < 421_926 { 10 } else { 8 };
        let digest = md5_hex(&format!("{chapter_id}{filename}"));
        let mut block_num = digest.chars().last().unwrap_or('0') as u32;
        block_num %= modulus;
        block_num * 2 + 2
    }
}

fn chapter_title(order: usize, suffix: &str) -> String {
    if suffix.is_empty() {
        format!("第{order}话")
    } else {
        format!("第{order}话 {suffix}")
    }
}

fn build_query(params: &[(&str, String)]) -> String {
    params
        .iter()
        .map(|(key, value)| format!("{}={}", percent_encode(key), percent_encode(value)))
        .collect::<Vec<_>>()
        .join("&")
}

fn percent_encode(value: &str) -> String {
    value
        .bytes()
        .flat_map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                vec![byte as char].into_iter().collect::<Vec<_>>()
            }
            _ => format!("%{:02X}", byte).chars().collect::<Vec<_>>(),
        })
        .collect()
}

fn md5_hex(data: &str) -> String {
    format!("{:x}", md5::compute(data))
}

fn decrypt_data(timestamp: u64, data: &str) -> Result<String, PluginError> {
    let encrypted = base64::engine::general_purpose::STANDARD
        .decode(data)
        .map_err(|err| PluginError::Parse(err.to_string()))?;
    let key = md5_hex(&format!("{timestamp}{APP_DATA_SECRET}"));
    let cipher = Aes256::new(GenericArray::from_slice(key.as_bytes()));

    let decrypted_with_padding = encrypted
        .chunks(16)
        .map(GenericArray::clone_from_slice)
        .flat_map(|mut block| {
            cipher.decrypt_block(&mut block);
            block.to_vec()
        })
        .collect::<Vec<_>>();

    let padding = decrypted_with_padding
        .last()
        .copied()
        .ok_or_else(|| PluginError::Parse("empty encrypted payload".to_string()))?
        as usize;
    let decrypted = decrypted_with_padding
        .get(..decrypted_with_padding.len().saturating_sub(padding))
        .ok_or_else(|| PluginError::Parse("invalid padding".to_string()))?;

    String::from_utf8(decrypted.to_vec()).map_err(|err| PluginError::Parse(err.to_string()))
}

fn stitch_image(source: &mut image::RgbImage, block_num: u32) -> image::RgbImage {
    let (width, height) = source.dimensions();
    let mut stitched = image::ImageBuffer::new(width, height);
    let remainder_height = height % block_num;

    for index in 0..block_num {
        let mut block_height = height / block_num;
        let source_y_start = height - (block_height * (index + 1)) - remainder_height;
        let mut target_y_start = block_height * index;

        if index == 0 {
            block_height += remainder_height;
        } else {
            target_y_start += remainder_height;
        }

        for y in 0..block_height {
            let source_y = source_y_start + y;
            let target_y = target_y_start + y;
            for x in 0..width {
                stitched.put_pixel(x, target_y, *source.get_pixel(x, source_y));
            }
        }
    }

    stitched
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct JmResp {
    code: i64,
    data: Value,
    #[serde(default)]
    error_msg: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RedirectRespData {
    redirect_aid: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SearchRespData {
    #[serde(deserialize_with = "string_or_i64")]
    total: i64,
    content: Vec<ComicInSearchRespData>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ComicInSearchRespData {
    id: String,
    author: String,
    name: String,
    category: CategoryRespData,
    #[serde(rename = "category_sub")]
    category_sub: CategorySubRespData,
    liked: bool,
    #[serde(rename = "is_favorite")]
    is_favorite: bool,
    #[serde(rename = "update_at")]
    update_at: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CategoryRespData {
    title: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CategorySubRespData {
    title: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GetComicRespData {
    id: i64,
    name: String,
    addtime: String,
    description: String,
    #[serde(rename = "total_views")]
    total_views: String,
    likes: String,
    series: Vec<SeriesRespData>,
    #[serde(rename = "series_id")]
    series_id: String,
    #[serde(rename = "comment_total")]
    comment_total: String,
    author: Vec<String>,
    tags: Vec<String>,
    works: Vec<String>,
    actors: Vec<String>,
    liked: bool,
    #[serde(rename = "is_favorite")]
    is_favorite: bool,
    #[serde(rename = "is_aids")]
    is_aids: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SeriesRespData {
    id: String,
    name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GetChapterRespData {
    images: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GetUserProfileRespData {
    uid: String,
    username: String,
    email: String,
    photo: String,
    #[serde(deserialize_with = "string_or_i64")]
    coin: i64,
    #[serde(rename = "album_favorites")]
    album_favorites: i64,
    s: String,
    #[serde(rename = "level_name")]
    level_name: String,
    #[serde(rename = "album_favorites_max")]
    album_favorites_max: i64,
    exp: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GetFavoriteRespData {
    list: Vec<ComicInFavoriteRespData>,
    #[serde(rename = "folder_list")]
    folder_list: Vec<FavoriteFolderRespData>,
    total: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ComicInFavoriteRespData {
    id: String,
    author: String,
    description: Option<String>,
    name: String,
    latest_ep: Option<String>,
    latest_ep_aid: Option<String>,
    image: String,
    category: CategoryRespData,
    #[serde(rename = "category_sub")]
    category_sub: CategorySubRespData,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FavoriteFolderRespData {
    name: String,
}

#[derive(Debug, Deserialize)]
struct GetWeeklyInfoRespData {
    categories: Vec<WeeklyCategoryRespData>,
    #[serde(rename = "type")]
    type_field: Vec<WeeklyTypeRespData>,
}

#[derive(Debug, Deserialize)]
struct WeeklyCategoryRespData {
    id: String,
    title: String,
    time: String,
}

#[derive(Debug, Deserialize)]
struct WeeklyTypeRespData {
    id: String,
    title: String,
}

#[derive(Debug, Deserialize)]
struct GetWeeklyRespData {
    list: Vec<ComicInWeeklyRespData>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ComicInWeeklyRespData {
    #[serde(deserialize_with = "string_value")]
    id: String,
    author: String,
    description: String,
    name: String,
    image: String,
    category: CategoryRespData,
    #[serde(rename = "category_sub")]
    category_sub: CategorySubRespData,
    liked: bool,
    #[serde(rename = "is_favorite")]
    is_favorite: bool,
    #[serde(rename = "update_at")]
    update_at: i64,
}

fn string_or_i64<'de, D>(deserializer: D) -> Result<i64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    match Value::deserialize(deserializer)? {
        Value::Number(number) => Ok(number.as_i64().unwrap_or_default()),
        Value::String(value) => value
            .parse::<i64>()
            .map_err(|err| serde::de::Error::custom(err.to_string())),
        other => Err(serde::de::Error::custom(format!(
            "unsupported number value: {other}"
        ))),
    }
}

fn string_value<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    match Value::deserialize(deserializer)? {
        Value::Number(number) => Ok(number.to_string()),
        Value::String(value) => Ok(value),
        other => Err(serde::de::Error::custom(format!(
            "unsupported string value: {other}"
        ))),
    }
}
