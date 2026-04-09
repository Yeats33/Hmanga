# Hmanga Design Spec

## Overview

Hmanga is an open-source, cross-platform manga downloader with a unified UI and a plugin system for multiple sites. Built entirely in Rust using Dioxus for the GUI. Monetization is through a donation-to-unlock model (honor system).

## Business Model

- **Free**: one default official plugin (JM)
- **Donated**: all official plugins + ability to install custom third-party plugins
- **Open source**: code is fully public, donation is honor-based ("防君子不防小人")

## Tech Stack

- **Language**: Rust (entire project)
- **GUI**: Dioxus (desktop target, experimental Android)
- **WASM Runtime**: wasmtime (for third-party plugins)
- **Serialization**: serde + MessagePack (WASM boundary), serde_json (config/responses)
- **HTTP**: reqwest + reqwest-middleware (retry)
- **Async**: tokio
- **Image**: image crate
- **Export**: lopdf (PDF), zip (CBZ)
- **Concurrency**: tokio::sync::Semaphore, parking_lot
- **Logging**: tracing + tracing-subscriber (file logging via tracing-appender)
- **Persistence**: serde_json flat files (config, download history, session cache)
- **Localization**: Chinese only for v1

## Architecture

### Project Structure

```
hmanga/
├── crates/
│   ├── hmanga-core/          # Core: plugin traits, data models, download engine
│   ├── hmanga-host/          # WASM host runtime (wasmtime), loads third-party plugins
│   ├── hmanga-plugin-jm/     # JM official plugin (FullPlugin)
│   └── hmanga-app/           # Dioxus desktop app (UI + glue)
├── plugin-sdk/
│   ├── hmanga-plugin-sdk/    # Plugin SDK crate (third-party devs import this)
│   └── examples/             # Example plugins
└── docs/
    └── plugin-guide/         # Plugin development documentation
```

### Data Flow

```
User action (Dioxus UI)
    ↓
hmanga-app (routes to correct plugin)
    ↓
├─ Official plugins: direct Rust function calls, zero overhead
└─ Custom plugins: via hmanga-host WASM runtime
    ↓
hmanga-core (download engine, concurrency, export)
    ↓
Filesystem / UI event feedback
```

### Key Decisions

- Official plugins are normal Rust crates, compiled into the binary, no WASM overhead
- Third-party plugins compile to `.wasm`, loaded via wasmtime with sandbox isolation
- Both implement the same trait, transparent to the UI layer
- `plugin-sdk` lives in the repo, third-party devs use git dependency:
  ```toml
  [dependencies]
  hmanga-plugin-sdk = { git = "https://github.com/Yeats33/Hmanga", path = "plugin-sdk/hmanga-plugin-sdk" }
  ```

## Plugin System

### Layered Trait Design

Two layers — plugin authors choose which to implement:

- **SimplePlugin**: request construction + response parsing only (host orchestrates the flow)
- **FullPlugin**: full control over the workflow (for complex sites like JM)

### Plugin Traits

