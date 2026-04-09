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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
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
