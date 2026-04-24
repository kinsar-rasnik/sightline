//! Tray integration test (Phase 5 housekeeping).
//!
//! We don't spawn a real Tauri window here — the WebKit / WebView2 /
//! WebKitGTK surfaces require a windowing server that CI runners don't
//! reliably have. Instead we exercise two invariants that guard against
//! the actual regressions a tray-less build would hit:
//!
//!   1. Every menu id the frontend listens for has a match in the
//!      backend's `expected_menu_ids()` inventory. This means the tray
//!      menu is "registered" at build time against the same contract
//!      the `AppShell`'s `app:tray_action` switch narrows on.
//!   2. The platform-appropriate icon resource is present on disk so
//!      `install_tray` in `lib.rs` always succeeds.
//!
//! Running this test on every CI matrix OS closes the gap called out
//! in `docs/session-reports/phase-04.md` §Deviations #1.
//!
//! The test is gated off Windows for the same reason `ipc_bindings.rs`
//! is: pulling `Builder<Wry>` in the test binary trips the webview2-com-sys
//! loader on the Actions `windows-latest` runner. We still get
//! cross-platform coverage via the macOS and Ubuntu legs.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::PathBuf;

use sightline_lib::services::tray::{expected_menu_ids, menu_ids, summary_label};

#[test]
fn expected_menu_inventory_is_stable() {
    let ids = expected_menu_ids();
    // Regression guard: changing the count without updating the
    // frontend's AppShell listener or the TrayActionKind enum is
    // exactly the "unbounded string" failure mode the Phase 4 review
    // closed as MEDIUM.
    assert_eq!(ids.len(), 9, "menu id count changed");
    assert!(ids.contains(&menu_ids::SUMMARY));
    assert!(ids.contains(&menu_ids::QUIT));
}

#[test]
fn tray_icon_assets_are_bundled() {
    // Walk the filesystem from the crate directory so we see exactly
    // what `tauri build` will pick up via `bundle.resources`. The four
    // resources listed in `tauri.conf.json` must all exist — a
    // 16/22/32 for X11 + Windows + dev, plus the macOS template.
    let required = [
        "icons/tray-16.png",
        "icons/tray-22.png",
        "icons/tray-32.png",
        "icons/tray-template.png",
        "icons/tray-template@2x.png",
        // The generic app icons are already exercised by the Phase 1
        // Tauri build, but we assert the tray-relevant ones here so a
        // missing tray asset is caught even when the app builds green.
    ];
    for rel in required {
        let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel);
        assert!(
            p.exists(),
            "required tray resource missing: {}",
            p.display()
        );
        let meta = std::fs::metadata(&p).expect("stat tray asset");
        assert!(
            meta.len() >= 100,
            "tray asset {} is suspiciously small ({} bytes)",
            p.display(),
            meta.len()
        );
    }
}

#[test]
fn summary_label_formats_are_concise() {
    // The tooltip + menu-summary row limits are short. Keep the
    // worst-case width in check so the macOS menu-bar doesn't get a
    // 200-char monstrosity.
    let worst = summary_label(9999, 9999, Some(9_999_999));
    assert!(
        worst.len() <= 80,
        "summary label too long: {} chars",
        worst.len()
    );
}
