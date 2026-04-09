# Hmanga V1 Bootstrap Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build the first runnable Hmanga desktop-only v1 baseline from the approved design spec: Rust workspace, plugin abstraction, WASM host bridge, download engine, Dioxus shell, JM official plugin, and CI.

**Architecture:** Start with a Rust workspace that isolates core domain logic, WASM hosting, official plugin code, and the Dioxus shell. De-risk the build by delivering vertical slices in this order: typed domain models, adapter boundary, resumable download path, then UI wiring. Keep v1 desktop-only and put every feature behind compile-tested, serialization-tested, or integration-tested seams.

**Tech Stack:** Rust workspace, Dioxus desktop, Tokio, Reqwest, Wasmtime, Serde/MessagePack, Tracing, Image, Zip, Lopdf, GitHub Actions.

---

## Spec Validation Notes

The current spec is implementable. Lock these decisions before writing code:

1. **Confirmed scope: v1 is desktop only.**
   Android is explicitly out of scope for the first release. Do not wire Android into the first CI/release workflow or milestone checklist.
2. **Split plugin discovery from plugin activation.**
   Add an `OfficialPluginCatalog` for visible-but-locked official plugins; `PluginRegistry` holds only activated adapters.
3. **Donation unlock is honor-system confirmation only.**
   Do not implement donation code generation or validation. Settings only needs a local "I already donated" confirmation that flips `donation_unlocked=true`.
4. **Use guest allocators only.**
   Ignore the earlier `host_alloc` mention in the spec. Standardize on guest exports `hm_alloc` / `hm_dealloc`.
5. **Add missing domain types early.**
   Define `ChapterTask`, `PluginInfo`, and persistence DTOs in `hmanga-core` before host/app work.
6. **Split the SDK into two crates.**
   `plugin-sdk/hmanga-plugin-sdk` stays a normal library crate; `plugin-sdk/hmanga-plugin-macro` is a proc-macro crate re-exported by the SDK.

## Implementation Notes

- Follow `@coding-standards` and `@verification-before-completion`.
- Use the repo Lore commit protocol for every commit.
- Keep the first runnable milestone narrow: one official plugin (`jm`), desktop only, Chinese UI only.
- Prefer unit tests in `hmanga-core` and `hmanga-host`, plus one smoke integration test for app bootstrap.

### Task 1: Bootstrap the Rust workspace and CI baseline

**Files:**
- Create: `Cargo.toml`
- Create: `.cargo/config.toml`
- Create: `rust-toolchain.toml`
- Create: `.gitignore`
- Create: `.github/workflows/ci.yml`
- Create: `crates/hmanga-core/Cargo.toml`
- Create: `crates/hmanga-core/src/lib.rs`
- Create: `crates/hmanga-host/Cargo.toml`
- Create: `crates/hmanga-host/src/lib.rs`
- Create: `crates/hmanga-plugin-jm/Cargo.toml`
- Create: `crates/hmanga-plugin-jm/src/lib.rs`
- Create: `crates/hmanga-app/Cargo.toml`
- Create: `crates/hmanga-app/src/main.rs`
- Create: `plugin-sdk/hmanga-plugin-sdk/Cargo.toml`
- Create: `plugin-sdk/hmanga-plugin-sdk/src/lib.rs`
- Create: `plugin-sdk/hmanga-plugin-macro/Cargo.toml`
- Create: `plugin-sdk/hmanga-plugin-macro/src/lib.rs`

**Step 1: Create the workspace manifests and package skeleton**

Create a workspace with members matching the spec, shared dependency versions, and a `desktop` cargo alias for the app crate.

**Step 2: Run the empty workspace check**

Run: `cargo check --workspace`
Expected: FAIL because crate entry files and module exports are still stubs or missing.

**Step 3: Add minimal compile stubs for every crate**

Add placeholder exports:

```rust
// crates/hmanga-core/src/lib.rs
pub mod error;
pub mod models;
```

```rust
// crates/hmanga-app/src/main.rs
fn main() {
    println!("hmanga bootstrap");
}
```

**Step 4: Wire desktop-only CI**

Create `.github/workflows/ci.yml` with:

```yaml
name: CI
on:
  pull_request:
  push:
    branches: [main, dev]
jobs:
  check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo fmt --check
      - run: cargo clippy --workspace --all-targets -- -D warnings
      - run: cargo test --workspace
      - run: cargo check -p hmanga-app
```

**Step 5: Re-run compile validation**

Run: `cargo check --workspace`
Expected: PASS

**Step 6: Commit**

Commit with a Lore message focused on creating the workspace and explicitly recording that Android is deferred out of v1.

