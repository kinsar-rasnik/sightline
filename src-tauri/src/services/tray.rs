//! Tray / menu-bar integration (Phase 5 housekeeping).
//!
//! Builds the platform-native tray icon + menu on app startup. Menu
//! items mirror the backend commands described in
//! [ADR-0014](../../docs/adr/0014-tray-daemon-architecture.md); the
//! actions flow through `cmd_emit_tray_action` so the existing webview
//! listener in `AppShell` handles route switching.
//!
//! This module is deliberately small: the hard work lives in the
//! Phase-4 commands (`pause_all_downloads`, `resume_all_downloads`,
//! `request_shutdown`, `emit_tray_action`). The tray is just a second
//! entry surface on top of those.

use std::sync::Arc;

use tauri::{
    AppHandle, Manager, Runtime,
    menu::{Menu, MenuBuilder, MenuItemBuilder, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};
use tracing::{debug, error, warn};

use crate::AppState;

/// Stable identifier for the application's tray icon. Matches
/// `tauri.conf.json` naming conventions — lowercase kebab-case.
pub const TRAY_ID: &str = "sightline-main";

/// Menu item ids. Kept as untyped strings here so a future refactor
/// that moves them into a `TrayMenuItem` enum can happen without a
/// protocol break. Each id maps 1:1 to a `TrayActionKind` or a backend
/// command.
pub mod menu_ids {
    pub const SUMMARY: &str = "summary";
    pub const OPEN_SIGHTLINE: &str = "open_sightline";
    pub const OPEN_LIBRARY: &str = "open_library";
    pub const OPEN_TIMELINE: &str = "open_timeline";
    pub const OPEN_DOWNLOADS: &str = "open_downloads";
    pub const OPEN_SETTINGS: &str = "open_settings";
    pub const PAUSE_ALL: &str = "pause_all";
    pub const RESUME_ALL: &str = "resume_all";
    pub const QUIT: &str = "quit";
}

/// The inventory the frontend tests + backend integration test use to
/// assert that every menu item the user can click exists. Pure data;
/// no runtime dependency.
pub fn expected_menu_ids() -> Vec<&'static str> {
    vec![
        menu_ids::SUMMARY,
        menu_ids::OPEN_SIGHTLINE,
        menu_ids::OPEN_LIBRARY,
        menu_ids::OPEN_TIMELINE,
        menu_ids::OPEN_DOWNLOADS,
        menu_ids::OPEN_SETTINGS,
        menu_ids::PAUSE_ALL,
        menu_ids::RESUME_ALL,
        menu_ids::QUIT,
    ]
}

/// Format the summary menu item's label from the live app summary.
/// Pure — unit tested below.
pub fn summary_label(active: i64, queued: i64, next_poll_eta_seconds: Option<i64>) -> String {
    let mut parts = Vec::with_capacity(3);
    if active > 0 {
        parts.push(format!("↓ {active} active"));
    }
    if queued > 0 {
        parts.push(format!("⋯ {queued} queued"));
    }
    if let Some(eta) = next_poll_eta_seconds
        && eta >= 0
    {
        let mins = eta / 60;
        if mins == 0 {
            parts.push("poll imminent".to_owned());
        } else if mins < 60 {
            parts.push(format!("poll in {mins}m"));
        } else {
            parts.push(format!("poll in {}h", mins / 60));
        }
    }
    if parts.is_empty() {
        "Idle".to_owned()
    } else {
        parts.join(" · ")
    }
}

/// Build the tray menu. Extracted so unit tests can construct it against
/// a test harness's `AppHandle` without paying for the full app setup.
pub fn build_menu<R: Runtime>(handle: &AppHandle<R>) -> tauri::Result<Menu<R>> {
    let summary = MenuItemBuilder::with_id(menu_ids::SUMMARY, "Sightline")
        .enabled(false)
        .build(handle)?;
    let open_sightline =
        MenuItemBuilder::with_id(menu_ids::OPEN_SIGHTLINE, "Open Sightline").build(handle)?;
    let open_library =
        MenuItemBuilder::with_id(menu_ids::OPEN_LIBRARY, "Open Library").build(handle)?;
    let open_timeline =
        MenuItemBuilder::with_id(menu_ids::OPEN_TIMELINE, "Open Timeline").build(handle)?;
    let open_downloads =
        MenuItemBuilder::with_id(menu_ids::OPEN_DOWNLOADS, "Open Downloads").build(handle)?;
    let open_settings =
        MenuItemBuilder::with_id(menu_ids::OPEN_SETTINGS, "Settings…").build(handle)?;
    let pause_all =
        MenuItemBuilder::with_id(menu_ids::PAUSE_ALL, "Pause all downloads").build(handle)?;
    let resume_all =
        MenuItemBuilder::with_id(menu_ids::RESUME_ALL, "Resume all downloads").build(handle)?;
    let quit = MenuItemBuilder::with_id(menu_ids::QUIT, "Quit Sightline").build(handle)?;
    let sep = PredefinedMenuItem::separator(handle)?;

    MenuBuilder::new(handle)
        .items(&[
            &summary,
            &sep,
            &open_sightline,
            &open_library,
            &open_timeline,
            &open_downloads,
            &open_settings,
            &sep,
            &pause_all,
            &resume_all,
            &sep,
            &quit,
        ])
        .build()
}

