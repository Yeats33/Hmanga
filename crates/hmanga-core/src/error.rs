use thiserror::Error;

use crate::models::PluginError;

pub type PluginResult<T> = std::result::Result<T, PluginError>;

#[derive(Debug, Error)]
pub enum HmangaError {
    #[error("[{plugin_id}] plugin error: {inner}")]
    Plugin {
        plugin_id: String,
        inner: PluginError,
    },
    #[error("download error: {0}")]
    Download(String),
    #[error("export error: {0}")]
    Export(String),
    #[error("config error: {0}")]
    Config(String),
    #[error("WASM runtime error: {0}")]
    WasmRuntime(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}