### Task 2: Define shared domain models, errors, and persistence DTOs in `hmanga-core`

**Files:**
- Create: `crates/hmanga-core/src/error.rs`
- Create: `crates/hmanga-core/src/models.rs`
- Create: `crates/hmanga-core/src/persistence.rs`
- Create: `crates/hmanga-core/src/config.rs`
- Create: `crates/hmanga-core/tests/model_roundtrip.rs`

**Step 1: Write failing serialization and default-config tests**

Create `crates/hmanga-core/tests/model_roundtrip.rs` with coverage for:

```rust
#[test]
fn plugin_error_roundtrips_via_rmp_serde() {}

#[test]
fn app_config_default_enables_only_jm() {}

#[test]
fn app_config_defaults_to_donation_locked() {}

#[test]
fn download_history_roundtrips_with_pending_task() {}
```

**Step 2: Run the targeted tests**

Run: `cargo test -p hmanga-core --test model_roundtrip`
Expected: FAIL with missing modules/types such as `AppConfig`, `PluginError`, `DownloadTask`, `ChapterTask`, or `PluginInfo`.

**Step 3: Implement the core types**

Include the spec models plus the missing types:

```rust
pub struct ChapterTask {
    pub chapter: ChapterInfo,
    pub downloaded_pages: u32,
    pub total_pages: Option<u32>,
    pub output_dir: PathBuf,
}

pub struct PluginInfo {
    pub meta: PluginMetaInfo,
    pub enabled: bool,
    pub locked: bool,
    pub health: PluginHealth,
}
```

Also define:
- `PluginError`, `HmangaError`
- `Comic`, `ChapterInfo`, `SearchResult`, `ImageUrl`, `Session`
- `DownloadTask`, `DownloadTaskState`, `DownloadEvent`
- `AppConfig`, `ConfigVersioned<T>`
- persistence DTOs for `download_history.json`, `sessions.json`, `reading_progress.json`

**Step 4: Re-run the targeted tests**

Run: `cargo test -p hmanga-core --test model_roundtrip`
Expected: PASS

**Step 5: Run crate-wide verification**

Run: `cargo test -p hmanga-core`
Expected: PASS

**Step 6: Commit**

Commit with a Lore message noting that the missing spec types were formalized in the core crate to unblock downstream adapters and UI state.

### Task 3: Build the plugin SDK contract and proc-macro split

**Files:**
- Modify: `plugin-sdk/hmanga-plugin-sdk/src/lib.rs`
- Create: `plugin-sdk/hmanga-plugin-sdk/src/abi.rs`
- Create: `plugin-sdk/hmanga-plugin-sdk/src/prelude.rs`
- Modify: `plugin-sdk/hmanga-plugin-macro/src/lib.rs`
- Create: `plugin-sdk/hmanga-plugin-sdk/tests/sdk_contract.rs`

**Step 1: Write a failing SDK contract test**

Create `plugin-sdk/hmanga-plugin-sdk/tests/sdk_contract.rs` covering:

```rust
#[test]
fn prelude_reexports_core_plugin_types() {}

#[test]
fn abi_helpers_pack_and_unpack_i64_results() {}
```

**Step 2: Run the SDK tests**

Run: `cargo test -p hmanga-plugin-sdk --test sdk_contract`
Expected: FAIL because the SDK does not yet expose the plugin traits, ABI helpers, or proc-macro re-export.

**Step 3: Implement the SDK surface**

Expose:
- `SimplePlugin`, `FullPlugin`, `HostApi`
- `PluginMetaInfo`, `Capabilities`, request/response types
- ABI helpers:

```rust
pub fn pack_ptr_len(ptr: u32, len: u32) -> i64 {
    ((ptr as i64) << 32) | len as i64
}
```

- prelude:

```rust
pub mod prelude {
    pub use hmanga_plugin_macro::hmanga_plugin;
    pub use crate::{FullPlugin, HostApi, SimplePlugin};
}
```

**Step 4: Implement the proc-macro skeleton**

Generate placeholder exports for:
- `hm_meta`
- `hm_alloc`
- `hm_dealloc`
- either `hm_search`-family or `hm_build_*` / `hm_parse_*` family depending on the implemented trait

Keep the macro minimal first: compile-time structure only, no advanced diagnostics yet.

**Step 5: Re-run SDK validation**

Run: `cargo test -p hmanga-plugin-sdk --test sdk_contract`
Expected: PASS

**Step 6: Commit**

Commit with a Lore message recording the library/proc-macro crate split as a Rust toolchain constraint, not a design preference.

### Task 4: Implement `hmanga-host` with native and WASM adapters

