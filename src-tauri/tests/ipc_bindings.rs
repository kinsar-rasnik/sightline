//! IPC binding drift test.
//!
//! Regenerates `src/ipc/bindings.ts` via the same code path the dev build
//! uses. If CI runs `git diff --exit-code src/ipc/bindings.ts` after this
//! test, any stale committed file is rejected.
//!
//! The test only runs on non-Windows targets: constructing `Builder<Wry>`
//! pulls in `wry` → `webview2-com-sys`, whose generated test binary fails
//! to load on the GitHub Actions Windows runner with
//! `STATUS_ENTRYPOINT_NOT_FOUND` (0xc0000139). The generated TS file is
//! byte-identical across platforms, so running the drift check on Linux
//! and macOS is sufficient to catch regressions.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

#[cfg(not(target_os = "windows"))]
use sightline_lib::ipc::{export_bindings, ipc_builder};

#[cfg(not(target_os = "windows"))]
#[test]
fn regenerate_ipc_bindings() {
    let builder = ipc_builder();
    export_bindings(&builder).expect("export tauri-specta bindings");
}
