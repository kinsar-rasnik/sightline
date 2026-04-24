//! IPC binding builder.
//!
//! A single source of truth for the command + event surface exposed to the
//! webview. `ipc_builder()` is consumed by:
//!
//! * [`crate::run`] — mounts it on the Tauri builder and (in debug builds)
//!   regenerates `src/ipc/bindings.ts`.
//! * `tests/ipc_bindings.rs` — regenerates the file and lets CI diff it
//!   against the committed copy to detect drift.
//!
//! See ADR-0007 for the rationale.

use std::path::{Path, PathBuf};

use specta_typescript::Typescript;
use tauri::Wry;
use tauri_specta::{Builder, collect_commands, collect_events};

use crate::commands;

/// Build the tauri-specta `Builder` with every command and event the app
/// exposes. The same instance is used at runtime (as the `invoke_handler`
/// source) and offline (to regenerate bindings).
pub fn ipc_builder() -> Builder<Wry> {
    Builder::<Wry>::new()
        .commands(collect_commands![commands::health::health,])
        .events(collect_events![])
}

/// Target path of the generated TS bindings, relative to the workspace
/// root. Computed from `CARGO_MANIFEST_DIR` so the test and the runtime
/// export always resolve to the same place.
pub fn bindings_path() -> PathBuf {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    // `src-tauri/` → `../src/ipc/bindings.ts`.
    manifest
        .parent()
        .unwrap_or(manifest)
        .join("src")
        .join("ipc")
        .join("bindings.ts")
}

/// Export the Builder to the canonical bindings file. Called from the
/// debug-mode `setup` hook and from the drift test.
pub fn export_bindings(builder: &Builder<Wry>) -> Result<(), String> {
    let config = Typescript::default().header(TS_HEADER);

    builder
        .export(config, bindings_path())
        .map_err(|e| format!("tauri-specta export: {e}"))
}

const TS_HEADER: &str = "// AUTO-GENERATED — DO NOT EDIT.\n\
// Regenerated on every `pnpm tauri dev` (debug) and via\n\
// `cargo test --test ipc_bindings` (CI drift check). See\n\
// docs/adr/0007-ipc-typegen.md.\n\
\n\
/* eslint-disable */\n";
