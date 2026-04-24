//! App-level Phase 4 commands: tray summary, shutdown coordination,
//! window close behavior, favorites, shortcut customization.

use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::Emitter;

use crate::AppState;
use crate::error::AppError;
use crate::services::downloads::DownloadsSummary;
use crate::services::events::{
    AppShutdownRequestedEvent, AppTrayActionEvent, EV_APP_SHUTDOWN_REQUESTED, EV_APP_TRAY_ACTION,
    EV_STREAMER_FAVORITED, EV_STREAMER_UNFAVORITED, StreamerFavoritedEvent,
    StreamerUnfavoritedEvent,
};
use crate::services::settings::{SettingsPatch, WindowCloseBehavior};

/// Snapshot used by the tray popover + menu-bar tooltip.
#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AppSummary {
    pub active_downloads: i64,
    pub queued_downloads: i64,
    pub bandwidth_bps: i64,
    /// `None` when no streamer is due yet (empty roster). Seconds to
    /// the next scheduled poll for any streamer.
    pub next_poll_eta_seconds: Option<i64>,
    /// `None` when the user has no streamers yet.
    pub streamer_count: i64,
}

#[tauri::command]
#[specta::specta]
pub async fn get_app_summary(state: tauri::State<'_, AppState>) -> Result<AppSummary, AppError> {
    let DownloadsSummary {
        active_count,
        queued_count,
        bandwidth_bps,
    } = state.downloads.summary().await?;
    let streamers = state.streamers.list_active().await?;
    let next_poll_eta_seconds = streamers
        .iter()
        .filter_map(|s| s.next_poll_eta_seconds)
        .min();
    Ok(AppSummary {
        active_downloads: active_count,
        queued_downloads: queued_count,
        bandwidth_bps,
        next_poll_eta_seconds,
        streamer_count: streamers.len() as i64,
    })
}

#[tauri::command]
#[specta::specta]
pub async fn pause_all_downloads(state: tauri::State<'_, AppState>) -> Result<i64, AppError> {
    state.downloads.pause_all().await
}

#[tauri::command]
#[specta::specta]
pub async fn resume_all_downloads(state: tauri::State<'_, AppState>) -> Result<i64, AppError> {
    state.downloads.resume_all().await
}

#[derive(Debug, Clone, Deserialize, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SetWindowCloseBehaviorInput {
    pub behavior: WindowCloseBehavior,
}

#[tauri::command]
#[specta::specta]
pub async fn set_window_close_behavior(
    state: tauri::State<'_, AppState>,
    input: SetWindowCloseBehaviorInput,
) -> Result<(), AppError> {
    state
        .settings
        .update(SettingsPatch {
            window_close_behavior: Some(input.behavior),
            ..Default::default()
        })
        .await?;
    Ok(())
}

#[derive(Debug, Clone, Deserialize, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ToggleFavoriteInput {
    pub streamer_id: String,
}

#[tauri::command]
#[specta::specta]
pub async fn toggle_streamer_favorite(
    state: tauri::State<'_, AppState>,
    input: ToggleFavoriteInput,
) -> Result<bool, AppError> {
    let (_summary, now_favorite) = state.streamers.toggle_favorite(&input.streamer_id).await?;
    let topic = if now_favorite {
        EV_STREAMER_FAVORITED
    } else {
        EV_STREAMER_UNFAVORITED
    };
    if now_favorite {
        let _ = state.app_handle.emit(
            topic,
            StreamerFavoritedEvent {
                twitch_user_id: input.streamer_id.clone(),
            },
        );
    } else {
        let _ = state.app_handle.emit(
            topic,
            StreamerUnfavoritedEvent {
                twitch_user_id: input.streamer_id.clone(),
            },
        );
    }
    Ok(now_favorite)
}

/// Broadcast a shutdown request with a 10 s drain deadline. Services
/// subscribe to this event via their existing `.shutdown()` handles;
/// the webview can show a "saving state…" toast until the process
/// actually exits.
#[tauri::command]
#[specta::specta]
pub async fn request_shutdown(state: tauri::State<'_, AppState>) -> Result<(), AppError> {
    let deadline_at = unix_now() + 10;
    let _ = state.app_handle.emit(
        EV_APP_SHUTDOWN_REQUESTED,
        AppShutdownRequestedEvent { deadline_at },
    );
    // Kick the services into their drain phase. Actual process exit is
    // issued by the tray's Quit handler (lib.rs) so the drain has a
    // chance to flush.
    state.poller_handle.shutdown();
    state.downloads_handle.shutdown();
    Ok(())
}

#[derive(Debug, Clone, Deserialize, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct TrayActionInput {
    /// Freely-formed action identifier. See `commands/tray_actions.ts`
    /// on the frontend for the closed set.
    pub kind: String,
}

/// Called by the tray menu handlers to emit a uniform webview event.
/// Always succeeds — emission errors are logged and swallowed.
#[tauri::command]
#[specta::specta]
pub async fn emit_tray_action(
    state: tauri::State<'_, AppState>,
    input: TrayActionInput,
) -> Result<(), AppError> {
    let _ = state
        .app_handle
        .emit(EV_APP_TRAY_ACTION, AppTrayActionEvent { kind: input.kind });
    Ok(())
}

/// Freeform key → keystroke map. The frontend owns the canonical list
/// of action IDs (e.g. `library`, `timeline`, `focus_search`, …); the
/// backend just persists the chosen keystrokes so they survive
/// restarts. We store the serialized map as a JSON string under a
/// single row in a dedicated table.
#[derive(Debug, Clone, Deserialize, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Shortcut {
    pub action_id: String,
    pub keys: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SetShortcutInput {
    pub action_id: String,
    pub keys: String,
}

#[tauri::command]
#[specta::specta]
pub async fn list_shortcuts(state: tauri::State<'_, AppState>) -> Result<Vec<Shortcut>, AppError> {
    state.shortcuts.list().await
}

#[tauri::command]
#[specta::specta]
pub async fn set_shortcut(
    state: tauri::State<'_, AppState>,
    input: SetShortcutInput,
) -> Result<Vec<Shortcut>, AppError> {
    state.shortcuts.set(&input.action_id, &input.keys).await?;
    state.shortcuts.list().await
}

#[tauri::command]
#[specta::specta]
pub async fn reset_shortcuts(state: tauri::State<'_, AppState>) -> Result<Vec<Shortcut>, AppError> {
    state.shortcuts.reset().await?;
    state.shortcuts.list().await
}

fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}
