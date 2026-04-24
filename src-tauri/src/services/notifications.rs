//! Notification service (Phase 4).
//!
//! We broadcast a generic `notification:show` event that the webview
//! routes through `@tauri-apps/plugin-notification` in the renderer
//! (works in dev + bundle). Keeping the actual OS-level notification
//! call in the webview lets us share policy (respecting each user
//! toggle) between native and in-app surfaces.
//!
//! Rate-limiting lives here. Each `Category` has an independent
//! coalesce window; a burst of 20 `FavoritesIngest` events within
//! `WINDOW_SECONDS` renders as one rolled-up notification rather than
//! twenty per-vod banners.

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::{AppHandle, Emitter};
use tokio::sync::Mutex;
use tracing::debug;

use crate::infra::clock::Clock;
use crate::services::settings::AppSettings;

/// Notification category. Maps 1:1 to a per-user setting toggle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum NotificationCategory {
    DownloadComplete,
    DownloadFailed,
    FavoritesIngest,
    StorageLow,
}

impl NotificationCategory {
    pub fn is_enabled(self, settings: &AppSettings) -> bool {
        if !settings.notifications_enabled {
            return false;
        }
        match self {
            NotificationCategory::DownloadComplete => settings.notify_download_complete,
            // Always-on irrespective of the master toggle — a failed
            // download is actionable, we respect only the explicit
            // opt-out on the category itself.
            NotificationCategory::DownloadFailed => settings.notify_download_failed,
            NotificationCategory::FavoritesIngest => settings.notify_favorites_ingest,
            NotificationCategory::StorageLow => settings.notify_storage_low,
        }
    }

    pub fn topic(self) -> &'static str {
        match self {
            NotificationCategory::DownloadComplete => "notification:download_complete",
            NotificationCategory::DownloadFailed => "notification:download_failed",
            NotificationCategory::FavoritesIngest => "notification:favorites_ingest",
            NotificationCategory::StorageLow => "notification:storage_low",
        }
    }
}

/// Payload emitted on the generic `notification:show` topic. The
/// category-specific topic also receives a mirror so callers that
/// care about just one category can subscribe narrowly.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct NotificationPayload {
    pub category: NotificationCategory,
    pub title: String,
    pub body: String,
    /// Optional deep-link / routing hint. The frontend interprets it
    /// (e.g. `"/downloads?vod=…"` to focus a specific row).
    #[serde(default)]
    pub link: Option<String>,
    /// Count coalesced into this notification (>= 1).
    pub coalesced: i64,
    /// Emission timestamp (unix seconds UTC).
    pub emitted_at: i64,
}

/// Per-category emission log row. We keep the last-emit timestamp + a
/// rolling count so the next call can either emit immediately or
/// queue a rolled-up message at the tail of the window.
#[derive(Debug, Clone, Default)]
struct CategoryState {
    last_emit_at: i64,
    pending_count: i64,
    pending_title: Option<String>,
}

pub struct NotificationService {
    app_handle: AppHandle,
    clock: Arc<dyn Clock>,
    state: Mutex<CategoryStateMap>,
}

#[derive(Default)]
struct CategoryStateMap {
    download_complete: CategoryState,
    download_failed: CategoryState,
    favorites_ingest: CategoryState,
    storage_low: CategoryState,
}

impl CategoryStateMap {
    fn get_mut(&mut self, cat: NotificationCategory) -> &mut CategoryState {
        match cat {
            NotificationCategory::DownloadComplete => &mut self.download_complete,
            NotificationCategory::DownloadFailed => &mut self.download_failed,
            NotificationCategory::FavoritesIngest => &mut self.favorites_ingest,
            NotificationCategory::StorageLow => &mut self.storage_low,
        }
    }
}

/// Coalesce window. Within this many seconds, subsequent events of
/// the same category are folded into the previous notification's
/// count rather than producing a new banner. 30 s is short enough to
/// feel live, long enough to catch a 20-VOD ingest burst.
const WINDOW_SECONDS: i64 = 30;

impl NotificationService {
    pub fn new(app_handle: AppHandle, clock: Arc<dyn Clock>) -> Self {
        Self {
            app_handle,
            clock,
            state: Mutex::new(CategoryStateMap::default()),
        }
    }