**Files:**
- Modify: `crates/hmanga-host/src/lib.rs`
- Create: `crates/hmanga-host/src/registry.rs`
- Create: `crates/hmanga-host/src/native.rs`
- Create: `crates/hmanga-host/src/wasm.rs`
- Create: `crates/hmanga-host/src/host_api.rs`
- Create: `crates/hmanga-host/src/catalog.rs`
- Create: `crates/hmanga-host/tests/registry.rs`

**Step 1: Write failing adapter tests**

Create `crates/hmanga-host/tests/registry.rs` covering:

```rust
#[tokio::test]
async fn native_simple_plugin_adapter_executes_build_then_parse() {}

#[tokio::test]
async fn registry_exposes_locked_official_plugins_separately_from_active_plugins() {}

#[tokio::test]
async fn locked_official_plugins_are_not_activated_until_local_confirmation() {}

#[tokio::test]
async fn wasm_loader_rejects_incompatible_sdk_version() {}
```

**Step 2: Run the host tests**

Run: `cargo test -p hmanga-host --test registry`
Expected: FAIL with missing registry, adapter, and catalog implementations.

**Step 3: Implement the registry and native adapter**

Add:

```rust
pub struct PluginRegistry {
    plugins: HashMap<String, Arc<dyn PluginAdapter>>,
}

pub struct OfficialPluginCatalog {
    official: Vec<PluginInfo>,
}
```

Implement `SimplePluginAdapter` first, with request execution delegated through a shared `HostRuntime`.

**Step 4: Implement the WASM bridge**

Add:
- guest export discovery
- `hm_alloc` / `hm_dealloc` memory management
- MessagePack serialization for call arguments/results
- `spawn_blocking` boundary for guest execution
- `HostApi` imports backed by Tokio + Reqwest
- SDK compatibility check against an explicit supported range constant

**Step 5: Re-run the host tests**

Run: `cargo test -p hmanga-host --test registry`
Expected: PASS

**Step 6: Run crate-wide verification**

Run: `cargo test -p hmanga-host`
Expected: PASS

**Step 7: Commit**

Commit with a Lore message noting the separation between visible plugin catalog and active adapter registry.

### Task 5: Implement resumable downloads and export orchestration in `hmanga-core`

**Files:**
- Create: `crates/hmanga-core/src/download/mod.rs`
- Create: `crates/hmanga-core/src/download/manager.rs`
- Create: `crates/hmanga-core/src/download/export.rs`
- Create: `crates/hmanga-core/src/download/speed.rs`
- Modify: `crates/hmanga-core/src/lib.rs`
- Create: `crates/hmanga-core/tests/download_manager.rs`

**Step 1: Write failing download-manager tests**

Create `crates/hmanga-core/tests/download_manager.rs` covering:

```rust
#[tokio::test]
async fn resume_skips_images_already_present_on_disk() {}

#[tokio::test]
async fn emits_progress_and_completion_events_for_single_chapter_download() {}

#[tokio::test]
async fn export_progress_is_emitted_for_cbz() {}
```

**Step 2: Run the targeted tests**

Run: `cargo test -p hmanga-core --test download_manager`
Expected: FAIL because the download manager and speed tracker do not exist yet.

**Step 3: Implement the `DownloadManager` vertical slice**

Implement:
- task creation
- per-chapter and per-image semaphores
- event broadcasting
- restart resume by checking existing files on disk
- pluggable export runners for `Raw`, `Cbz`, `Pdf`
- a simple rolling-window `SpeedTracker`

Use deterministic image naming:

```rust
format!("{:04}.jpg", page_index + 1)
```

**Step 4: Re-run the targeted tests**

Run: `cargo test -p hmanga-core --test download_manager`
Expected: PASS

**Step 5: Run crate-wide verification**

Run: `cargo test -p hmanga-core`
Expected: PASS

**Step 6: Commit**

Commit with a Lore message documenting resume semantics and file naming so future changes do not silently break resumability.

### Task 6: Build the Dioxus desktop shell and state/event bridge

**Files:**
- Modify: `crates/hmanga-app/src/main.rs`
- Create: `crates/hmanga-app/src/app.rs`
- Create: `crates/hmanga-app/src/state.rs`
- Create: `crates/hmanga-app/src/routes.rs`
- Create: `crates/hmanga-app/src/components/sidebar.rs`
- Create: `crates/hmanga-app/src/components/search_pane.rs`
- Create: `crates/hmanga-app/src/components/downloads_pane.rs`
- Create: `crates/hmanga-app/src/components/settings_pane.rs`
- Create: `crates/hmanga-app/src/components/mod.rs`
- Create: `crates/hmanga-app/tests/bootstrap.rs`

**Step 1: Write a failing app bootstrap test**

