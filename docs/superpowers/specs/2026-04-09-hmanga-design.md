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
/// All plugins must implement base metadata
pub trait PluginMeta {
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    fn version(&self) -> &str;
    fn icon(&self) -> &[u8];
    fn description(&self) -> &str;
    fn capabilities(&self) -> Capabilities;
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
pub trait SimplePlugin: PluginMeta {
    fn build_search_request(&self, query: &str, page: u32, sort: SearchSort) -> HttpRequest;
    fn parse_search_response(&self, data: &[u8]) -> Result<SearchResult>;
    fn build_comic_request(&self, comic_id: &str) -> HttpRequest;
    fn parse_comic_response(&self, data: &[u8]) -> Result<Comic>;
    fn build_chapter_request(&self, chapter_id: &str) -> HttpRequest;
    fn parse_chapter_response(&self, data: &[u8]) -> Result<Vec<ImageUrl>>;
    fn process_image(&self, data: Vec<u8>, ctx: &ImageContext) -> Result<Vec<u8>> { Ok(data) }
    fn build_login_request(&self, username: &str, password: &str) -> Option<HttpRequest> { None }
    fn parse_login_response(&self, data: &[u8]) -> Result<Session> { unimplemented!() }
    fn build_favorites_request(&self, session: &Session, page: u32) -> Option<HttpRequest> { None }
    fn parse_favorites_response(&self, data: &[u8]) -> Result<FavoriteResult> { unimplemented!() }
}

/// Full plugin — controls the entire workflow
pub trait FullPlugin: PluginMeta {
    async fn search(&self, host: &dyn HostApi, query: &str, page: u32, sort: SearchSort) -> Result<SearchResult>;
    async fn get_comic(&self, host: &dyn HostApi, comic_id: &str) -> Result<Comic>;
    async fn get_chapter_images(&self, host: &dyn HostApi, chapter_id: &str) -> Result<Vec<ImageUrl>>;
    async fn process_image(&self, host: &dyn HostApi, data: Vec<u8>, ctx: &ImageContext) -> Result<Vec<u8>>;
    async fn login(&self, host: &dyn HostApi, username: &str, password: &str) -> Result<Session> { unimplemented!() }
    async fn get_favorites(&self, host: &dyn HostApi, session: &Session, page: u32) -> Result<FavoriteResult> { unimplemented!() }
    async fn get_weekly(&self, host: &dyn HostApi) -> Result<WeeklyResult> { unimplemented!() }
    async fn get_ranking(&self, host: &dyn HostApi, page: u32) -> Result<SearchResult> { unimplemented!() }
}

/// Host API provided to plugins
pub trait HostApi {
    async fn http_request(&self, req: HttpRequest) -> Result<HttpResponse>;
    async fn decode_image(&self, data: &[u8], format: ImageFormat) -> Result<RgbImage>;
    fn log(&self, level: LogLevel, msg: &str);
    fn get_config(&self, key: &str) -> Option<String>;
    fn set_config(&self, key: &str, value: &str);
}
```

### Unified Data Models

```rust
pub struct Comic {
    pub id: String,
    pub source: String,
    pub title: String,
    pub author: String,
    pub cover_url: String,
    pub description: String,
    pub tags: Vec<String>,
    pub chapters: Vec<ChapterInfo>,
    pub extra: HashMap<String, String>,
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
    pub headers: HashMap<String, String>,
    pub index: u32,
}

pub struct Session {
    pub token: String,
    pub username: String,
    pub extra: HashMap<String, String>,
}
```

### PluginRegistry — Unified Dispatch

```rust
pub struct PluginRegistry {
    plugins: HashMap<String, Box<dyn PluginAdapter>>,
}

/// Adapts both native and WASM plugins to a uniform interface
pub trait PluginAdapter: Send + Sync {
    fn meta(&self) -> &PluginMeta;
    async fn search(&self, query: &str, page: u32, sort: SearchSort) -> Result<SearchResult>;
    async fn get_comic(&self, comic_id: &str) -> Result<Comic>;
    async fn get_chapter_images(&self, chapter_id: &str) -> Result<Vec<ImageUrl>>;
    async fn process_image(&self, data: Vec<u8>, ctx: &ImageContext) -> Result<Vec<u8>>;
    async fn login(&self, username: &str, password: &str) -> Result<Session>;
    async fn get_favorites(&self, session: &Session, page: u32) -> Result<FavoriteResult>;
}
```

### WASM Plugin Runtime

- Engine: wasmtime
- Data serialization across WASM boundary: MessagePack
- Plugin SDK provides `#[hmanga_plugin]` proc macro for automatic WASM export generation
- Host functions exposed: `host_http_request`, `host_decode_image`, `host_log`, `host_get_config`, `host_set_config`

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

## Donation Unlock

### Mechanism

- Donor pays via any channel (WeChat/Alipay/Aifadian)
- Receives a donation code (UUID-like, generated offline)
- Enters code in Settings → stored in `~/.hmanga/config.json`
- Local format validation only, no online verification, no device binding, no expiry

### Verification

```rust
const SALT: &str = "hmanga-donate-2024";

pub fn generate_code(user_id: &str) -> String {
    let hash = md5::compute(format!("{SALT}-{user_id}"));
    format!("HM-{:X}", hash)
}

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

## Platform Notes

- Desktop (Win/Mac/Linux): v1 target, fully supported
- Android: v1.1 target, experimental Dioxus support
- WASM plugins work on all platforms (wasmtime supports Android)
- Mobile UI: sidebar collapses to bottom navigation bar (responsive)

## Reference Projects

- [Yeats33/jmcomic-downloader](https://github.com/Yeats33/jmcomic-downloader) — Primary reference for JM site logic
- [lanyeeee/wnacg-downloader](https://github.com/lanyeeee/wnacg-downloader) — Reference for architecture patterns
- [lanyeeee/copymanga-downloader](https://github.com/lanyeeee/copymanga-downloader) — Reference for architecture patterns

All three use Tauri 2 + Vue 3 + Naive UI + Pinia. Hmanga replaces the frontend with Dioxus (pure Rust) and adds a plugin system.