/// Install the tray icon. `run()` calls this once at startup; it loads
/// the platform-appropriate icon from the bundle resources, builds the
/// menu, and wires up the click handlers.
///
/// `icon_bytes` / `is_template` are passed in so tests can inject
/// a tiny 1×1 PNG without paying for disk I/O.
pub fn install<R: Runtime>(
    handle: &AppHandle<R>,
    icon_bytes: &[u8],
    is_template: bool,
) -> tauri::Result<()> {
    let menu = build_menu(handle)?;
    let icon = tauri::image::Image::from_bytes(icon_bytes)?;
    let tray = TrayIconBuilder::<R>::with_id(TRAY_ID)
        .icon(icon)
        .icon_as_template(is_template)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .tooltip("Sightline")
        .on_menu_event(move |app, event| {
            on_menu_event(app, event.id().0.as_str());
        })
        .on_tray_icon_event(|tray, event| {
            // Left-click reopens the window. Right-click shows the
            // menu (the default), which we hand off to the OS.
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
                && let Some(window) = tray.app_handle().get_webview_window("main")
            {
                let _ = window.show();
                let _ = window.set_focus();
            }
        })
        .build(handle)?;
    // Keep a reference so the icon doesn't drop at the end of setup.
    handle.manage(Arc::new(tray));
    Ok(())
}

/// Handle a tray-menu click. Pure routing — the per-kind work lives in
/// the existing commands. `quit` is handled in-process because it has
/// to bypass the webview lifecycle (the window may be hidden).
fn on_menu_event<R: Runtime>(app: &AppHandle<R>, id: &str) {
    debug!(menu_id = %id, "tray menu click");
    match id {
        menu_ids::QUIT => {
            if let Some(state) = app.try_state::<AppState>() {
                state.poller_handle.shutdown();
                state.downloads_handle.shutdown();
            }
            // Give the drain a moment; the existing `on_window_event`
            // Quit branch does the same.
            app.exit(0);
        }
        other if expected_menu_ids().contains(&other) => {
            // Forward to the frontend via the existing tray-action bus.
            use crate::services::events::{AppTrayActionEvent, EV_APP_TRAY_ACTION};
            use tauri::Emitter;
            if let Err(e) = app.emit(
                EV_APP_TRAY_ACTION,
                AppTrayActionEvent {
                    kind: other.to_owned(),
                },
            ) {
                error!(error = %e, id = %other, "tray action emit failed");
            }
            // Also ensure the window is visible for any "open_*" action.
            if other.starts_with("open_")
                && let Some(window) = app.get_webview_window("main")
            {
                let _ = window.show();
                let _ = window.set_focus();
            }
            // pause_all / resume_all also pipe through the commands layer
            // so DB state stays consistent. We invoke them via the
            // managed state's service handles directly to avoid an IPC
            // round-trip.
            if other == menu_ids::PAUSE_ALL
                && let Some(state) = app.try_state::<AppState>()
            {
                let downloads = state.downloads.clone();
                tauri::async_runtime::spawn(async move {
                    if let Err(e) = downloads.pause_all().await {
                        warn!(error = ?e, "tray pause_all failed");
                    }
                });
            }
            if other == menu_ids::RESUME_ALL
                && let Some(state) = app.try_state::<AppState>()
            {
                let downloads = state.downloads.clone();
                tauri::async_runtime::spawn(async move {
                    if let Err(e) = downloads.resume_all().await {
                        warn!(error = ?e, "tray resume_all failed");
                    }
                });
            }
        }
        _ => warn!(id = %id, "unknown tray menu id"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expected_menu_ids_match_tray_action_kinds() {
        // Every id except `summary` (disabled) and `quit` (handled
        // in-process) should map to a TrayActionKind wire string.
        let mapped: Vec<&str> = expected_menu_ids()
            .into_iter()
            .filter(|id| *id != menu_ids::SUMMARY && *id != menu_ids::QUIT)
            .collect();
        let expected = vec![
            "open_sightline",
            "open_library",
            "open_timeline",
            "open_downloads",
            "open_settings",
            "pause_all",
            "resume_all",
        ];
        assert_eq!(mapped, expected);
    }

    #[test]
    fn summary_label_handles_all_branches() {
        assert_eq!(summary_label(0, 0, None), "Idle");
        assert_eq!(summary_label(0, 0, Some(0)), "poll imminent");
        assert_eq!(summary_label(2, 0, None), "↓ 2 active");
        assert_eq!(summary_label(0, 5, Some(90)), "⋯ 5 queued · poll in 1m");
        assert_eq!(
            summary_label(3, 2, Some(3600)),
            "↓ 3 active · ⋯ 2 queued · poll in 1h"
        );
    }

    #[test]
    fn summary_label_skips_negative_eta() {
        // An overdue poll shouldn't render a nonsensical "poll in -1m"
        // line; the scheduler will fire the next cycle anyway.
        assert_eq!(summary_label(1, 0, Some(-30)), "↓ 1 active");
    }
}