Create `crates/hmanga-app/tests/bootstrap.rs` with a smoke test that asserts the app state initializes with:
- active site set to aggregate
- plugin list rendered from `OfficialPluginCatalog`
- only `jm` enabled by default
- donation unlock defaults to `false`

**Step 2: Run the bootstrap test**

Run: `cargo test -p hmanga-app --test bootstrap`
Expected: FAIL because app state, routes, and components are not implemented.

**Step 3: Implement the state container and shell**

Create:

```rust
pub struct AppState {
    pub active_site: Signal<SiteId>,
    pub plugins: Signal<Vec<PluginInfo>>,
    pub download_tasks: Signal<HashMap<TaskId, DownloadTask>>,
    pub donation_unlocked: Signal<bool>,
}
```

Wire:
- sidebar
- aggregate search tab shell
- downloads pane
- settings pane with "我已捐献，解锁插件" confirmation action
- `use_coroutine` event bridge for `DownloadEvent`

Keep UI text Chinese-only for v1.

**Step 4: Re-run the app bootstrap test**

Run: `cargo test -p hmanga-app --test bootstrap`
Expected: PASS

**Step 5: Run a desktop compile check**

Run: `cargo check -p hmanga-app`
Expected: PASS

**Step 6: Commit**

Commit with a Lore message capturing that locked plugins are visible in the UI but absent from the active runtime registry.

### Task 7: Add the JM official plugin baseline and local-confirmation unlock flow

**Files:**
- Modify: `crates/hmanga-plugin-jm/src/lib.rs`
- Create: `crates/hmanga-plugin-jm/src/api.rs`
- Create: `crates/hmanga-plugin-jm/src/crypto.rs`
- Create: `crates/hmanga-plugin-jm/src/image.rs`
- Create: `crates/hmanga-plugin-jm/tests/jm_plugin.rs`
- Modify: `crates/hmanga-app/src/settings.rs`
- Modify: `crates/hmanga-host/src/catalog.rs`

**Step 1: Write failing JM plugin tests**

Create `crates/hmanga-plugin-jm/tests/jm_plugin.rs` covering:

```rust
#[tokio::test]
async fn meta_reports_full_plugin_capabilities() {}

#[test]
fn unlock_confirmation_sets_donation_unlocked_flag() {}

#[test]
fn descramble_restores_fixture_image_dimensions() {}
```

**Step 2: Run the JM tests**

Run: `cargo test -p hmanga-plugin-jm --test jm_plugin`
Expected: FAIL because the plugin metadata, unlock helper, and image processing logic are not implemented.

**Step 3: Implement the first JM vertical slice**

Implement:
- `FullPlugin` metadata and capability flags
- local unlock helper that persists `donation_unlocked=true`
- request signing/decryption helpers
- descramble pipeline behind fixture-backed tests

Keep network-facing methods narrow at first:
- search
- comic detail
- chapter images
- process image

Defer favorites/ranking/weekly until the first slice compiles and tests.

**Step 4: Re-run the JM tests**

Run: `cargo test -p hmanga-plugin-jm --test jm_plugin`
Expected: PASS

**Step 5: Run workspace verification**

Run: `cargo test --workspace`
Expected: PASS

**Step 6: Commit**

Commit with a Lore message noting that JM ships as the single free activated plugin in v1.

### Task 8: Finish docs, release notes, and end-to-end verification

**Files:**
- Modify: `README.md`
- Create: `docs/plugin-guide/README.md`
- Create: `docs/plugin-guide/simple-plugin.md`
- Create: `docs/plugin-guide/full-plugin.md`
- Create: `docs/plugin-guide/wasm-abi.md`

**Step 1: Write the missing documentation checklist**

Create or update docs so they explain:
- workspace layout
- desktop-only v1 scope
- plugin SDK usage
- install path `~/.hmanga/plugins/`
- donation unlock behavior

**Step 2: Run full verification**

Run:
- `cargo fmt --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `cargo check -p hmanga-app`

Expected: PASS for all four commands.

**Step 3: Update the README**

Replace the current one-line repo description with:
- project overview
- current status
- build steps
- spec link
- v1 scope and non-goals

**Step 4: Commit**

Commit with a Lore message documenting the verified desktop-only release baseline and the remaining deferred scope.

## Final Verification Checklist

- `cargo fmt --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `cargo check -p hmanga-app`
- Manual smoke run of `cargo run -p hmanga-app`

## Deferred Until After This Plan

- Android build/release workflow
- WASM plugins on Android
- Cross-site deduplication in aggregate search
- Ranking/weekly/tags for non-JM plugins
- Rich reader modes beyond a basic desktop shell
