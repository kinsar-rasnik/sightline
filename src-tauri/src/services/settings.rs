//! Application settings service — game filter, poll interval knobs,
//! and a typed view of the credentials meta row.

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use specta::Type;
use sqlx::Row;

use crate::domain::library_layout::LibraryLayoutKind;
use crate::domain::poll_schedule::PollIntervals;
use crate::domain::quality_preset::QualityPreset;
use crate::error::AppError;
use crate::infra::clock::Clock;
use crate::infra::db::Db;

// `Eq` is intentionally omitted — `completion_threshold: f64` (Phase 6
// housekeeping) doesn't implement total equality. `PartialEq` is what
// the test helpers actually use.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub enabled_game_ids: Vec<String>,
    pub poll_floor_seconds: i64,
    pub poll_recent_seconds: i64,
    pub poll_ceiling_seconds: i64,
    pub concurrency_cap: i64,
    pub first_backfill_limit: i64,
    pub credentials: CredentialsStatus,

    // --- Phase 3: downloads + library layout + storage. ---
    /// Absolute path the user chose as library root. `None` until
    /// the user picks one via the Settings UI folder picker.
    pub library_root: Option<String>,
    pub library_layout: LibraryLayoutKind,
    /// Optional override. `None` means "use the platform default".
    pub staging_path: Option<String>,
    pub max_concurrent_downloads: i64,
    /// `None` = unlimited.
    pub bandwidth_limit_bps: Option<i64>,
    pub quality_preset: QualityPreset,
    pub auto_update_yt_dlp: bool,

    // --- Phase 4: tray daemon + notifications. ---
    pub window_close_behavior: WindowCloseBehavior,
    pub start_at_login: bool,
    pub show_dock_icon: bool,
    pub notifications_enabled: bool,
    pub notify_download_complete: bool,
    pub notify_download_failed: bool,
    pub notify_favorites_ingest: bool,
    pub notify_storage_low: bool,

    // --- Phase 6: watch-progress completion threshold. ---
    /// Fraction in `[0.7, 1.0]` at which the watch-progress state
    /// machine transitions `in_progress → completed`. Persisted in
    /// `app_settings.completion_threshold` (migration 0009). The
    /// column-level CHECK constraint enforces the same bounds, so a
    /// frontend that hand-rolls an out-of-range value gets a SQLite
    /// constraint failure rather than silently skipping the transition.
    pub completion_threshold: f64,
}

/// What happens when the user clicks the window close button.
///
/// - `Hide` (default, new Phase-4 behaviour): the window is hidden,
///   poller + download queue keep running, tray stays on.
/// - `Quit`: explicit quit. Tokio services drain gracefully and the
///   process exits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum WindowCloseBehavior {
    Hide,
    Quit,
}

