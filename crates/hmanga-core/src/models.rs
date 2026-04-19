use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PluginMetaInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    pub sdk_version: u32,
    pub icon: Vec<u8>,
    pub description: String,
    pub capabilities: Capabilities,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Capabilities {
    pub search: bool,
    pub login: bool,
    pub favorites: bool,
    pub ranking: bool,
    pub weekly: bool,
    pub tags_browsing: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PluginKind {
    OfficialBundled,
    OfficialInstallable,
    ThirdParty,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PluginRuntimeKind {
    Native,
    Wasm,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PluginHealth {
    Healthy,
    Disabled,
    LoadError(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PluginInfo {
    pub meta: PluginMetaInfo,
    pub kind: PluginKind,
    pub runtime: PluginRuntimeKind,
    pub installed: bool,
    pub unlocked: bool,
    pub enabled: bool,
    pub health: PluginHealth,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Error)]
pub enum PluginError {
    #[error("feature not supported")]
    NotSupported,
    #[error("network error: {0}")]
    Network(String),
    #[error("parse error: {0}")]
    Parse(String),
    #[error("auth error: {0}")]
    Auth(String),
    #[error("{0}")]
    Other(String),
}

/// niuhuan/jmcomic-downloader compatible metadata subset.
/// Serialized as `booker.json` for cross-compatibility.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NiuhuanCompat {
    pub id: i64,
    pub name: String,
    #[serde(default)]
    pub author: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub addtime: String,
    #[serde(default)]
    pub total_views: String,
    #[serde(default)]
    pub likes: String,
    #[serde(default)]
    pub comment_total: String,
    #[serde(default)]
    pub series_id: String,
    #[serde(default)]
    pub works: Vec<String>,
    #[serde(default)]
    pub actors: Vec<String>,
}

impl NiuhuanCompat {
    /// Convert to Hmanga's Comic, filling in defaults for incompatible fields.
    pub fn to_comic(&self) -> Comic {
        let mut extra = std::collections::HashMap::new();
        if !self.addtime.is_empty() {
            extra.insert("addtime".to_string(), self.addtime.clone());
        }
        if !self.total_views.is_empty() {
            extra.insert("total_views".to_string(), self.total_views.clone());
        }
        if !self.likes.is_empty() {
            extra.insert("likes".to_string(), self.likes.clone());
        }
        if !self.comment_total.is_empty() {
            extra.insert("comment_total".to_string(), self.comment_total.clone());
        }
        if !self.series_id.is_empty() {
            extra.insert("series_id".to_string(), self.series_id.clone());
        }
        if !self.works.is_empty() {
            extra.insert("works".to_string(), self.works.join(","));
        }
        if !self.actors.is_empty() {
            extra.insert("actors".to_string(), self.actors.join(","));
        }

        Comic {
            id: self.id.to_string(),
            source: "jm".to_string(),
            title: self.name.clone(),
            author: self.author.join(", "),
            cover_url: String::new(),
            description: self.description.clone(),
            tags: self.tags.clone(),
            chapters: Vec::new(),
            extra,
            strict: Some(self.clone()),
        }
    }

    /// Convert from Hmanga's Comic, using only compatible fields.
    /// ID is parsed as i64 if possible.
    pub fn from_comic(comic: &Comic) -> Option<Self> {
        let id = comic.id.parse::<i64>().ok()?;
        Some(Self {
            id,
            name: comic.title.clone(),
            author: comic.author.split(", ").map(|s| s.to_string()).collect(),
            tags: comic.tags.clone(),
            description: comic.description.clone(),
            addtime: comic.extra.get("addtime").cloned().unwrap_or_default(),
            total_views: comic.extra.get("total_views").cloned().unwrap_or_default(),
            likes: comic.extra.get("likes").cloned().unwrap_or_default(),
            comment_total: comic
                .extra
                .get("comment_total")
                .cloned()
                .unwrap_or_default(),
            series_id: comic.extra.get("series_id").cloned().unwrap_or_default(),
            works: comic
                .extra
                .get("works")
                .map(|s| s.split(',').map(|s| s.to_string()).collect())
                .unwrap_or_default(),
            actors: comic
                .extra
                .get("actors")
                .map(|s| s.split(',').map(|s| s.to_string()).collect())
                .unwrap_or_default(),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct Comic {
    pub id: String,
    pub source: String,
    pub title: String,
    pub author: String,
    pub cover_url: String,
    pub description: String,
    pub tags: Vec<String>,
    pub chapters: Vec<ChapterInfo>,
    pub extra: HashMap<String, String>,
    /// niuhuan/jmcomic-downloader compatible subset, serialized as `booker.json`.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub strict: Option<NiuhuanCompat>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChapterInfo {
    pub id: String,
    pub title: String,
    pub page_count: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SearchResult {
    pub comics: Vec<Comic>,
    pub current_page: u32,
    pub total_pages: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ImageUrl {
    pub url: String,
    pub headers: HashMap<String, String>,
    pub index: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ImageContext {
    pub comic_id: String,
    pub chapter_id: String,
    pub page_index: u32,
    pub extra: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Session {
    pub token: String,
    pub username: String,
    pub extra: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FavoriteResult {
    pub comics: Vec<Comic>,
    pub current_page: u32,
    pub total_pages: u32,
    pub folder_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WeeklyResult {
    pub title: String,
    pub comics: Vec<Comic>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HttpRequest {
    pub url: String,
    pub method: HttpMethod,
    pub headers: HashMap<String, String>,
    pub body: Option<Vec<u8>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HttpResponse {
    pub status: u16,
    pub headers: HashMap<String, Vec<String>>,
    pub body: Vec<u8>,
}

impl HttpResponse {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .get(&name.to_ascii_lowercase())
            .and_then(|values| values.first())
            .map(String::as_str)
    }

    pub fn header_values(&self, name: &str) -> Option<&[String]> {
        self.headers
            .get(&name.to_ascii_lowercase())
            .map(Vec::as_slice)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SearchSort {
    Latest,
    Popular,
    Relevance,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum DownloadFormat {
    Raw,
    Cbz,
    Pdf,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

pub type TaskId = u64;
pub type SiteId = String;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChapterTask {
    pub chapter: ChapterInfo,
    pub downloaded_pages: u32,
    pub total_pages: Option<u32>,
    pub output_dir: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DownloadTask {
    pub id: TaskId,
    pub source: String,
    pub comic: Comic,
    pub chapters: Vec<ChapterTask>,
    pub state: DownloadTaskState,
    pub output_dir: PathBuf,
    pub format: DownloadFormat,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DownloadTaskState {
    Pending,
    Downloading { progress: f32 },
    Paused,
    Completed,
    Failed { error: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DownloadEvent {
    TaskCreated(TaskId),
    Progress {
        task_id: TaskId,
        chapter_id: String,
        downloaded: u32,
        total: u32,
    },
    SpeedUpdate(u64),
    TaskCompleted(TaskId),
    TaskFailed {
        task_id: TaskId,
        error: String,
    },
    ExportProgress {
        task_id: TaskId,
        format: DownloadFormat,
        progress: f32,
    },
}