```rust
/// Plugin metadata — a data struct (not a trait) returned by meta()
pub struct PluginMetaInfo {
    pub id: String,           // "jm", "wnacg", "copymanga"
    pub name: String,         // "禁漫天堂"
    pub version: String,      // "0.1.0"
    pub sdk_version: u32,     // SDK ABI version for compatibility check
    pub icon: Vec<u8>,        // PNG bytes
    pub description: String,
    pub capabilities: Capabilities,
}

pub struct Capabilities {
    pub search: bool,
    pub login: bool,
    pub favorites: bool,
    pub ranking: bool,
    pub weekly: bool,
    pub tags_browsing: bool,
}

/// Simple plugin — request/response only, host orchestrates
pub trait SimplePlugin: Send + Sync {
    fn meta(&self) -> PluginMetaInfo;
    fn build_search_request(&self, query: &str, page: u32, sort: SearchSort) -> HttpRequest;
    fn parse_search_response(&self, data: &[u8]) -> PluginResult<SearchResult>;
    fn build_comic_request(&self, comic_id: &str) -> HttpRequest;
    fn parse_comic_response(&self, data: &[u8]) -> PluginResult<Comic>;
    fn build_chapter_request(&self, chapter_id: &str) -> HttpRequest;
    fn parse_chapter_response(&self, data: &[u8]) -> PluginResult<Vec<ImageUrl>>;
    fn process_image(&self, data: Vec<u8>, ctx: &ImageContext) -> PluginResult<Vec<u8>> { Ok(data) }
    fn build_login_request(&self, username: &str, password: &str) -> Option<HttpRequest> { None }
    fn parse_login_response(&self, data: &[u8]) -> PluginResult<Session> { Err(PluginError::NotSupported) }
    fn build_favorites_request(&self, session: &Session, page: u32) -> Option<HttpRequest> { None }
    fn parse_favorites_response(&self, data: &[u8]) -> PluginResult<FavoriteResult> { Err(PluginError::NotSupported) }
}

/// Full plugin — controls the entire workflow
/// Note: native plugins use async fn directly; WASM plugins are synchronous
/// internally and wrapped in async by the host (see WASM ABI section).
pub trait FullPlugin: Send + Sync {
    fn meta(&self) -> PluginMetaInfo;
    async fn search(&self, host: &dyn HostApi, query: &str, page: u32, sort: SearchSort) -> PluginResult<SearchResult>;
    async fn get_comic(&self, host: &dyn HostApi, comic_id: &str) -> PluginResult<Comic>;
    async fn get_chapter_images(&self, host: &dyn HostApi, chapter_id: &str) -> PluginResult<Vec<ImageUrl>>;
    async fn process_image(&self, host: &dyn HostApi, data: Vec<u8>, ctx: &ImageContext) -> PluginResult<Vec<u8>>;
    async fn login(&self, host: &dyn HostApi, username: &str, password: &str) -> PluginResult<Session> { Err(PluginError::NotSupported) }
    async fn get_favorites(&self, host: &dyn HostApi, session: &Session, page: u32) -> PluginResult<FavoriteResult> { Err(PluginError::NotSupported) }
    async fn get_weekly(&self, host: &dyn HostApi) -> PluginResult<WeeklyResult> { Err(PluginError::NotSupported) }
    async fn get_ranking(&self, host: &dyn HostApi, page: u32) -> PluginResult<SearchResult> { Err(PluginError::NotSupported) }
}

/// Host API provided to plugins
pub trait HostApi: Send + Sync {
    async fn http_request(&self, req: HttpRequest) -> PluginResult<HttpResponse>;
    fn log(&self, level: LogLevel, msg: &str);
    fn get_config(&self, key: &str) -> Option<String>;
    fn set_config(&self, key: &str, value: &str);
}
```

### Error Types

```rust
/// Plugin-level error type — used across the WASM boundary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PluginError {
    /// Feature not implemented by this plugin
    NotSupported,
    /// Network request failed
    Network(String),
    /// Response parsing failed
    Parse(String),
    /// Authentication required or failed
    Auth(String),
    /// Generic error with message
    Other(String),
}

pub type PluginResult<T> = std::result::Result<T, PluginError>;

/// App-level error type — wraps plugin errors with source context
#[derive(Debug, thiserror::Error)]
pub enum HmangaError {
    #[error("[{source}] plugin error: {inner}")]
    Plugin { source: String, inner: PluginError },
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
```

Error handling strategy:
- Plugin errors are returned as `PluginResult<T>`, serializable across WASM boundary
- WASM traps (panics, infinite loops) are caught by wasmtime and converted to `HmangaError::WasmRuntime` — the app never crashes from a bad plugin
- UI displays errors as toast notifications; network errors offer a retry button
- Download failures mark the task as `Failed` with the error message preserved

### Unified Data Models

