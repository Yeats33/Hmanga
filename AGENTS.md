# Repository Guidelines

## Project Structure & Module Organization

This Rust workspace contains:

- **`crates/hmanga-core`**: Core domain logic (models, error handling, persistence, download management)
- **`crates/hmanga-host`**: Plugin execution engine supporting both native and WASM plugins
- **`crates/hmanga-app`**: Desktop GUI application built with Dioxus
- **`crates/hmanga-plugin-jm`**:JMcomic plugin implementation
- **`plugin-sdk/hmanga-plugin-sdk`**: Plugin development SDK and macros

All crates follow the standard `src/lib.rs` / `src/main.rs` structure.

## Build, Test, and Development Commands

| Command | Description |
|---------|-------------|
| `cargo build` | Compile all workspace crates |
| `cargo build --release` | Build optimized release binaries |
| `cargo test` | Run all unit and integration tests |
| `cargo run -p hmanga-app` | Launch the desktop application |
| `cargo run --desktop` | Alias for launching the app (defined in `.cargo/config.toml`) |

## Coding Style & Naming Conventions

- Edition: Rust 2021
- Formatting: `cargo fmt` (enforced via CI)
- Linting: `cargo clippy` (warnings treated as errors)
- Documentation: All public APIs must have doc comments
- Error handling: Use `thiserror` for error types
- Async: Prefer `tokio` for async operations

## Testing Guidelines

- Framework: Built-in Rust test framework (`#[test]`)
- Coverage: Aim for >80% coverage on core logic
- Test location: `tests/` directory alongside source
- Integration tests: Place in `crates/*/tests/` with descriptive names (e.g., `download_manager.rs`)

## Commit & Pull Request Guidelines

- **Commit format**: `<type>(<scope>): <description>`
  - Types: `feat`, `fix`, `refactor`, `docs`, `test`, `chore`
  - Examples: `feat(core): implement resumable downloads`, `fix(host): wasm plugin loading`
- **PR requirements**:
  - Clear description of changes
  - Link related issues
  - Include test coverage updates
  - Run `cargo fmt` and `cargo clippy` before committing

## Security & Configuration Tips

- Never commit secrets or API keys
- `.gitignore` excludes `target/`, `.worktrees/`, and `.omx/`
- Use workspace dependencies for version consistency
- WASM plugins run in sandboxed `wasmtime` runtime
