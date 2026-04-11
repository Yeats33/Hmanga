use hmanga_core::AppConfig;
use hmanga_core::Comic;
use hmanga_plugin_jm::JmUserProfile;
use hmanga_plugin_wnacg::WnacgUserProfile;
use std::path::PathBuf;

use crate::service::LocalComicEntry;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SiteTab {
    Aggregate,
    Jm,
    Wnacg,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkspaceTab {
    Downloads,
    Library,
    Settings,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LibrarySort {
    DownloadDate,
    UpdateDate,
    Title,
    Author,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DownloadPanelTab {
    Queue,
    Preview,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BrowseTab {
    Search,
    Favorites,
    Weekly,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CollectionViewMode {
    List,
    Image,
    SingleColumn,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrowseFilter {
    pub id: String,
    pub label: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DownloadRow {
    pub chapter_id: String,
    pub label: String,
    pub comic_title: String,
    pub chapter_title: String,
    pub chapter_dir: PathBuf,
    pub status: DownloadRowState,
    pub detail: String,
    pub downloaded_pages: u32,
    pub total_pages: u32,
    pub current_item: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DownloadRowState {
    Downloading,
    Paused,
    Completed,
    Failed,
    Cancelled,
}

impl DownloadRowState {
    pub fn label(&self) -> &'static str {
        match self {
            DownloadRowState::Downloading => "下载中",
            DownloadRowState::Paused => "已暂停",
            DownloadRowState::Completed => "已完成",
            DownloadRowState::Failed => "失败",
            DownloadRowState::Cancelled => "已取消",
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ReaderState {
    pub title: String,
    pub pages: Vec<String>,
    pub current_index: usize,
    pub source_dir: Option<PathBuf>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UiState {
    pub site_tab: SiteTab,
    pub workspace_tab: WorkspaceTab,
    pub download_panel_tab: DownloadPanelTab,
    pub browse_tab: BrowseTab,
    pub browse_view_mode: CollectionViewMode,
    pub library_view_mode: CollectionViewMode,
    pub search_query: String,
    pub jm_username: String,
    pub jm_password: String,
    pub jm_profile: Option<JmUserProfile>,
    pub wnacg_username: String,
    pub wnacg_password: String,
    pub wnacg_profile: Option<WnacgUserProfile>,
    pub settings_config: AppConfig,
    pub favorites_page: u32,
    pub favorites_total_pages: u32,
    pub weekly_categories: Vec<BrowseFilter>,
    pub weekly_types: Vec<BrowseFilter>,
    pub selected_weekly_category: Option<String>,
    pub selected_weekly_type: Option<String>,
    pub status: String,
    pub loading: bool,
    pub search_results: Vec<Comic>,
    pub search_current_page: u32,
    pub search_total_pages: u32,
    pub search_query_text: String,
    pub selected_comic: Option<Comic>,
    pub downloads: Vec<DownloadRow>,
    pub library: Vec<LocalComicEntry>,
    pub library_sort: LibrarySort,
    pub reader: ReaderState,
    pub reader_fullscreen: bool,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            site_tab: SiteTab::Aggregate,
            workspace_tab: WorkspaceTab::Downloads,
            download_panel_tab: DownloadPanelTab::Queue,
            browse_tab: BrowseTab::Search,
            browse_view_mode: CollectionViewMode::List,
            library_view_mode: CollectionViewMode::List,
            search_query: String::new(),
            jm_username: String::new(),
            jm_password: String::new(),
            jm_profile: None,
            wnacg_username: String::new(),
            wnacg_password: String::new(),
            wnacg_profile: None,
            settings_config: AppConfig::default(),
            favorites_page: 1,
            favorites_total_pages: 1,
            weekly_categories: Vec::new(),
            weekly_types: Vec::new(),
            selected_weekly_category: None,
            selected_weekly_type: None,
            status: "输入关键词后开始搜索。".to_string(),
            loading: false,
            search_results: Vec::new(),
            search_current_page: 1,
            search_total_pages: 1,
            search_query_text: String::new(),
            selected_comic: None,
            downloads: Vec::new(),
            library: Vec::new(),
            library_sort: LibrarySort::DownloadDate,
            reader: ReaderState::default(),
            reader_fullscreen: false,
        }
    }
}

impl UiState {
    pub fn open_reader(&mut self, reader: ReaderState) {
        self.reader = reader;
        self.download_panel_tab = DownloadPanelTab::Preview;
    }

    pub fn set_browse_tab(&mut self, browse_tab: BrowseTab) {
        self.browse_tab = browse_tab;
    }

    pub fn set_browse_view_mode(&mut self, view_mode: CollectionViewMode) {
        self.browse_view_mode = view_mode;
    }

    pub fn set_library_view_mode(&mut self, view_mode: CollectionViewMode) {
        self.library_view_mode = view_mode;
    }

    pub fn close_reader_fullscreen(&mut self) {
        self.reader_fullscreen = false;
    }

    pub fn open_reader_fullscreen(&mut self) {
        self.reader_fullscreen = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ui_state_defaults_to_download_workspace_and_queue_panel() {
        let state = UiState::default();

        assert_eq!(state.workspace_tab, WorkspaceTab::Downloads);
        assert_eq!(state.download_panel_tab, DownloadPanelTab::Queue);
        assert_eq!(state.browse_tab, BrowseTab::Search);
        assert_eq!(state.browse_view_mode, CollectionViewMode::List);
        assert_eq!(state.library_view_mode, CollectionViewMode::List);
        assert!(!state.reader_fullscreen);
    }

    #[test]
    fn open_reader_enters_preview_without_forcing_fullscreen() {
        let mut state = UiState::default();

        state.open_reader(ReaderState {
            title: "demo".to_string(),
            pages: vec!["page-1".to_string()],
            current_index: 0,
            source_dir: None,
        });

        assert_eq!(state.download_panel_tab, DownloadPanelTab::Preview);
        assert_eq!(state.reader.title, "demo");
        assert!(!state.reader_fullscreen);
    }

    #[test]
    fn switching_browse_tab_updates_only_browse_layer() {
        let mut state = UiState::default();
        state.set_browse_tab(BrowseTab::Favorites);

        assert_eq!(state.workspace_tab, WorkspaceTab::Downloads);
        assert_eq!(state.browse_tab, BrowseTab::Favorites);
    }

    #[test]
    fn collection_view_modes_are_toggled_independently() {
        let mut state = UiState::default();

        state.set_browse_view_mode(CollectionViewMode::Image);
        state.set_library_view_mode(CollectionViewMode::Image);

        assert_eq!(state.browse_view_mode, CollectionViewMode::Image);
        assert_eq!(state.library_view_mode, CollectionViewMode::Image);
        assert_eq!(state.browse_tab, BrowseTab::Search);
        assert_eq!(state.workspace_tab, WorkspaceTab::Downloads);
    }

    #[test]
    fn favorites_pagination_defaults_to_first_page() {
        let state = UiState::default();
        assert_eq!(state.favorites_page, 1);
        assert_eq!(state.favorites_total_pages, 1);
    }

    #[test]
    fn settings_page_is_a_peer_workspace() {
        let state = UiState {
            workspace_tab: WorkspaceTab::Settings,
            ..Default::default()
        };
        assert_eq!(state.workspace_tab, WorkspaceTab::Settings);
    }

    #[test]
    fn download_row_state_labels_are_stable() {
        assert_eq!(DownloadRowState::Downloading.label(), "下载中");
        assert_eq!(DownloadRowState::Paused.label(), "已暂停");
        assert_eq!(DownloadRowState::Completed.label(), "已完成");
        assert_eq!(DownloadRowState::Failed.label(), "失败");
        assert_eq!(DownloadRowState::Cancelled.label(), "已取消");
    }

    #[test]
    fn download_row_can_hold_progress_numbers() {
        let row = DownloadRow {
            chapter_id: "1".to_string(),
            label: "demo".to_string(),
            comic_title: "comic".to_string(),
            chapter_title: "chapter".to_string(),
            chapter_dir: PathBuf::from("/tmp/chapter"),
            status: DownloadRowState::Downloading,
            detail: "处理中".to_string(),
            downloaded_pages: 3,
            total_pages: 10,
            current_item: "0003.png".to_string(),
        };

        assert_eq!(row.downloaded_pages, 3);
        assert_eq!(row.total_pages, 10);
        assert_eq!(row.current_item, "0003.png");
    }
}