```rust
pub struct Comic {
    pub id: String,
    pub source: String,             // plugin id that produced this
    pub title: String,
    pub author: String,
    pub cover_url: String,
    pub description: String,
    pub tags: Vec<String>,
    pub chapters: Vec<ChapterInfo>,
    pub extra: HashMap<String, String>,  // site-specific fields
}

pub struct ChapterInfo {
    pub id: String,
    pub title: String,
    pub page_count: Option<u32>,
}

pub struct SearchResult {
    pub comics: Vec<Comic>,
    pub current_page: u32,
    pub total_pages: u32,
}

pub struct ImageUrl {
    pub url: String,
    pub headers: HashMap<String, String>,  // referer, auth, etc.
    pub index: u32,
}

/// Context passed to process_image for decryption/descrambling
pub struct ImageContext {
    pub comic_id: String,
    pub chapter_id: String,
    pub page_index: u32,
    pub extra: HashMap<String, String>,  // plugin-specific (e.g. scramble_id for JM)
}

pub struct Session {
    pub token: String,
    pub username: String,
    pub extra: HashMap<String, String>,
}

pub struct FavoriteResult {
    pub comics: Vec<Comic>,
    pub current_page: u32,
    pub total_pages: u32,
    pub folder_name: Option<String>,
}

pub struct WeeklyResult {
    pub title: String,
    pub comics: Vec<Comic>,
}

pub struct HttpRequest {
    pub url: String,
    pub method: HttpMethod,
    pub headers: HashMap<String, String>,
    pub body: Option<Vec<u8>>,
}

pub struct HttpResponse {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

pub enum HttpMethod { Get, Post, Put, Delete }

pub enum SearchSort { Latest, Popular, Relevance }

pub enum DownloadFormat { Raw, Cbz, Pdf }

pub enum LogLevel { Debug, Info, Warn, Error }

pub type TaskId = u64;
pub type SiteId = String;  // same as plugin id
```

### PluginRegistry — Unified Dispatch

```rust
pub struct PluginRegistry {
    plugins: HashMap<String, Box<dyn PluginAdapter>>,
}

/// Adapts both native and WASM plugins to a uniform interface
pub trait PluginAdapter: Send + Sync {
    fn meta(&self) -> PluginMetaInfo;
    async fn search(&self, query: &str, page: u32, sort: SearchSort) -> PluginResult<SearchResult>;
    async fn get_comic(&self, comic_id: &str) -> PluginResult<Comic>;
    async fn get_chapter_images(&self, chapter_id: &str) -> PluginResult<Vec<ImageUrl>>;
    async fn process_image(&self, data: Vec<u8>, ctx: &ImageContext) -> PluginResult<Vec<u8>>;
    async fn login(&self, username: &str, password: &str) -> PluginResult<Session>;
    async fn get_favorites(&self, session: &Session, page: u32) -> PluginResult<FavoriteResult>;
    async fn get_weekly(&self) -> PluginResult<WeeklyResult>;
    async fn get_ranking(&self, page: u32) -> PluginResult<SearchResult>;
}

/// Wraps a SimplePlugin into a PluginAdapter by orchestrating the request/response cycle.
/// Works for both native SimplePlugins and WASM SimplePlugins.
pub struct SimplePluginAdapter<P: SimplePlugin> {
    plugin: P,
    http_client: reqwest::Client,
}

impl<P: SimplePlugin> PluginAdapter for SimplePluginAdapter<P> {
    async fn search(&self, query: &str, page: u32, sort: SearchSort) -> PluginResult<SearchResult> {
        let req = self.plugin.build_search_request(query, page, sort);
        let resp = self.execute_http(req).await?;
        self.plugin.parse_search_response(&resp.body)
    }
    // ... same pattern for all methods: build_request → execute → parse_response
}
```

### WASM ABI and Plugin Runtime

**Core problem**: WASM modules are synchronous. Plugin traits use async. The host bridges this gap.

**Architecture**:

```
WASM Guest (plugin .wasm)              Host (hmanga-host)
──────────────────────                  ────────────────────
Exports synchronous functions:          Calls guest functions from async context:
  hm_meta() -> ptr                        spawn_blocking(|| instance.call("hm_search", ...))
  hm_search(ptr, len) -> ptr              ↓
  hm_get_comic(ptr, len) -> ptr           deserialize result
  hm_get_chapter_images(ptr, len) -> ptr  return to caller
  hm_process_image(ptr, len) -> ptr
  hm_login(ptr, len) -> ptr
  ...

Imports (host functions):               Provides these to the guest:
  host_http_request(ptr, len) -> ptr      blocks the WASM thread, runs HTTP async on tokio
  host_log(level, ptr, len)               logs via tracing
  host_get_config(ptr, len) -> ptr        reads plugin config
  host_set_config(ptr, len, ptr, len)     writes plugin config
  host_alloc(len) -> ptr                  guest memory allocation helper
```

