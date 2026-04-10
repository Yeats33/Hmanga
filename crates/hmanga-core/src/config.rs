use std::path::PathBuf;

use serde::{Deserialize, Serialize};

pub const APP_CONFIG_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct AppConfig {
    pub version: u32,
    pub donation_unlocked: bool,
    pub download_dir: PathBuf,
    pub export_dir: PathBuf,
    pub chapter_concurrency: usize,
    pub chapter_download_interval_sec: u64,
    pub image_concurrency: usize,
    pub image_download_interval_sec: u64,
    pub download_all_favorites_interval_sec: u64,
    pub update_downloaded_comics_interval_sec: u64,
    pub api_domain: String,
    pub custom_api_domain: String,
    pub should_download_cover: bool,
    pub download_format: String,
    pub proxy: Option<String>,
    pub enabled_plugins: Vec<String>,
    pub jm_username: String,
    pub jm_password: String,
    pub theme: ThemeMode,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            version: APP_CONFIG_VERSION,
            donation_unlocked: false,
            download_dir: PathBuf::from("Comics"),
            export_dir: PathBuf::from("Exports"),
            chapter_concurrency: 3,
            chapter_download_interval_sec: 0,
            image_concurrency: 5,
            image_download_interval_sec: 0,
            download_all_favorites_interval_sec: 0,
            update_downloaded_comics_interval_sec: 0,
            api_domain: "www.cdnhth.cc".to_string(),
            custom_api_domain: String::new(),
            should_download_cover: true,
            download_format: "webp".to_string(),
            proxy: None,
            enabled_plugins: vec!["jm".to_string()],
            jm_username: String::new(),
            jm_password: String::new(),
            theme: ThemeMode::Auto,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ThemeMode {
    Auto,
    Light,
    Dark,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConfigVersioned<T> {
    pub version: u32,
    pub data: T,
}