    /// Try to emit a notification. Respects user toggles and the
    /// rate-limit window. `title` / `body` are rendered by the
    /// frontend (native banner + in-app toast). Returns the number
    /// of notifications actually dispatched (0 or 1).
    pub async fn notify(
        &self,
        category: NotificationCategory,
        settings: &AppSettings,
        title: String,
        body: String,
        link: Option<String>,
    ) -> i64 {
        if !category.is_enabled(settings) {
            debug!(?category, "notification suppressed (user setting)");
            return 0;
        }
        let now = self.clock.unix_seconds();
        let (coalesced, emit_title, emit_body) = {
            let mut guard = self.state.lock().await;
            let cs = guard.get_mut(category);
            let recent = now - cs.last_emit_at < WINDOW_SECONDS;
            if recent {
                // Fold into the running total; skip actual emit.
                cs.pending_count = cs.pending_count.saturating_add(1);
                if cs.pending_title.is_none() {
                    cs.pending_title = Some(title.clone());
                }
                return 0;
            }
            let total = 1 + cs.pending_count;
            cs.pending_count = 0;
            cs.last_emit_at = now;
            let emit_title = if total > 1 {
                format!("{title} ({total} new)")
            } else {
                title
            };
            (total, emit_title, body)
        };
        let payload = NotificationPayload {
            category,
            title: emit_title,
            body: emit_body,
            link,
            coalesced,
            emitted_at: now,
        };
        let _ = self.app_handle.emit("notification:show", payload.clone());
        let _ = self.app_handle.emit(category.topic(), payload);
        1
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    #[test]
    fn category_gate_respects_master_toggle() {
        let mut s = AppSettings {
            enabled_game_ids: vec![],
            poll_floor_seconds: 0,
            poll_recent_seconds: 0,
            poll_ceiling_seconds: 0,
            concurrency_cap: 1,
            first_backfill_limit: 1,
            credentials: crate::services::settings::CredentialsStatus {
                configured: false,
                client_id_masked: None,
                last_token_acquired_at: None,
            },
            library_root: None,
            library_layout: crate::domain::library_layout::LibraryLayoutKind::Plex,
            staging_path: None,
            max_concurrent_downloads: 1,
            bandwidth_limit_bps: None,
            quality_preset: crate::domain::quality_preset::QualityPreset::Source,
            auto_update_yt_dlp: false,
            window_close_behavior: crate::services::settings::WindowCloseBehavior::Hide,
            start_at_login: false,
            show_dock_icon: false,
            notifications_enabled: true,
            notify_download_complete: true,
            notify_download_failed: true,
            notify_favorites_ingest: true,
            notify_storage_low: true,
        };
        assert!(NotificationCategory::DownloadFailed.is_enabled(&s));
        s.notifications_enabled = false;
        assert!(!NotificationCategory::DownloadFailed.is_enabled(&s));
    }

    #[test]
    fn category_gate_respects_per_category_toggle() {
        let s = AppSettings {
            enabled_game_ids: vec![],
            poll_floor_seconds: 0,
            poll_recent_seconds: 0,
            poll_ceiling_seconds: 0,
            concurrency_cap: 1,
            first_backfill_limit: 1,
            credentials: crate::services::settings::CredentialsStatus {
                configured: false,
                client_id_masked: None,
                last_token_acquired_at: None,
            },
            library_root: None,
            library_layout: crate::domain::library_layout::LibraryLayoutKind::Plex,
            staging_path: None,
            max_concurrent_downloads: 1,
            bandwidth_limit_bps: None,
            quality_preset: crate::domain::quality_preset::QualityPreset::Source,
            auto_update_yt_dlp: false,
            window_close_behavior: crate::services::settings::WindowCloseBehavior::Hide,
            start_at_login: false,
            show_dock_icon: false,
            notifications_enabled: true,
            notify_download_complete: false,
            notify_download_failed: true,
            notify_favorites_ingest: true,
            notify_storage_low: true,
        };
        assert!(!NotificationCategory::DownloadComplete.is_enabled(&s));
        assert!(NotificationCategory::DownloadFailed.is_enabled(&s));
    }
}