**Data flow across WASM boundary**:
1. Host serializes input to MessagePack bytes
2. Host writes bytes into guest memory (via `host_alloc`)
3. Host calls guest export with (ptr, len)
4. Guest deserializes, processes, serializes result to MessagePack
5. Guest returns (ptr, len) pointing to result in guest memory
6. Host reads and deserializes the result

**Async bridging for host functions**:
- When the WASM guest calls `host_http_request`, the host function blocks the current thread
- The host function internally does `tokio::runtime::Handle::current().block_on(async_http_request)`
- Each WASM plugin runs on its own `spawn_blocking` thread, so blocking is safe
- This means WASM plugins are effectively single-threaded per call (which is fine — concurrency is managed by the host's download engine)

**Plugin type detection**:
- At load time, the host checks which exports exist
- If `hm_build_search_request` + `hm_parse_search_response` exist → SimplePlugin
- If `hm_search` exists → FullPlugin
- Wrapped in `SimplePluginAdapter` or `FullPluginAdapter` accordingly

**The `#[hmanga_plugin]` proc macro** generates:
- `hm_meta()` export that returns serialized `PluginMetaInfo`
- For SimplePlugin: `hm_build_*_request` / `hm_parse_*_response` exports with serialization glue
- For FullPlugin: `hm_search`, `hm_get_comic`, etc. exports with serialization glue
- `host_*` import bindings that the plugin calls as normal Rust functions
- Guest-side allocator (`hm_alloc`, `hm_dealloc`) for memory management

**Plugin SDK usage** (what third-party devs write):

```rust
use hmanga_plugin_sdk::prelude::*;

#[hmanga_plugin]
pub struct MySitePlugin;

impl SimplePlugin for MySitePlugin {
    fn meta(&self) -> PluginMetaInfo { ... }
    fn build_search_request(&self, query: &str, page: u32, sort: SearchSort) -> HttpRequest { ... }
    fn parse_search_response(&self, data: &[u8]) -> PluginResult<SearchResult> { ... }
    // ...
}
```

Compile with: `cargo build --target wasm32-wasip1 --release`
Output: `target/wasm32-wasip1/release/my_site_plugin.wasm`
Install: copy to `~/.hmanga/plugins/`

### Plugin Compatibility

- `PluginMetaInfo.sdk_version` is a monotonically increasing integer (starting at 1)
- At load time, the host checks `sdk_version` against its supported range
- If the plugin's `sdk_version` is too new (host doesn't support it), the plugin is rejected with a clear error message prompting the user to update the app
- Breaking ABI changes increment `sdk_version`; additive changes (new optional exports) do not

### Plugin Loading Flow

```
App startup
    ↓
1. Register all official plugins (compiled-in, direct Rust)
    ↓
2. Check is_donated
    ├─ false: activate only default free plugin (JM), others show 🔒
    └─ true:
        ├─ Activate all official plugins
        └─ Scan ~/.hmanga/plugins/*.wasm
            ├─ For each .wasm: wasmtime load → validate exports → read PluginMeta
            ├─ Pass: register to PluginRegistry
            └─ Fail: log warning, skip
    ↓
3. PluginRegistry ready, UI renders sidebar based on registered plugins
```

## UI Layout

```
┌──────────────────────────────────────────────────┐
│  ┌──┐  ┌──────────────────────────────────────┐  │
│  │🌐│  │  [搜索] [收藏夹] [下载中] [已下载] [阅读器]│  │
│  ├──┤  │                                       │  │
│  │JM│  │         Content Area                  │  │
│  ├──┤  │                                       │  │
│  │..│  │                                       │  │
│  ├──┤  │                                       │  │
│  │⚙ │  │                                       │  │
│  └──┘  └──────────────────────────────────────┘  │
│ Sidebar  Main content                             │
└──────────────────────────────────────────────────┘
```