impl WindowCloseBehavior {
    pub fn as_db_str(self) -> &'static str {
        match self {
            WindowCloseBehavior::Hide => "hide",
            WindowCloseBehavior::Quit => "quit",
        }
    }

    pub fn from_db_str(s: &str) -> Self {
        match s {
            "quit" => WindowCloseBehavior::Quit,
            _ => WindowCloseBehavior::Hide,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CredentialsStatus {
    pub configured: bool,
    pub client_id_masked: Option<String>,
    pub last_token_acquired_at: Option<i64>,
}

/// Partial settings update — any subset may be supplied by the frontend.
///
/// `#[specta(optional)]` on each `Option<T>` field makes tauri-specta
/// emit `T?: T` rather than `T | null` required keys, so the frontend
/// can pass literal `{ enabledGameIds: ... }` objects without having to
/// spread a full "empty patch" baseline. See ADR-0009.
#[derive(Debug, Default, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", default)]
pub struct SettingsPatch {
    #[specta(optional)]
    pub enabled_game_ids: Option<Vec<String>>,
    #[specta(optional)]
    pub poll_floor_seconds: Option<i64>,
    #[specta(optional)]
    pub poll_recent_seconds: Option<i64>,
    #[specta(optional)]
    pub poll_ceiling_seconds: Option<i64>,
    #[specta(optional)]
    pub concurrency_cap: Option<i64>,
    #[specta(optional)]
    pub first_backfill_limit: Option<i64>,

    // --- Phase 3 fields. ---
    #[specta(optional)]
    pub library_root: Option<String>,
    #[specta(optional)]
    pub library_layout: Option<LibraryLayoutKind>,
    #[specta(optional)]
    pub staging_path: Option<String>,
    #[specta(optional)]
    pub max_concurrent_downloads: Option<i64>,
    /// Sentinel: provide `Some(n)` to set a cap, or `Some(-1)` to
    /// clear (unlimited). Omit to leave unchanged.
    #[specta(optional)]
    pub bandwidth_limit_bps: Option<i64>,
    #[specta(optional)]
    pub quality_preset: Option<QualityPreset>,
    #[specta(optional)]
    pub auto_update_yt_dlp: Option<bool>,

    // --- Phase 4 fields. ---
    #[specta(optional)]
    pub window_close_behavior: Option<WindowCloseBehavior>,
    #[specta(optional)]
    pub start_at_login: Option<bool>,
    #[specta(optional)]
    pub show_dock_icon: Option<bool>,
    #[specta(optional)]
    pub notifications_enabled: Option<bool>,
    #[specta(optional)]
    pub notify_download_complete: Option<bool>,
    #[specta(optional)]
    pub notify_download_failed: Option<bool>,
    #[specta(optional)]
    pub notify_favorites_ingest: Option<bool>,
    #[specta(optional)]
    pub notify_storage_low: Option<bool>,

    // --- Phase 6 fields. ---
    #[specta(optional)]
    pub completion_threshold: Option<f64>,
}

#[derive(Debug)]
pub struct SettingsService {
    db: Db,
    clock: Arc<dyn Clock>,
}

impl SettingsService {
    pub fn new(db: Db, clock: Arc<dyn Clock>) -> Self {
        Self { db, clock }
    }

    /// Read the current settings row + credential summary.
    pub async fn get(&self) -> Result<AppSettings, AppError> {
        let row = sqlx::query(
            "SELECT enabled_game_ids_json, poll_floor_seconds, poll_recent_seconds,
                    poll_ceiling_seconds, concurrency_cap, first_backfill_limit,
                    library_root, library_layout, staging_path,
                    max_concurrent_downloads, bandwidth_limit_bps,
                    quality_preset, auto_update_yt_dlp,
                    window_close_behavior, start_at_login, show_dock_icon,
                    notifications_enabled, notify_download_complete,
                    notify_download_failed, notify_favorites_ingest,
                    notify_storage_low,
                    completion_threshold
             FROM app_settings WHERE id = 1",
        )
        .fetch_one(self.db.pool())
        .await?;

        let games_json: String = row.try_get(0)?;
        let enabled_game_ids: Vec<String> =
            serde_json::from_str(&games_json).map_err(AppError::from)?;

        let credentials = self.read_credentials_meta().await?;

        let library_layout_str: String = row.try_get(7)?;
        let library_layout =
            LibraryLayoutKind::from_db_str(&library_layout_str).unwrap_or(LibraryLayoutKind::Plex);
        let quality_preset_str: String = row.try_get(11)?;
        let quality_preset =
            QualityPreset::from_db_str(&quality_preset_str).unwrap_or(QualityPreset::Source);
        let auto_update_raw: i64 = row.try_get(12)?;
        let close_behavior_str: String = row.try_get(13)?;
        let window_close_behavior = WindowCloseBehavior::from_db_str(&close_behavior_str);
        let start_at_login: i64 = row.try_get(14)?;
        let show_dock_icon: i64 = row.try_get(15)?;
        let notifications_enabled: i64 = row.try_get(16)?;
        let notify_download_complete: i64 = row.try_get(17)?;
        let notify_download_failed: i64 = row.try_get(18)?;
        let notify_favorites_ingest: i64 = row.try_get(19)?;
        let notify_storage_low: i64 = row.try_get(20)?;
        let completion_threshold: f64 = row.try_get(21)?;

        Ok(AppSettings {
            enabled_game_ids,
            poll_floor_seconds: row.try_get(1)?,
            poll_recent_seconds: row.try_get(2)?,
            poll_ceiling_seconds: row.try_get(3)?,
            concurrency_cap: row.try_get(4)?,
            first_backfill_limit: row.try_get(5)?,
            credentials,
            library_root: row.try_get(6)?,
            library_layout,
            staging_path: row.try_get(8)?,
            max_concurrent_downloads: row.try_get(9)?,
            bandwidth_limit_bps: row.try_get(10)?,
            quality_preset,
            auto_update_yt_dlp: auto_update_raw != 0,
            window_close_behavior,
            start_at_login: start_at_login != 0,
            show_dock_icon: show_dock_icon != 0,
            notifications_enabled: notifications_enabled != 0,
            notify_download_complete: notify_download_complete != 0,
            notify_download_failed: notify_download_failed != 0,
            notify_favorites_ingest: notify_favorites_ingest != 0,
            notify_storage_low: notify_storage_low != 0,
            completion_threshold,
        })
    }

    /// Apply a partial update. Values not supplied retain their current
    /// row value; normalization enforces poll-interval monotonicity and
    /// clamps ridiculous values.
    pub async fn update(&self, patch: SettingsPatch) -> Result<AppSettings, AppError> {
        let current = self.get().await?;
        let desired_intervals = PollIntervals {
            floor_seconds: patch
                .poll_floor_seconds
                .unwrap_or(current.poll_floor_seconds),
            recent_seconds: patch
                .poll_recent_seconds
                .unwrap_or(current.poll_recent_seconds),
            ceiling_seconds: patch
                .poll_ceiling_seconds
                .unwrap_or(current.poll_ceiling_seconds),
        }
        .normalized();

        let games = patch
            .enabled_game_ids
            .clone()
            .unwrap_or(current.enabled_game_ids);
        let concurrency = patch
            .concurrency_cap
            .unwrap_or(current.concurrency_cap)
            .clamp(1, 16);
        let backfill = patch
            .first_backfill_limit
            .unwrap_or(current.first_backfill_limit)
            .clamp(1, 500);

        let library_root = match patch.library_root.clone() {
            Some(root) => {
                // Reject obviously-unsafe roots before we persist. The
                // disk-preflight and atomic-move flows rely on the
                // root being a real directory that is NOT the
                // filesystem root.
                validate_library_root(&root)?;
                Some(root)
            }
            None => current.library_root,
        };
        let library_layout = patch.library_layout.unwrap_or(current.library_layout);
        let staging_path = match patch.staging_path.clone() {
            Some(path) => {
                validate_staging_override(&path, library_root.as_deref())?;
                Some(path)
            }
            None => current.staging_path,
        };
        let max_concurrent = patch
            .max_concurrent_downloads
            .unwrap_or(current.max_concurrent_downloads)
            .clamp(1, 5);
        // `-1` sentinel means "clear the cap" (unlimited).
        let bandwidth_limit_bps = match patch.bandwidth_limit_bps {
            Some(-1) => None,
            Some(n) if n > 0 => Some(n),
            Some(_) => current.bandwidth_limit_bps,
            None => current.bandwidth_limit_bps,
        };
        let quality_preset = patch.quality_preset.unwrap_or(current.quality_preset);
        let auto_update_yt_dlp = patch
            .auto_update_yt_dlp
            .unwrap_or(current.auto_update_yt_dlp);

        let window_close_behavior = patch
            .window_close_behavior
            .unwrap_or(current.window_close_behavior);
        let start_at_login = patch.start_at_login.unwrap_or(current.start_at_login);
        let show_dock_icon = patch.show_dock_icon.unwrap_or(current.show_dock_icon);
        let notifications_enabled = patch
            .notifications_enabled
            .unwrap_or(current.notifications_enabled);
        let notify_download_complete = patch
            .notify_download_complete
            .unwrap_or(current.notify_download_complete);
        let notify_download_failed = patch
            .notify_download_failed
            .unwrap_or(current.notify_download_failed);
        let notify_favorites_ingest = patch
            .notify_favorites_ingest
            .unwrap_or(current.notify_favorites_ingest);
        let notify_storage_low = patch
            .notify_storage_low
            .unwrap_or(current.notify_storage_low);
        // Mirrors the column-level CHECK on `app_settings.completion_threshold`
        // — clamp client-side so a Settings UI slider that briefly slips out
        // of bounds still produces a writeable row instead of a SQLite
        // constraint failure surfaced to the user.
        let completion_threshold = patch
            .completion_threshold
            .unwrap_or(current.completion_threshold)
            .clamp(0.7, 1.0);

        let games_json = serde_json::to_string(&games).map_err(AppError::from)?;
        let now = self.clock.unix_seconds();

        sqlx::query(
            "UPDATE app_settings
             SET enabled_game_ids_json = ?,
                 poll_floor_seconds = ?,
                 poll_recent_seconds = ?,
                 poll_ceiling_seconds = ?,
                 concurrency_cap = ?,
                 first_backfill_limit = ?,
                 library_root = ?,
                 library_layout = ?,
                 staging_path = ?,
                 max_concurrent_downloads = ?,
                 bandwidth_limit_bps = ?,
                 quality_preset = ?,
                 auto_update_yt_dlp = ?,
                 window_close_behavior = ?,
                 start_at_login = ?,
                 show_dock_icon = ?,
                 notifications_enabled = ?,
                 notify_download_complete = ?,
                 notify_download_failed = ?,
                 notify_favorites_ingest = ?,
                 notify_storage_low = ?,
                 completion_threshold = ?,
                 updated_at = ?
             WHERE id = 1",
        )
        .bind(&games_json)
        .bind(desired_intervals.floor_seconds)
        .bind(desired_intervals.recent_seconds)
        .bind(desired_intervals.ceiling_seconds)
        .bind(concurrency)
        .bind(backfill)
        .bind(&library_root)
        .bind(library_layout.as_db_str())
        .bind(&staging_path)
        .bind(max_concurrent)
        .bind(bandwidth_limit_bps)
        .bind(quality_preset.as_db_str())
        .bind(if auto_update_yt_dlp { 1 } else { 0 })
        .bind(window_close_behavior.as_db_str())
        .bind(if start_at_login { 1 } else { 0 })
        .bind(if show_dock_icon { 1 } else { 0 })
        .bind(if notifications_enabled { 1 } else { 0 })
        .bind(if notify_download_complete { 1 } else { 0 })
        .bind(if notify_download_failed { 1 } else { 0 })
        .bind(if notify_favorites_ingest { 1 } else { 0 })
        .bind(if notify_storage_low { 1 } else { 0 })
        .bind(completion_threshold)
        .bind(now)
        .execute(self.db.pool())
        .await?;

        self.get().await
    }

    async fn read_credentials_meta(&self) -> Result<CredentialsStatus, AppError> {
        let row = sqlx::query(
            "SELECT configured, client_id_masked, last_token_acquired_at
             FROM credentials_meta WHERE id = 1",
        )
        .fetch_one(self.db.pool())
        .await?;

        let configured: i64 = row.try_get(0)?;
        Ok(CredentialsStatus {
            configured: configured != 0,
            client_id_masked: row.try_get(1)?,
            last_token_acquired_at: row.try_get(2)?,
        })
    }

    /// Write-side update used by the credentials service after a
    /// successful save / clear.
    pub(crate) async fn set_credentials_meta(
        &self,
        status: &CredentialsStatus,
    ) -> Result<(), AppError> {
        let now = self.clock.unix_seconds();
        sqlx::query(
            "UPDATE credentials_meta
             SET configured = ?,
                 client_id_masked = ?,
                 last_token_acquired_at = ?,
                 updated_at = ?
             WHERE id = 1",
        )
        .bind(if status.configured { 1 } else { 0 })
        .bind(&status.client_id_masked)
        .bind(status.last_token_acquired_at)
        .bind(now)
        .execute(self.db.pool())
        .await?;
        Ok(())
    }

    /// Shortcut used by the poller — returns the interval knobs as a
    /// typed value.
    pub fn intervals_from(settings: &AppSettings) -> PollIntervals {
        PollIntervals {
            floor_seconds: settings.poll_floor_seconds,
            recent_seconds: settings.poll_recent_seconds,
            ceiling_seconds: settings.poll_ceiling_seconds,
        }
        .normalized()
    }
}

/// Validate a proposed `library_root` value before persisting it.
///
/// A malicious or buggy frontend could otherwise hand us "/" or
/// "C:\" and cause every subsequent atomic move to write into the
/// filesystem root. See the ADR-0012 security posture.
fn validate_library_root(raw: &str) -> Result<(), AppError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(AppError::InvalidInput {
            detail: "library_root is empty".into(),
        });
    }
    let path = std::path::Path::new(trimmed);
    if !path.is_absolute() {
        return Err(AppError::InvalidInput {
            detail: "library_root must be an absolute path".into(),
        });
    }
    // Reject a filesystem root; a user picking "/" (or "C:\") via the
    // folder picker would otherwise mean the library migrator and
    // atomic-move flows scribble across the entire disk.
    if path.parent().is_none() || path == std::path::Path::new("/") {
        return Err(AppError::InvalidInput {
            detail: "library_root must not be the filesystem root".into(),
        });
    }
    Ok(())
}

