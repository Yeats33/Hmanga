use std::collections::HashMap;
use std::path::PathBuf;

use hmanga_core::{
    AppConfig, ChapterInfo, ChapterTask, Comic, DownloadFormat, DownloadHistory, DownloadTask,
    DownloadTaskState, PluginError,
};

#[test]
fn plugin_error_roundtrips_via_rmp_serde() {
    let input = PluginError::Parse("bad payload".to_string());
    let encoded = rmp_serde::to_vec(&input).expect("encode plugin error");
    let decoded: PluginError = rmp_serde::from_slice(&encoded).expect("decode plugin error");

    assert_eq!(decoded, input);
}

#[test]
fn app_config_default_enables_only_jm() {
    let config = AppConfig::default();

    assert_eq!(config.enabled_plugins, vec!["jm".to_string()]);
}

#[test]
fn app_config_defaults_to_donation_locked() {
    let config = AppConfig::default();

    assert!(!config.donation_unlocked);
}

#[test]
fn app_config_defaults_include_per_site_settings() {
    let config = AppConfig::default();

    assert_eq!(config.sites.jm.api_domain, "www.cdnhth.cc");
    assert_eq!(config.sites.wnacg.api_domain, "www.wnacg.com");
    assert!(config.sites.jm.use_global_download_format);
    assert!(config.sites.wnacg.use_global_download_format);
    assert!(config.sites.jm.use_global_cover_preference);
    assert!(config.sites.wnacg.use_global_cover_preference);
}

#[test]
fn download_history_roundtrips_with_pending_task() {
    let comic = Comic {
        id: "comic-1".to_string(),
        source: "jm".to_string(),
        title: "Example".to_string(),
        author: "Author".to_string(),
        cover_url: "https://example.invalid/cover.jpg".to_string(),
        description: "desc".to_string(),
        tags: vec!["tag".to_string()],
        chapters: vec![ChapterInfo {
            id: "chapter-1".to_string(),
            title: "Chapter 1".to_string(),
            page_count: Some(1),
        }],
        extra: HashMap::new(),
        ..Default::default()
    };
    let task = DownloadTask {
        id: 1,
        source: "jm".to_string(),
        comic: comic.clone(),
        chapters: vec![ChapterTask {
            chapter: comic.chapters[0].clone(),
            downloaded_pages: 0,
            total_pages: Some(1),
            output_dir: PathBuf::from("downloads/jm/example/chapter-1"),
        }],
        state: DownloadTaskState::Pending,
        output_dir: PathBuf::from("downloads/jm/example"),
        format: DownloadFormat::Raw,
    };
    let history = DownloadHistory {
        version: 1,
        tasks: vec![task],
    };

    let encoded = rmp_serde::to_vec(&history).expect("encode download history");
    let decoded: DownloadHistory =
        rmp_serde::from_slice(&encoded).expect("decode download history");

    assert_eq!(decoded, history);
}