Sidebar icons (top to bottom):
- 🌐 Aggregate view (all sites)
- Site-specific icons (JM, wnacg, copymanga, ...)
- ⚙ Settings

### Dioxus Component Tree

```
App
├── Sidebar
│   ├── SiteIcon (Aggregate)
│   ├── SiteIcon (per plugin)
│   └── SettingsIcon
├── AggregateView
│   ├── SubTabs
│   ├── SearchPane          # Cross-site search, results tagged with source
│   ├── FavoritesPane       # Merged favorites from all sites
│   ├── DownloadingPane     # Unified download queue
│   ├── DownloadedPane
│   └── ReaderPane
├── SiteView
│   ├── SubTabs (driven by plugin Capabilities)
│   ├── SearchPane          # Reused component, fixed source filter
│   ├── FavoritesPane
│   ├── WeeklyPane          # Site-specific (JM)
│   ├── RankingPane         # Site-specific
│   └── TagsPane            # Site-specific
└── SettingsView
    ├── GeneralSettings
    ├── DonateUnlock
    └── PluginManager
```

### Aggregate Search Strategy

When the user searches from the Aggregate view:
1. The host dispatches `search()` to all active plugins **in parallel** (via `tokio::JoinSet`)
2. Each plugin has a **5 second timeout**; if a plugin fails or times out, its results are skipped and a warning icon is shown
3. Results are **grouped by source**, displayed in sections: "JM (12 results)", "wnacg (8 results)", etc.
4. Within each source section, results are ordered by the plugin's native ranking
5. No cross-site deduplication in v1 (same manga on different sites appears separately)
6. Pagination: each source has its own page counter, a "load more" button per source section

### Key UI Decisions

- `SearchPane`, `FavoritesPane`, etc. are generic components accepting `source: Option<String>`. `None` = aggregate all, `Some("jm")` = filter to JM only
- Site-specific panes only rendered when plugin declares the corresponding Capability
- Reader is global — same ReaderPane regardless of source
- Download queue is global, managed by hmanga-core's DownloadManager

## Download Engine

### DownloadManager

```rust
pub struct DownloadManager {
    tasks: Arc<RwLock<HashMap<TaskId, DownloadTask>>>,
    chapter_sem: Arc<Semaphore>,
    image_sem: Arc<Semaphore>,
    speed_tracker: Arc<SpeedTracker>,
    event_tx: broadcast::Sender<DownloadEvent>,
}

pub struct DownloadTask {
    pub id: TaskId,
    pub source: String,
    pub comic: Comic,
    pub chapters: Vec<ChapterTask>,
    pub state: DownloadTaskState,
    pub output_dir: PathBuf,
    pub format: DownloadFormat,
}

pub enum DownloadTaskState {
    Pending,
    Downloading { progress: f32 },
    Paused,
    Completed,
    Failed { error: String },
}

pub enum DownloadEvent {
    TaskCreated(TaskId),
    Progress { task_id: TaskId, chapter_id: String, downloaded: u32, total: u32 },
    SpeedUpdate(u64),
    TaskCompleted(TaskId),
    TaskFailed { task_id: TaskId, error: String },
    ExportProgress { task_id: TaskId, format: DownloadFormat, progress: f32 },
}
```

### Download Flow

```
User clicks download
    ↓
DownloadManager creates DownloadTask
    ↓
Per chapter (chapter_sem concurrency):
    ├─ plugin.get_chapter_images()
    ├─ Per image (image_sem concurrency):
    │   ├─ HostApi.http_request() to download
    │   ├─ plugin.process_image() to decrypt/reassemble
    │   └─ Save to disk
    └─ Chapter done, emit Progress event
    ↓
All done → optional export CBZ/PDF
    ↓
UI updates via event_rx subscription
```

### State Management (Dioxus)

```rust
pub struct AppState {
    pub active_site: Signal<SiteId>,
    pub plugins: Signal<Vec<PluginInfo>>,
    pub search_results: Signal<HashMap<String, SearchResult>>,
    pub favorites: Signal<HashMap<String, FavoriteResult>>,
    pub download_tasks: Signal<HashMap<TaskId, DownloadTask>>,
    pub download_speed: Signal<u64>,
    pub sessions: Signal<HashMap<String, Session>>,
    pub config: Signal<AppConfig>,
    pub is_donated: Signal<bool>,
}
```

