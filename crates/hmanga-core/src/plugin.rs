use std::pin::Pin;

use crate::{
    Comic, FavoriteResult, HttpRequest, HttpResponse, ImageUrl, LogLevel, PluginMetaInfo,
    PluginResult, SearchResult, SearchSort, Session, WeeklyResult,
};
use async_trait::async_trait;

/// HostApi defines the common runtime capabilities exposed to site plugins.
pub trait HostApi: Send + Sync {
    /// Execute an HTTP request on behalf of a plugin.
    fn http_request(
        &self,
        request: HttpRequest,
    ) -> Pin<Box<dyn std::future::Future<Output = PluginResult<HttpResponse>> + Send>>;

    /// Emit a best-effort host log entry.
    fn log(&self, _level: LogLevel, _message: &str) {}
}

/// Dynamic plugin trait for unified plugin dispatch.
/// All plugins implement this trait to allow dynamic dispatch.
#[async_trait]
pub trait DynPlugin: Send + Sync {
    fn id(&self) -> &str;
    fn meta(&self) -> PluginMetaInfo;

    async fn search(
        &self,
        host: &dyn HostApi,
        query: &str,
        page: u32,
        sort: SearchSort,
    ) -> PluginResult<SearchResult>;

    async fn get_comic(&self, host: &dyn HostApi, comic_id: &str) -> PluginResult<Comic>;

    async fn get_chapter_images(
        &self,
        host: &dyn HostApi,
        chapter_id: &str,
    ) -> PluginResult<Vec<ImageUrl>>;

    async fn login(
        &self,
        host: &dyn HostApi,
        username: &str,
        password: &str,
    ) -> PluginResult<Session>;

    /// Get favorites. Pass `session` if the plugin requires authentication.
    /// Returns `NotSupported` if the plugin doesn't support favorites.
    async fn get_favorites(
        &self,
        host: &dyn HostApi,
        session: Option<&Session>,
        page: u32,
    ) -> PluginResult<FavoriteResult>;

    /// Get weekly content. Returns `NotSupported` if not supported.
    async fn get_weekly(&self, host: &dyn HostApi) -> PluginResult<WeeklyResult>;
}
