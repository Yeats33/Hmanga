pub mod config;
pub mod error;
pub mod models;
pub mod persistence;

pub use config::{AppConfig, ConfigVersioned, ThemeMode, APP_CONFIG_VERSION};
pub use error::{HmangaError, PluginResult};
pub use models::*;
pub use persistence::{
    DownloadHistory, ReadingProgressEntry, ReadingProgressStore, SessionStore,
    DOWNLOAD_HISTORY_VERSION, READING_PROGRESS_VERSION, SESSION_STORE_VERSION,
};
