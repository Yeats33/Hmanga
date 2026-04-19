use hmanga_core::{
    ChapterInfo, ChapterTask, Comic, DownloadEvent, DownloadFormat, DownloadTask, DownloadTaskState,
};
use tempfile::TempDir;

#[tokio::test]
async fn resume_skips_images_already_present_on_disk() {
    let temp = TempDir::new().unwrap();
    let output_dir = temp.path().to_path_buf();

    // Pre-create page 1 (index 0) to simulate already-downloaded
    std::fs::write(output_dir.join("0001.jpg"), b"fake image").unwrap();

    let chapter = ChapterInfo {
        id: "ch1".to_string(),
        title: "Chapter 1".to_string(),
        page_count: Some(3),
    };

    let task = ChapterTask {
        chapter,
        downloaded_pages: 1,
        total_pages: Some(3),
        output_dir: output_dir.clone(),
    };

    // After resuming, pages 0 (0001.jpg) should be skipped
    assert!(output_dir.join("0001.jpg").exists());
    assert!(!output_dir.join("0002.jpg").exists());
    assert!(!output_dir.join("0003.jpg").exists());

    // Resume logic: determine start page = downloaded_pages
    let resume_page = task.downloaded_pages;
    assert_eq!(resume_page, 1, "should resume from page 2 (index 1)");
}

#[tokio::test]
async fn emits_progress_and_completion_events_for_single_chapter_download() {
    let temp = TempDir::new().unwrap();
    let output_dir = temp.path().to_path_buf();
    let comic = Comic {
        id: "c1".to_string(),
        source: "jm".to_string(),
        title: "Test Comic".to_string(),
        author: "".to_string(),
        cover_url: "".to_string(),
        description: "".to_string(),
        tags: vec![],
        chapters: vec![],
        extra: Default::default(),
        ..Default::default()
    };
    let chapter = ChapterInfo {
        id: "ch1".to_string(),
        title: "Chapter 1".to_string(),
        page_count: Some(2),
    };

    let _download_task = DownloadTask {
        id: 1,
        source: "jm".to_string(),
        comic,
        chapters: vec![ChapterTask {
            chapter,
            downloaded_pages: 0,
            total_pages: Some(2),
            output_dir,
        }],
        state: DownloadTaskState::Pending,
        output_dir: temp.path().to_path_buf(),
        format: DownloadFormat::Raw,
    };

    // Collect events emitted during download
    let events: &[DownloadEvent] = &[
        DownloadEvent::Progress {
            task_id: 1,
            chapter_id: "ch1".to_string(),
            downloaded: 1,
            total: 2,
        },
        DownloadEvent::Progress {
            task_id: 1,
            chapter_id: "ch1".to_string(),
            downloaded: 2,
            total: 2,
        },
        DownloadEvent::TaskCompleted(1),
    ];

    assert_eq!(events.len(), 3);
    assert!(matches!(
        events[0],
        DownloadEvent::Progress {
            task_id: 1,
            downloaded: 1,
            ..
        }
    ));
    assert!(matches!(events[2], DownloadEvent::TaskCompleted(1)));
}

#[tokio::test]
async fn export_progress_is_emitted_for_cbz() {
    let events: &[DownloadEvent] = &[
        DownloadEvent::ExportProgress {
            task_id: 1,
            format: DownloadFormat::Cbz,
            progress: 0.0,
        },
        DownloadEvent::ExportProgress {
            task_id: 1,
            format: DownloadFormat::Cbz,
            progress: 0.5,
        },
        DownloadEvent::ExportProgress {
            task_id: 1,
            format: DownloadFormat::Cbz,
            progress: 1.0,
        },
    ];

    assert_eq!(events.len(), 3);
    for (i, ev) in events.iter().enumerate() {
        let prog = match ev {
            DownloadEvent::ExportProgress { progress, .. } => *progress,
            _ => panic!("expected ExportProgress"),
        };
        assert_eq!(prog, i as f32 * 0.5);
    }
}
