//! IPC binding drift test.
//!
//! Regenerates `src/ipc/bindings.ts` via the same code path the dev build
//! uses. If CI runs `git diff --exit-code src/ipc/bindings.ts` after this
//! test, any stale committed file is rejected.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use sightline_lib::ipc::{export_bindings, ipc_builder};

#[test]
fn regenerate_ipc_bindings() {
    let builder = ipc_builder();
    export_bindings(&builder).expect("export tauri-specta bindings");
}