/// Validate a proposed `staging_path` override. Must be absolute,
/// cannot live inside `library_root` (would defeat the atomic-move
/// design).
fn validate_staging_override(raw: &str, library_root: Option<&str>) -> Result<(), AppError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(AppError::InvalidInput {
            detail: "staging_path is empty".into(),
        });
    }
    let path = std::path::Path::new(trimmed);
    if !path.is_absolute() {
        return Err(AppError::InvalidInput {
            detail: "staging_path must be an absolute path".into(),
        });
    }
    if let Some(root) = library_root {
        let root_path = std::path::Path::new(root);
        if path.starts_with(root_path) {
            return Err(AppError::InvalidInput {
                detail: "staging_path must not be under library_root".into(),
            });
        }
    }
    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::infra::clock::FixedClock;

    async fn setup() -> SettingsService {
        let db = Db::open_in_memory().await.unwrap();
        db.migrate().await.unwrap();
        SettingsService::new(db, Arc::new(FixedClock::at(1_000_000)))
    }

    #[tokio::test]
    async fn get_returns_seeded_defaults() {
        let svc = setup().await;
        let s = svc.get().await.unwrap();
        assert_eq!(s.enabled_game_ids, vec!["32982"]);
        assert_eq!(s.poll_floor_seconds, 600);
        assert!(!s.credentials.configured);
    }

    #[tokio::test]
    async fn update_normalizes_intervals() {
        let svc = setup().await;
        let out = svc
            .update(SettingsPatch {
                poll_floor_seconds: Some(120),
                poll_recent_seconds: Some(60),
                poll_ceiling_seconds: Some(30),
                ..Default::default()
            })
            .await
            .unwrap();
        assert!(out.poll_floor_seconds <= out.poll_recent_seconds);
        assert!(out.poll_recent_seconds <= out.poll_ceiling_seconds);
    }

    #[tokio::test]
    async fn update_respects_game_filter_override() {
        let svc = setup().await;
        let out = svc
            .update(SettingsPatch {
                enabled_game_ids: Some(vec!["32982".into(), "516575".into()]),
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(out.enabled_game_ids, vec!["32982", "516575"]);
    }

    #[tokio::test]
    async fn update_clamps_concurrency_cap_and_backfill() {
        let svc = setup().await;
        let out = svc
            .update(SettingsPatch {
                concurrency_cap: Some(9_999),
                first_backfill_limit: Some(-5),
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(out.concurrency_cap, 16);
        assert_eq!(out.first_backfill_limit, 1);
    }

    /// Path string that `std::path::Path::is_absolute` accepts on the
    /// host — `/tmp/...` on Unix, `C:\tmp\...` on Windows.
    fn abs(rel: &str) -> String {
        if cfg!(windows) {
            format!(r"C:\tmp\{}", rel.trim_start_matches('/'))
        } else {
            format!("/tmp/{}", rel.trim_start_matches('/'))
        }
    }

    #[tokio::test]
    async fn phase_3_fields_have_sensible_defaults() {
        let svc = setup().await;
        let s = svc.get().await.unwrap();
        assert_eq!(s.library_root, None);
        assert_eq!(s.library_layout, LibraryLayoutKind::Plex);
        assert_eq!(s.staging_path, None);
        assert_eq!(s.max_concurrent_downloads, 2);
        assert_eq!(s.bandwidth_limit_bps, None);
        assert_eq!(s.quality_preset, QualityPreset::Source);
        assert!(s.auto_update_yt_dlp);
    }

    #[tokio::test]
    async fn update_writes_phase_3_fields() {
        let svc = setup().await;
        let root = abs("lib");
        let out = svc
            .update(SettingsPatch {
                library_root: Some(root.clone()),
                library_layout: Some(LibraryLayoutKind::Flat),
                max_concurrent_downloads: Some(3),
                bandwidth_limit_bps: Some(5_000_000),
                quality_preset: Some(QualityPreset::P720p60),
                auto_update_yt_dlp: Some(false),
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(out.library_root.as_deref(), Some(root.as_str()));
        assert_eq!(out.library_layout, LibraryLayoutKind::Flat);
        assert_eq!(out.max_concurrent_downloads, 3);
        assert_eq!(out.bandwidth_limit_bps, Some(5_000_000));
        assert_eq!(out.quality_preset, QualityPreset::P720p60);
        assert!(!out.auto_update_yt_dlp);
    }

    #[tokio::test]
    async fn bandwidth_minus_one_clears_cap() {
        let svc = setup().await;
        svc.update(SettingsPatch {
            bandwidth_limit_bps: Some(1_000_000),
            ..Default::default()
        })
        .await
        .unwrap();
        let out = svc
            .update(SettingsPatch {
                bandwidth_limit_bps: Some(-1),
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(out.bandwidth_limit_bps, None);
    }

    #[tokio::test]
    async fn update_rejects_empty_or_relative_library_root() {
        let svc = setup().await;
        let err = svc
            .update(SettingsPatch {
                library_root: Some(String::new()),
                ..Default::default()
            })
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::InvalidInput { .. }));
        let err = svc
            .update(SettingsPatch {
                library_root: Some("relative/path".into()),
                ..Default::default()
            })
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::InvalidInput { .. }));
    }

    #[tokio::test]
    #[cfg(not(windows))]
    async fn update_rejects_filesystem_root_as_library_root() {
        // Windows path semantics differ (a path like "/" is treated as
        // non-absolute by `Path::is_absolute`, so it gets caught by
        // the earlier "must be absolute" arm). The Unix assertion is
        // the one we want to pin.
        let svc = setup().await;
        let err = svc
            .update(SettingsPatch {
                library_root: Some("/".into()),
                ..Default::default()
            })
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::InvalidInput { .. }));
    }

    #[tokio::test]
    async fn update_rejects_staging_under_library_root() {
        let svc = setup().await;
        let root = abs("lib");
        svc.update(SettingsPatch {
            library_root: Some(root.clone()),
            ..Default::default()
        })
        .await
        .unwrap();
        let separator = if cfg!(windows) { '\\' } else { '/' };
        let nested = format!("{root}{separator}staging");
        let err = svc
            .update(SettingsPatch {
                staging_path: Some(nested),
                ..Default::default()
            })
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::InvalidInput { .. }));
    }

    #[tokio::test]
    async fn update_accepts_absolute_non_nested_staging() {
        let svc = setup().await;
        svc.update(SettingsPatch {
            library_root: Some(abs("lib")),
            staging_path: Some(abs("staging")),
            ..Default::default()
        })
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn completion_threshold_defaults_to_zero_point_nine() {
        let svc = setup().await;
        let s = svc.get().await.unwrap();
        assert!((s.completion_threshold - 0.9).abs() < f64::EPSILON);
    }

    #[tokio::test]
    async fn completion_threshold_clamped_to_seven_to_ten_tenths() {
        let svc = setup().await;
        let high = svc
            .update(SettingsPatch {
                completion_threshold: Some(2.0),
                ..Default::default()
            })
            .await
            .unwrap();
        assert!((high.completion_threshold - 1.0).abs() < f64::EPSILON);
        let low = svc
            .update(SettingsPatch {
                completion_threshold: Some(0.1),
                ..Default::default()
            })
            .await
            .unwrap();
        assert!((low.completion_threshold - 0.7).abs() < f64::EPSILON);
    }

    #[tokio::test]
    async fn completion_threshold_round_trips_in_range() {
        let svc = setup().await;
        let out = svc
            .update(SettingsPatch {
                completion_threshold: Some(0.85),
                ..Default::default()
            })
            .await
            .unwrap();
        assert!((out.completion_threshold - 0.85).abs() < f64::EPSILON);
    }

    #[tokio::test]
    async fn max_concurrent_downloads_clamped_to_1_5() {
        let svc = setup().await;
        let high = svc
            .update(SettingsPatch {
                max_concurrent_downloads: Some(99),
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(high.max_concurrent_downloads, 5);
        let low = svc
            .update(SettingsPatch {
                max_concurrent_downloads: Some(0),
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(low.max_concurrent_downloads, 1);
    }
}
