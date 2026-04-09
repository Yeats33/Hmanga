use crate::{HttpRequest, HttpResponse, LogLevel, PluginResult};

/// HostApi defines the common runtime capabilities exposed to site plugins.
pub trait HostApi: Send + Sync {
    /// Execute an HTTP request on behalf of a plugin.
    fn http_request(
        &self,
        request: HttpRequest,
    ) -> impl std::future::Future<Output = PluginResult<HttpResponse>> + Send;

    /// Emit a best-effort host log entry.
    fn log(&self, _level: LogLevel, _message: &str) {}
}
