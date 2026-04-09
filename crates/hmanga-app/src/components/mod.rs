use dioxus::prelude::*;

mod sidebar;
mod search_pane;
mod downloads_pane;
mod settings_pane;

pub use sidebar::Sidebar;
pub use search_pane::SearchPane;
pub use downloads_pane::DownloadsPane;
pub use settings_pane::SettingsPane;
