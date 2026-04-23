---
description: Rust backend conventions; loaded when touching files in src-tauri/
glob: "src-tauri/**/*.rs"
---

# Rust backend rules

## Layering

- `commands/` → `services/` → `domain/` / `infra/`. No back-edges.
- A command handler is ≤20 lines: deserialize, call one service method, serialize. If a handler grows beyond that, the logic belongs in a service.
- `domain/` modules have no `use` of `tokio`, `sqlx`, `reqwest`. If you are tempted, the type you're writing belongs in `services/` or `infra/`.

## Error handling

- Every module that can fail defines a `thiserror` enum at the top of the file. Do not propagate `anyhow::Error` across module boundaries.
- `AppError` is the only error that crosses IPC. Lower-level errors map into `AppError::*` at the services layer.
- `unwrap()`, `expect()`, and direct `panic!` are forbidden outside `#[cfg(test)]`. Clippy enforces this via `unwrap_used`, `expect_used`, `panic`.

## Async

- Never hold a `std::sync::Mutex` guard across `.await`. Use `tokio::sync::Mutex`/`RwLock` for shared state crossing an await, or scope the guard with `{ let x = mtx.lock().unwrap(); ... }`.
- Prefer `tokio::select!` for structured cancellation over `tokio::spawn`-then-abort.
- Long-lived services own an `mpsc::Receiver` for commands and a `oneshot::Receiver` for shutdown.

## sqlx

- Use the `query!`/`query_as!` macros where possible for compile-time validation. In CI, `SQLX_OFFLINE=true` with committed `.sqlx/` metadata.
- Every write goes through a `Transaction`. Reads can use the pool directly.
- No string concatenation into SQL. Bind parameters only.

## Testing

- Unit tests live in a `#[cfg(test)] mod tests` block at the bottom of the same file, except when the module exceeds ~400 lines — then move to `src-tauri/tests/<module>.rs`.
- Integration tests use an in-memory or temp-file SQLite pool via `SqlitePool::connect("sqlite::memory:")`.
- Clock-dependent code injects a `Clock` trait; never read wall-clock time directly from domain/services code.

## Style

- `rustfmt` with the settings in `rustfmt.toml`. No hand-aligned code.
- Public items have a one-line doc comment. Non-trivial internal items have doc comments when the name is not self-explanatory.
- No `#[allow(clippy::...)]` without a trailing comment explaining why.