Event bridge: download engine emits events via `broadcast::Sender`, Dioxus subscribes in `use_coroutine`, updates Signals for automatic re-render.

### Reader

- **Scroll mode** (default): continuous vertical scroll, pages stacked top to bottom
- **Page mode**: left/right swipe or arrow keys, one page at a time
- Zoom: pinch-to-zoom on mobile, Ctrl+scroll on desktop
- Keyboard: arrow keys for navigation, F for fullscreen
- Reading progress: stored locally per comic/chapter in `~/.hmanga/reading_progress.json`
- Preloading: prefetch next 3 pages ahead of current position

## Persistence

All data stored as flat JSON files under `~/.hmanga/`:

```
~/.hmanga/
├── config.json              # App config (download dir, concurrency, proxy, donate code, theme)
├── sessions.json            # Login sessions per site { "jm": { token, username, ... } }
├── download_history.json    # Completed + in-progress downloads (resumable on restart)
├── reading_progress.json    # { "jm:comic_id:chapter_id": { page: 5, timestamp: ... } }
├── favorites_cache/         # Cached favorites per site (refreshed on app start)
│   └── jm.json
├── plugins/                 # Third-party WASM plugins
│   └── my-site.wasm
└── logs/                    # Log files (rotated daily)
    └── hmanga.2026-04-09.log
```

**Download resume**: On startup, `DownloadManager` loads `download_history.json`, finds tasks with state `Downloading`/`Pending`, checks which images are already on disk, and resumes from where it left off.

**No SQLite in v1**: flat files are sufficient for the data volume. Can migrate to SQLite later if needed.

## Plugin Security

WASM sandbox boundaries for third-party plugins:

**Allowed** (via host functions only):
- HTTP requests (via `host_http_request`) — the host executes all network calls
- Read/write plugin-specific config keys (via `host_get_config`/`host_set_config`)
- Logging (via `host_log`)

**Not allowed**:
- Direct filesystem access
- Direct network access (no WASI networking)
- Access to other plugins' data
- Access to app config, sessions, or donate status

**Resource limits**:
- Memory: 256 MB max per plugin instance (wasmtime linear memory limit)
- Execution time: 30 second timeout per function call (covers infinite loops)
- No thread spawning inside WASM (single-threaded guest)

**Failure isolation**:
- WASM traps (panic, OOM, timeout) are caught by wasmtime
- Converted to `HmangaError::WasmRuntime`, surfaced as UI error toast
- Failed plugin is marked as errored in the sidebar (red dot), user can retry or disable
- App continues running normally — a broken plugin never crashes the host

## Donation Unlock

### Mechanism

- Donor pays via any channel (WeChat/Alipay/Aifadian)
- Receives a donation code (UUID-like, generated offline)
- Enters code in Settings → stored in `~/.hmanga/config.json`
- Local format validation only, no online verification, no device binding, no expiry

### Verification

Intentionally minimal — any well-formatted code is accepted. The `generate_code` function is a separate CLI tool for record-keeping only (tracking which codes were issued to whom). The app does not verify that a code was actually generated by us.

```rust
/// Used by the maintainer's CLI tool to generate codes for donors.
/// NOT shipped in the app binary — lives in a separate `tools/` crate.
pub fn generate_code(user_id: &str) -> String {
    let hash = md5::compute(format!("hmanga-{user_id}"));
    format!("HM-{:X}", hash)
}

/// App-side: accepts any well-formatted code. Honor system.
pub fn verify_code(code: &str) -> bool {
    code.starts_with("HM-")
        && code.len() == 35
        && code[3..].chars().all(|c| c.is_ascii_hexdigit())
}
```

### Config

```json
{
  "donate_code": "HM-A1B2C3D4E5F6...",
  "download_dir": "/Users/xxx/Comics",
  "chapter_concurrency": 3,
  "image_concurrency": 5,
  "proxy": null,
  "enabled_plugins": ["jm"],
  "theme": "auto"
}
```

