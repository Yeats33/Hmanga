use hmanga_core::{SearchResult, SearchSort};

/// HostApi provides the capabilities that host runtime exposes to plugins.
#[derive(Debug, Default)]
pub struct HostApi {}

impl HostApi {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn http_get(&self, _url: &str) -> Result<Vec<u8>, String> {
        // TODO: implement with reqwest
        Err("not implemented".to_string())
    }

    pub async fn http_post(&self, _url: &str, _body: &[u8]) -> Result<Vec<u8>, String> {
        Err("not implemented".to_string())
    }
}

/// Execute a native plugin's build_search + parse_search_result flow.
pub async fn execute_native_search(
    _api: &HostApi,
    _build_fn: impl FnOnce(&mut Vec<u8>) -> Result<(), String>,
    _parse_fn: impl FnOnce(&[u8]) -> Result<SearchResult, String>,
    query: &str,
    sort: SearchSort,
    _page: u32,
) -> Result<SearchResult, String> {
    // Minimal: call build to construct request bytes, then parse response.
    // In a real implementation this would invoke the plugin's native entry points.
    let _ = query;
    let _ = sort;
    let _build_fn = _build_fn;
    Err("native plugin execution not yet wired".to_string())
}
