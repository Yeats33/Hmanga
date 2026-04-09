use std::io::Write;

use crate::{DownloadEvent, DownloadFormat, TaskId};

/// Callback type for emitting download events.
pub type EventCallback = Box<dyn Fn(DownloadEvent) + Send + Sync>;

/// ExportRunner produces CBZ or PDF archives from downloaded images.
#[derive(Debug)]
pub struct ExportRunner {
    _priv: (),
}

impl ExportRunner {
    /// Create a new export runner.
    pub fn new() -> Self {
        Self { _priv: () }
    }

    /// Run CBZ export, emitting progress events via `on_event`.
    pub fn run_cbz(
        &self,
        task_id: TaskId,
        image_dir: &std::path::Path,
        output_path: &std::path::Path,
        on_event: &EventCallback,
    ) -> Result<(), String> {
        let mut images: Vec<_> = std::fs::read_dir(image_dir)
            .map_err(|e| e.to_string())?
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path()
                    .extension()
                    .map(|ext| ext == "jpg" || ext == "png" || ext == "webp")
                    .unwrap_or(false)
            })
            .collect();

        images.sort_by_key(|e| e.path());

        let total = images.len();
        if total == 0 {
            return Err("no images found".to_string());
        }

        let file = std::fs::File::create(output_path).map_err(|e| e.to_string())?;
        let mut zip = zip::ZipWriter::new(file);
        let opts = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);

        for (i, img) in images.iter().enumerate() {
            let name = format!(
                "{:04}.{}",
                i + 1,
                img.path().extension().unwrap_or_default().to_string_lossy()
            );
            zip.start_file(&name, opts).map_err(|e| e.to_string())?;
            let data = std::fs::read(img.path()).map_err(|e| e.to_string())?;
            zip.write_all(&data).map_err(|e| e.to_string())?;

            on_event(DownloadEvent::ExportProgress {
                task_id,
                format: DownloadFormat::Cbz,
                progress: (i + 1) as f32 / total as f32,
            });
        }

        zip.finish().map_err(|e| e.to_string())?;
        Ok(())
    }
}

impl Default for ExportRunner {
    fn default() -> Self {
        Self::new()
    }
}
