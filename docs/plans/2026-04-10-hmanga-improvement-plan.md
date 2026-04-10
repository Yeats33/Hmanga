# Hmanga Improvement Plan

## Goal

Move `Hmanga` from a usable preview to a day-to-day desktop manga tool close to
`Yeats33/jmcomic-downloader`, with emphasis on download stability, configuration
correctness, local library quality, and release reliability.

## Current Baseline

Already available:

- JM search, comic details, chapter downloads
- Favorites, weekly discovery, login, login restore
- Local shelf, reader preview, fullscreen reader
- CBZ export
- Legacy `jmcomic-downloader` library compatibility
- Tag-based GitHub release automation
- Initial multi-platform installer release workflow

Still incomplete or only partially aligned with the reference implementation:

- Proxy / route switching not fully wired through runtime updates
- Download format selection and optional cover download
- Full settings parity with the reference app
- Download queue grouping, richer telemetry, and retry behavior
- Local shelf update workflow polish
- PDF export
- Log viewer / log directory maintenance

## Phase P0: Stabilize In-Flight Work

Objective: close the current unfinished settings/runtime refactor before adding
new features.

Tasks:

1. Finish runtime hot-reload for:
   - API domain
   - custom API domain
   - proxy
2. Re-run:
   - `cargo fmt --all`
   - `cargo clippy --workspace --all-targets -- -D warnings`
   - `cargo test --workspace`
3. Publish a small patch release after the tree is clean.

Success criteria:

- Saving settings immediately affects subsequent network requests
- Workspace is green locally and in CI

## Phase P1: Complete Core Settings

Objective: make the settings page genuinely comparable to the reference app for
high-frequency usage.

Tasks:

1. Download behavior settings:
   - chapter concurrency
   - image concurrency
   - chapter download interval
   - image download interval
   - favorites batch interval
   - library update interval
2. Network settings:
   - system proxy
   - direct connection
   - custom proxy
   - API route switching
   - custom API domain
3. Output settings:
   - download format `jpg/png/webp`
   - download cover toggle
   - directory format (`dir_fmt`)

Success criteria:

- Every setting shown in the UI persists to config
- Every runtime-affecting setting is consumed by the service layer

## Phase P2: Upgrade Download Engine

Objective: make the downloader closer to the reference implementation in
responsiveness and observability.

Tasks:

1. Improve chapter download flow:
   - robust pause / resume / cancel semantics
   - clearer per-task lifecycle
   - better handling for partial progress
2. Improve image download flow:
   - stronger chapter-internal concurrency behavior
   - retry strategy for transient failures
   - optional per-image backoff
3. Improve queue UX:
   - incomplete / complete grouping
   - speed display
   - ETA or at least useful progress summaries
   - clearer failure detail

Success criteria:

- Queue remains usable under multiple concurrent chapters
- Pausing and resuming feels predictable
- Download rows expose meaningful status beyond a single word

## Phase P3: Complete Discovery and Account Features

Objective: close the most useful JM browsing gaps.

Tasks:

1. Favorites:
   - pagination polish
   - folder support
   - sync favorites
   - stronger batch download flow
2. Weekly:
   - better category/type transitions
   - smooth handoff from weekly items to chapter flow
3. Account:
   - richer user info block
   - clearer login failure and expired-session handling

Success criteria:

- Favorites and weekly panes can drive the same full browse-download-read loop
- Account state is stable across restarts

## Phase P4: Complete Local Shelf

Objective: make local management feel like a first-class page, not just a file
viewer.

Tasks:

1. Library update:
   - rescan local comics
   - compare against remote chapters
   - queue missing chapters
2. Shelf UX:
   - cover display
   - filtering / searching
   - clearer platform badges
   - better delete confirmations
3. Export:
   - keep CBZ polished
   - add PDF export

Success criteria:

- Local shelf is good enough for users who rarely re-download old content
- Export options cover the two most useful archival formats

## Phase P5: Logs and Operational UX

Objective: improve diagnosability and reduce support friction.

Tasks:

1. Add log view / log controls:
   - open log directory
   - inspect recent log output
   - warn when log directory grows too large
2. Release workflow polish:
   - verify installer names and artifact paths across runners
   - keep release notes concise and reliable
3. Documentation:
   - short release note template
   - known limitations section

Success criteria:

- Common user-reported failures are diagnosable from the app UI
- Tagged releases consistently publish usable assets

## Recommended Versioning

### `v0.1.3`

Scope:

- Finish P0
- Stabilize settings/runtime updates

### `v0.1.4`

Scope:

- P1
- the highest-value parts of P2

### `v0.2.0`

Scope:

- P3
- P4 core deliverables

### `v0.2.x`

Scope:

- P5
- remaining polish gaps from earlier phases

## Immediate Next Steps

1. Finish the runtime hot-reload refactor so proxy and API route settings
   actually reconfigure the active JM runtime.
2. Add download format and cover-download toggles to the settings page and make
   the downloader honor them.
3. Improve the download queue display with speed and clearer progress grouping.