## CI/CD

### Release Workflow (triggered by tag `v*`)

**build-desktop** (matrix):
| Runner | Target |
|--------|--------|
| windows-latest | x86_64-pc-windows-msvc |
| macos-latest | aarch64-apple-darwin |
| ubuntu-latest | x86_64-unknown-linux-gnu |

Steps: checkout → Rust toolchain → Dioxus CLI → `dx build --release --platform desktop` → platform packaging (Win: .msi+.exe, Mac: .dmg, Linux: .deb+.AppImage) → upload artifact

**build-android**:
- Runner: ubuntu-latest
- Steps: checkout → Rust + Android NDK → Java 17 → cargo ndk (aarch64-linux-android, armv7-linux-androideabi) → `dx build --release --platform android` → sign .apk → upload artifact
- Note: Android is v1.1 target (Dioxus Android is experimental)

**release**:
- Needs: build-desktop, build-android
- Create GitHub Release, upload all artifacts, generate changelog (git-cliff)

### PR Checks

```
cargo fmt --check
cargo clippy -- -D warnings
cargo test --workspace
dx build --platform desktop
```

### Branch Strategy

```
main ──── stable, tags trigger release
  └─ dev ──── daily development
      └─ feat/*
      └─ fix/*
```

## JM Plugin — Image Descrambling

JM (禁漫天堂) scrambles images as an anti-scraping measure. The `process_image` method in the JM plugin must reverse this.

**Algorithm** (derived from the reference jmcomic-downloader):
1. Each image is divided into a grid of tiles (e.g., 10 rows)
2. The tile order is scrambled based on a key derived from `scramble_id` (fetched from the `/chapter_view_template` API)
3. The `scramble_id` determines the number of segments and the rearrangement mapping
4. The JM plugin's `process_image`:
   - Receives raw JPEG bytes + `ImageContext` containing `chapter_id` and `extra["scramble_id"]`
   - Decodes the image
   - Reverses the tile rearrangement based on the scramble algorithm
   - Re-encodes to PNG/JPEG
   - Returns the corrected image bytes

**Additionally**, JM's API responses are AES-256 encrypted. The JM plugin handles decryption internally (using keys derived from the request timestamp, as in the reference implementation). This is why JM must be a FullPlugin — the multi-step encryption/decryption flow cannot be expressed as simple request/response pairs.

## Platform Notes

- Desktop (Win/Mac/Linux): v1 target, fully supported
- Android: v1.1 target, experimental Dioxus support
- WASM plugins work on all platforms (wasmtime supports Android)
- Mobile UI: sidebar collapses to bottom navigation bar (responsive)

## Risks and Mitigations

| Risk | Severity | Mitigation |
|------|----------|------------|
| Dioxus Android is experimental | Medium | Android is v1.1, not v1. Desktop first. If Dioxus Android stalls, evaluate alternatives (e.g., separate mobile app with shared core). |
| wasmtime on Android is not tier-1 | Medium | Android v1.1 launches with official plugins only (compiled-in). WASM plugin support on Android deferred to v1.2 if wasmtime proves unstable. |
| Dioxus component ecosystem is immature | Medium | Build custom components as needed. Keep components simple and reusable. The reference projects' UIs are not overly complex. |
| JM may change their API/encryption | Low | Normal maintenance. The reference jmcomic-downloader has handled this for multiple versions. Plugin system makes updates easy — just update the JM plugin. |
| WASM plugin development friction | Low | Provide good examples, a template repo, and clear docs. The SimplePlugin layer minimizes boilerplate for easy sites. |

## Reference Projects

- [Yeats33/jmcomic-downloader](https://github.com/Yeats33/jmcomic-downloader) — Primary reference for JM site logic
- [lanyeeee/wnacg-downloader](https://github.com/lanyeeee/wnacg-downloader) — Reference for architecture patterns
- [lanyeeee/copymanga-downloader](https://github.com/lanyeeee/copymanga-downloader) — Reference for architecture patterns

All three use Tauri 2 + Vue 3 + Naive UI + Pinia. Hmanga replaces the frontend with Dioxus (pure Rust) and adds a plugin system.
