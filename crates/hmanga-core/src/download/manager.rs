use std::collections::HashMap;
use std::path::PathBuf;

use tokio::sync::broadcast;

use crate::{ChapterTask, DownloadEvent, DownloadTask, TaskId};

use super::export::{EventCallback, ExportRunner};
use super::speed::SpeedTracker;

/// DownloadManager orchestrates concurrent chapter downloads and CBZ/PDF export.
#[derive(Debug)]
pub struct DownloadManager {
    tasks: HashMap<TaskId, DownloadTask>,
    #[allow(dead_code)]
    tx: broadcast::Sender<DownloadEvent>,
    #[allow(dead_code)]
    speed_tracker: SpeedTracker,
}

impl DownloadManager {
    pub fn new() -> (Self, broadcast::Receiver<DownloadEvent>) {
        let (tx, rx) = broadcast::channel(100);
        let manager = Self {
            tasks: HashMap::new(),
            tx,
            speed_tracker: SpeedTracker::default(),
        };
        (manager, rx)
    }

    /// Add a download task.
    pub fn add_task(&mut self, task: DownloadTask) {
        self.tasks.insert(task.id, task);
    }

    /// Determine the page to resume from by checking existing files on disk.
    /// Returns the page index to resume from (0-based), or 0 if no files exist.
    pub fn resume_from_page(&self, chapter_task: &ChapterTask) -> u32 {
        let output_dir = &chapter_task.output_dir;
        let mut page = 0;
        loop {
            let filename = format!("{:04}.jpg", page + 1);
            if output_dir.join(&filename).exists() {
                page += 1;
            } else {
                break;
            }
        }
        page
    }

    /// Emit a download event to all subscribers.
    #[allow(dead_code)]
    fn emit(&self, event: DownloadEvent) {
        let _ = self.tx.send(event);
    }

    /// Run CBZ export for a completed download task.
    pub fn export_cbz(
        &self,
        task_id: TaskId,
        image_dir: PathBuf,
        output_path: PathBuf,
    ) -> Result<(), String> {
        let _task = self
            .tasks
            .get(&task_id)
            .ok_or_else(|| "task not found".to_string())?;

        let tx = self.tx.clone();
        let callback: EventCallback = Box::new(move |ev: DownloadEvent| {
            let _ = tx.send(ev);
        });

        let runner = ExportRunner::new();
        runner.run_cbz(task_id, &image_dir, &output_path, &callback)
    }
}

impl Default for DownloadManager {
    fn default() -> Self {
        let (manager, _) = Self::new();
        manager
    }
}
