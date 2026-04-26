//! Application settings service — game filter, poll interval knobs,
//! and a typed view of the credentials meta row.

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use specta::Type;
use sqlx::Row;

use crate::domain::distribution::DistributionMode;
use crate::domain::library_layout::LibraryLayoutKind;
use crate::domain::poll_schedule::PollIntervals;
use crate::domain::quality::{EncoderCapability, VideoQualityProfile};
use crate::domain::quality_preset::QualityPreset;
use crate::domain::sync::SyncLayout;
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

    // --- Phase 6: multi-view sync engine knobs. ---
    /// Drift tolerance in milliseconds for the multi-view sync loop.
    /// Persisted in `app_settings.sync_drift_threshold_ms` (migration
    /// 0011); the column-level CHECK enforces `[50.0, 1000.0]`.  See
    /// ADR-0022 for the rationale on the default of 250 ms.
    pub sync_drift_threshold_ms: f64,
    /// Layout the multi-view page mounts when the user opens a fresh
    /// session.  v1 only ships `Split5050`.
    pub sync_default_layout: SyncLayout,
    /// Strategy for picking the leader pane when a session starts.
    /// `'first-opened'` selects pane index 0 (the primary VOD the
    /// user clicked).  Modeled as a string rather than an enum so v2
    /// can add `'longest'` etc. without an IPC contract bump.
    pub sync_default_leader: String,

    // --- Phase 7: auto-cleanup. ---
    /// Master toggle for the auto-cleanup service. Default off
    /// (ADR-0024); the UI confirms the plan before flipping this on.
    pub cleanup_enabled: bool,
    /// Used-fraction threshold above which the scheduled tick runs
    /// (default 0.9, range 0.5..=0.99).
    pub cleanup_high_watermark: f64,
    /// Used-fraction stop threshold the cleanup run targets
    /// (default 0.75, range 0.4..=0.95). Service-layer write enforces
    /// `cleanup_low_watermark < cleanup_high_watermark`.
    pub cleanup_low_watermark: f64,
    /// Local hour of day at which the scheduled tick fires
    /// (default 3, range 0..=23).
    pub cleanup_schedule_hour: i64,

    // --- Phase 7: update checker. ---
    /// Master toggle for the GitHub Releases update checker.
    /// Default off (ADR-0026, privacy posture).
    pub update_check_enabled: bool,
    /// Wall-clock seconds at which the daily check most recently ran
    /// (any outcome).  None when the user has just enabled the
    /// feature.
    pub update_check_last_run: Option<i64>,
    /// Tag the user explicitly suppressed via "Skip this version".
    /// Empty / cleared when the user clicks "Don't skip".
    pub update_check_skip_version: Option<String>,

    // --- Phase 8: quality pipeline (ADR-0028, ADR-0029). ---
    /// Chosen video-quality profile.  Default `'720p30'` for new
    /// installs.  Persisted in `app_settings.video_quality_profile`
    /// (migration 0015).  Distinct from the legacy `quality_preset`
    /// field which is preserved for backwards-compat with v1.0
    /// installs that already have a value there.
    pub video_quality_profile: VideoQualityProfile,
    /// User opt-in for the libx265 / libx264 software fallback path.
    /// Default false; the encoder-detection pass surfaces a warning
    /// when no hardware encoder is available and this is off.
    pub software_encode_opt_in: bool,
    /// Detection result from `services::encoder_detection`.  `None`
    /// when detection has never run (cold start or after the user
    /// clicked "Re-detect" and the call hasn't yet completed).
    pub encoder_capability: Option<EncoderCapability>,
    /// Concurrency cap on background re-encodes.  Default 1, hard
    /// clamped to 1..=2.
    pub max_concurrent_reencodes: i64,
    /// CPU-load fraction above which a sustained burst pauses the
    /// in-flight ffmpeg encoder (ADR-0029).
    pub cpu_throttle_high_threshold: f64,
    /// CPU-load fraction below which a sustained idle period resumes
    /// a paused encoder.  Strictly less than `cpu_throttle_high_threshold`
    /// (service-layer rejects an inverted pair).
    pub cpu_throttle_low_threshold: f64,

    // --- Phase 8: distribution model (ADR-0030, ADR-0031). ---
    /// Default `Pull` for new installs; existing v1.0 installs are
    /// pinned to `Auto` by migration 0017.
    pub distribution_mode: DistributionMode,
    /// Per-streamer cap on `(queued + downloading + ready)` rows.
    /// Default 2; range [1, 20].
    pub sliding_window_size: i64,
    /// Whether the player's prefetch hook (ADR-0031) is allowed to
    /// auto-pick the next VOD.  Defaults to true; off = strict
    /// pull-only, the user picks every VOD by hand.
    pub prefetch_enabled: bool,
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
    #[specta(optional)]
    pub sync_drift_threshold_ms: Option<f64>,
    #[specta(optional)]
    pub sync_default_layout: Option<SyncLayout>,
    #[specta(optional)]
    pub sync_default_leader: Option<String>,

    // --- Phase 7 fields. ---
    #[specta(optional)]
    pub cleanup_enabled: Option<bool>,
    #[specta(optional)]
    pub cleanup_high_watermark: Option<f64>,
    #[specta(optional)]
    pub cleanup_low_watermark: Option<f64>,
    #[specta(optional)]
    pub cleanup_schedule_hour: Option<i64>,
    #[specta(optional)]
    pub update_check_enabled: Option<bool>,
    /// Pass an empty string to clear; pass `Some("v1.2.3")` to skip
    /// that release.  Omit to leave unchanged.
    #[specta(optional)]
    pub update_check_skip_version: Option<String>,

    // --- Phase 8 fields. ---
    #[specta(optional)]
    pub video_quality_profile: Option<VideoQualityProfile>,
    #[specta(optional)]
    pub software_encode_opt_in: Option<bool>,
    #[specta(optional)]
    pub max_concurrent_reencodes: Option<i64>,
    #[specta(optional)]
    pub cpu_throttle_high_threshold: Option<f64>,
    #[specta(optional)]
    pub cpu_throttle_low_threshold: Option<f64>,

    #[specta(optional)]
    pub distribution_mode: Option<DistributionMode>,
    #[specta(optional)]
    pub sliding_window_size: Option<i64>,
    #[specta(optional)]
    pub prefetch_enabled: Option<bool>,
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
                    completion_threshold,
                    sync_drift_threshold_ms, sync_default_layout, sync_default_leader,
                    cleanup_enabled, cleanup_high_watermark, cleanup_low_watermark,
                    cleanup_schedule_hour,
                    update_check_enabled, update_check_last_run, update_check_skip_version,
                    video_quality_profile, software_encode_opt_in, encoder_capability,
                    max_concurrent_reencodes,
                    cpu_throttle_high_threshold, cpu_throttle_low_threshold,
                    distribution_mode, sliding_window_size, prefetch_enabled
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
        let sync_drift_threshold_ms: f64 = row.try_get(22)?;
        let sync_default_layout_str: String = row.try_get(23)?;
        let sync_default_layout =
            SyncLayout::from_db_str(&sync_default_layout_str).unwrap_or(SyncLayout::Split5050);
        let sync_default_leader: String = row.try_get(24)?;
        let cleanup_enabled_raw: i64 = row.try_get(25)?;
        let cleanup_high_watermark: f64 = row.try_get(26)?;
        let cleanup_low_watermark: f64 = row.try_get(27)?;
        let cleanup_schedule_hour: i64 = row.try_get(28)?;
        let update_check_enabled_raw: i64 = row.try_get(29)?;
        let update_check_last_run: Option<i64> = row.try_get(30)?;
        let update_check_skip_version: Option<String> = row.try_get(31)?;
        let video_quality_profile_str: String = row.try_get(32)?;
        let video_quality_profile = VideoQualityProfile::from_db_str(&video_quality_profile_str)
            .unwrap_or(VideoQualityProfile::P720p30);
        let software_encode_opt_in_raw: i64 = row.try_get(33)?;
        let encoder_capability_json: Option<String> = row.try_get(34)?;
        let encoder_capability = match encoder_capability_json {
            Some(json) => match serde_json::from_str(&json) {
                Ok(cap) => Some(cap),
                Err(e) => {
                    // Log + clear-on-read so a corrupt blob (schema
                    // drift, bad hand-edit) re-triggers detection
                    // on next startup rather than silently disabling
                    // hardware encoding.  See R-RC-01 finding P2 on
                    // commit 94e4340.
                    tracing::warn!(
                        error = %e,
                        "encoder_capability JSON corrupt — clearing in-memory copy"
                    );
                    None
                }
            },
            None => None,
        };
        let max_concurrent_reencodes: i64 = row.try_get(35)?;
        let cpu_throttle_high_threshold: f64 = row.try_get(36)?;
        let cpu_throttle_low_threshold: f64 = row.try_get(37)?;
        let distribution_mode_str: String = row.try_get(38)?;
        let distribution_mode =
            DistributionMode::from_db_str(&distribution_mode_str).unwrap_or(DistributionMode::Pull);
        let sliding_window_size: i64 = row.try_get(39)?;
        let prefetch_enabled_raw: i64 = row.try_get(40)?;

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
            sync_drift_threshold_ms,
            sync_default_layout,
            sync_default_leader,
            cleanup_enabled: cleanup_enabled_raw != 0,
            cleanup_high_watermark,
            cleanup_low_watermark,
            cleanup_schedule_hour,
            update_check_enabled: update_check_enabled_raw != 0,
            update_check_last_run,
            update_check_skip_version,
            video_quality_profile,
            software_encode_opt_in: software_encode_opt_in_raw != 0,
            encoder_capability,
            max_concurrent_reencodes,
            cpu_throttle_high_threshold,
            cpu_throttle_low_threshold,
            distribution_mode,
            sliding_window_size,
            prefetch_enabled: prefetch_enabled_raw != 0,
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
        // v2.0.3: hard-clamp to 1..=3.  Values above 3 historically
        // produced fragment-rename races on Twitch under load
        // (the actual cause of the v2.0.2 download-engine bug)
        // because each yt-dlp worker contends for the same staging
        // dir.  See ADR-0035.
        let max_concurrent = patch
            .max_concurrent_downloads
            .unwrap_or(current.max_concurrent_downloads)
            .clamp(1, 3);
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

        // Mirrors the [50, 1000] CHECK on `sync_drift_threshold_ms` (migration
        // 0011 / ADR-0022).  Clamp client-side so a Settings slider can't
        // end up with a SQLite constraint failure surfaced to the user.
        let sync_drift_threshold_ms = patch
            .sync_drift_threshold_ms
            .unwrap_or(current.sync_drift_threshold_ms)
            .clamp(50.0, 1000.0);
        let sync_default_layout = patch
            .sync_default_layout
            .unwrap_or(current.sync_default_layout);
        // The column-level CHECK enforces `'first-opened'` for v1; we
        // validate the input matches the vocabulary so an unknown
        // string fails fast with a typed error rather than at the
        // SQLite layer.
        let sync_default_leader = match patch.sync_default_leader {
            Some(value) => {
                if value != "first-opened" {
                    return Err(AppError::InvalidInput {
                        detail: format!("unsupported sync_default_leader '{value}'"),
                    });
                }
                value
            }
            None => current.sync_default_leader,
        };

        // --- Phase 7 cleanup knobs. ---
        let cleanup_enabled = patch.cleanup_enabled.unwrap_or(current.cleanup_enabled);
        let cleanup_high_watermark = patch
            .cleanup_high_watermark
            .unwrap_or(current.cleanup_high_watermark)
            .clamp(0.5, 0.99);
        let cleanup_low_watermark = patch
            .cleanup_low_watermark
            .unwrap_or(current.cleanup_low_watermark)
            .clamp(0.4, 0.95);
        if cleanup_low_watermark >= cleanup_high_watermark {
            return Err(AppError::InvalidInput {
                detail: format!(
                    "cleanup_low_watermark ({cleanup_low_watermark:.2}) must be < cleanup_high_watermark ({cleanup_high_watermark:.2})"
                ),
            });
        }
        let cleanup_schedule_hour = patch
            .cleanup_schedule_hour
            .unwrap_or(current.cleanup_schedule_hour)
            .clamp(0, 23);

        // --- Phase 7 updater knobs.  ---
        let update_check_enabled = patch
            .update_check_enabled
            .unwrap_or(current.update_check_enabled);
        let update_check_skip_version = match patch.update_check_skip_version {
            Some(value) => {
                if value.is_empty() {
                    None
                } else {
                    Some(value)
                }
            }
            None => current.update_check_skip_version.clone(),
        };

        // --- Phase 8 quality pipeline knobs (ADR-0028, ADR-0029). ---
        let video_quality_profile = patch
            .video_quality_profile
            .unwrap_or(current.video_quality_profile);
        let software_encode_opt_in = patch
            .software_encode_opt_in
            .unwrap_or(current.software_encode_opt_in);
        let max_concurrent_reencodes = patch
            .max_concurrent_reencodes
            .unwrap_or(current.max_concurrent_reencodes)
            .clamp(1, 2);
        let cpu_throttle_high_threshold = patch
            .cpu_throttle_high_threshold
            .unwrap_or(current.cpu_throttle_high_threshold)
            .clamp(0.5, 0.9);
        let cpu_throttle_low_threshold = patch
            .cpu_throttle_low_threshold
            .unwrap_or(current.cpu_throttle_low_threshold)
            .clamp(0.3, 0.8);
        // Anti-thrash: low must be at least 5 percentage points
        // below high. Mirrors `ThrottleThresholds::is_well_formed`
        // and the pattern used by `cleanup_low_watermark < cleanup_high_watermark`.
        if cpu_throttle_high_threshold - cpu_throttle_low_threshold < 0.05 {
            return Err(AppError::InvalidInput {
                detail: format!(
                    "cpu_throttle_low_threshold ({cpu_throttle_low_threshold:.2}) must be at least 0.05 below cpu_throttle_high_threshold ({cpu_throttle_high_threshold:.2})"
                ),
            });
        }

        // --- Phase 8 distribution knobs (ADR-0030, ADR-0031). ---
        let distribution_mode = patch.distribution_mode.unwrap_or(current.distribution_mode);
        let sliding_window_size = patch
            .sliding_window_size
            .unwrap_or(current.sliding_window_size)
            .clamp(1, 20);
        let prefetch_enabled = patch.prefetch_enabled.unwrap_or(current.prefetch_enabled);

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
                 sync_drift_threshold_ms = ?,
                 sync_default_layout = ?,
                 sync_default_leader = ?,
                 cleanup_enabled = ?,
                 cleanup_high_watermark = ?,
                 cleanup_low_watermark = ?,
                 cleanup_schedule_hour = ?,
                 update_check_enabled = ?,
                 update_check_skip_version = ?,
                 video_quality_profile = ?,
                 software_encode_opt_in = ?,
                 max_concurrent_reencodes = ?,
                 cpu_throttle_high_threshold = ?,
                 cpu_throttle_low_threshold = ?,
                 distribution_mode = ?,
                 sliding_window_size = ?,
                 prefetch_enabled = ?,
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
        .bind(sync_drift_threshold_ms)
        .bind(sync_default_layout.as_db_str())
        .bind(&sync_default_leader)
        .bind(if cleanup_enabled { 1 } else { 0 })
        .bind(cleanup_high_watermark)
        .bind(cleanup_low_watermark)
        .bind(cleanup_schedule_hour)
        .bind(if update_check_enabled { 1 } else { 0 })
        .bind(&update_check_skip_version)
        .bind(video_quality_profile.as_db_str())
        .bind(if software_encode_opt_in { 1 } else { 0 })
        .bind(max_concurrent_reencodes)
        .bind(cpu_throttle_high_threshold)
        .bind(cpu_throttle_low_threshold)
        .bind(distribution_mode.as_db_str())
        .bind(sliding_window_size)
        .bind(if prefetch_enabled { 1 } else { 0 })
        .bind(now)
        .execute(self.db.pool())
        .await?;

        self.get().await
    }

    /// Persist the encoder-capability JSON detected by
    /// [`crate::services::encoder_detection::EncoderDetectionService`].
    /// Pure write — does not touch any other column, so it stays
    /// outside the main `update` flow which would otherwise need a
    /// new SettingsPatch field for an internal-only blob.
    pub async fn record_encoder_capability(
        &self,
        capability: &EncoderCapability,
    ) -> Result<(), AppError> {
        let json = serde_json::to_string(capability).map_err(AppError::from)?;
        let now = self.clock.unix_seconds();
        sqlx::query(
            "UPDATE app_settings
                SET encoder_capability = ?, updated_at = ?
              WHERE id = 1",
        )
        .bind(&json)
        .bind(now)
        .execute(self.db.pool())
        .await?;
        Ok(())
    }

    /// Persist the timestamp of the last update-check tick (any
    /// outcome). Used by `UpdaterService` to enforce the once-per-day
    /// gate without going through the full settings-update path.
    pub async fn record_update_check_run(&self, when: i64) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE app_settings
                SET update_check_last_run = ?, updated_at = ?
              WHERE id = 1",
        )
        .bind(when)
        .bind(self.clock.unix_seconds())
        .execute(self.db.pool())
        .await?;
        Ok(())
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
    async fn sync_settings_have_documented_defaults() {
        let svc = setup().await;
        let s = svc.get().await.unwrap();
        assert!((s.sync_drift_threshold_ms - 250.0).abs() < f64::EPSILON);
        assert_eq!(s.sync_default_layout, SyncLayout::Split5050);
        assert_eq!(s.sync_default_leader, "first-opened");
    }

    #[tokio::test]
    async fn sync_drift_threshold_clamped_to_50_to_1000() {
        let svc = setup().await;
        let high = svc
            .update(SettingsPatch {
                sync_drift_threshold_ms: Some(5_000.0),
                ..Default::default()
            })
            .await
            .unwrap();
        assert!((high.sync_drift_threshold_ms - 1_000.0).abs() < f64::EPSILON);
        let low = svc
            .update(SettingsPatch {
                sync_drift_threshold_ms: Some(10.0),
                ..Default::default()
            })
            .await
            .unwrap();
        assert!((low.sync_drift_threshold_ms - 50.0).abs() < f64::EPSILON);
    }

    #[tokio::test]
    async fn sync_drift_threshold_round_trips_in_range() {
        let svc = setup().await;
        let out = svc
            .update(SettingsPatch {
                sync_drift_threshold_ms: Some(180.0),
                ..Default::default()
            })
            .await
            .unwrap();
        assert!((out.sync_drift_threshold_ms - 180.0).abs() < f64::EPSILON);
    }

    #[tokio::test]
    async fn unsupported_sync_default_leader_is_rejected() {
        let svc = setup().await;
        let err = svc
            .update(SettingsPatch {
                sync_default_leader: Some("random".to_string()),
                ..Default::default()
            })
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::InvalidInput { .. }));
    }

    #[tokio::test]
    async fn max_concurrent_downloads_clamped_to_1_3() {
        // v2.0.3: cap tightened from 1..=5 to 1..=3 (ADR-0035).
        let svc = setup().await;
        let high = svc
            .update(SettingsPatch {
                max_concurrent_downloads: Some(99),
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(high.max_concurrent_downloads, 3);
        let low = svc
            .update(SettingsPatch {
                max_concurrent_downloads: Some(0),
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(low.max_concurrent_downloads, 1);
    }

    #[tokio::test]
    async fn max_concurrent_downloads_inrange_round_trips() {
        // 1, 2, 3 must all be writable verbatim — the hard cap of 3
        // is a ceiling, not a forced default.  This pins the AC2
        // contract that an existing user with value 2 (CEO context)
        // doesn't get silently flattened.
        let svc = setup().await;
        for v in 1..=3 {
            let out = svc
                .update(SettingsPatch {
                    max_concurrent_downloads: Some(v),
                    ..Default::default()
                })
                .await
                .unwrap();
            assert_eq!(out.max_concurrent_downloads, v);
        }
    }

    #[tokio::test]
    async fn phase_7_cleanup_defaults() {
        let svc = setup().await;
        let s = svc.get().await.unwrap();
        assert!(!s.cleanup_enabled);
        assert!((s.cleanup_high_watermark - 0.9).abs() < f64::EPSILON);
        assert!((s.cleanup_low_watermark - 0.75).abs() < f64::EPSILON);
        assert_eq!(s.cleanup_schedule_hour, 3);
    }

    #[tokio::test]
    async fn cleanup_low_must_be_less_than_high() {
        let svc = setup().await;
        let err = svc
            .update(SettingsPatch {
                cleanup_low_watermark: Some(0.95),
                cleanup_high_watermark: Some(0.9),
                ..Default::default()
            })
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::InvalidInput { .. }));
    }

    #[tokio::test]
    async fn cleanup_high_watermark_clamped_to_50_99_pct() {
        let svc = setup().await;
        let high = svc
            .update(SettingsPatch {
                cleanup_high_watermark: Some(2.0),
                ..Default::default()
            })
            .await
            .unwrap();
        assert!((high.cleanup_high_watermark - 0.99).abs() < f64::EPSILON);
        let low = svc
            .update(SettingsPatch {
                cleanup_high_watermark: Some(0.1),
                cleanup_low_watermark: Some(0.4),
                ..Default::default()
            })
            .await
            .unwrap();
        assert!((low.cleanup_high_watermark - 0.5).abs() < f64::EPSILON);
    }

    #[tokio::test]
    async fn cleanup_schedule_hour_clamped_to_0_23() {
        let svc = setup().await;
        let high = svc
            .update(SettingsPatch {
                cleanup_schedule_hour: Some(99),
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(high.cleanup_schedule_hour, 23);
        let low = svc
            .update(SettingsPatch {
                cleanup_schedule_hour: Some(-5),
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(low.cleanup_schedule_hour, 0);
    }

    #[tokio::test]
    async fn phase_7_update_check_defaults() {
        let svc = setup().await;
        let s = svc.get().await.unwrap();
        assert!(!s.update_check_enabled);
        assert_eq!(s.update_check_last_run, None);
        assert_eq!(s.update_check_skip_version, None);
    }

    #[tokio::test]
    async fn update_check_skip_version_round_trip_and_clear() {
        let svc = setup().await;
        let with = svc
            .update(SettingsPatch {
                update_check_skip_version: Some("v1.2.3".into()),
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(with.update_check_skip_version.as_deref(), Some("v1.2.3"));
        let cleared = svc
            .update(SettingsPatch {
                update_check_skip_version: Some(String::new()),
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(cleared.update_check_skip_version, None);
    }

    #[tokio::test]
    async fn record_update_check_run_persists_timestamp() {
        let svc = setup().await;
        svc.record_update_check_run(1_234_567).await.unwrap();
        let s = svc.get().await.unwrap();
        assert_eq!(s.update_check_last_run, Some(1_234_567));
    }

    // --- Phase 8 quality-pipeline tests (ADR-0028, ADR-0029). ---

    #[tokio::test]
    async fn phase_8_quality_defaults() {
        let svc = setup().await;
        let s = svc.get().await.unwrap();
        assert_eq!(s.video_quality_profile, VideoQualityProfile::P720p30);
        assert!(!s.software_encode_opt_in);
        assert!(s.encoder_capability.is_none());
        assert_eq!(s.max_concurrent_reencodes, 1);
        assert!((s.cpu_throttle_high_threshold - 0.7).abs() < 1e-6);
        assert!((s.cpu_throttle_low_threshold - 0.5).abs() < 1e-6);
    }

    #[tokio::test]
    async fn quality_profile_round_trips_via_settings() {
        let svc = setup().await;
        let out = svc
            .update(SettingsPatch {
                video_quality_profile: Some(VideoQualityProfile::P1080p60),
                software_encode_opt_in: Some(true),
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(out.video_quality_profile, VideoQualityProfile::P1080p60);
        assert!(out.software_encode_opt_in);
    }

    #[tokio::test]
    async fn max_concurrent_reencodes_clamped_to_1_2() {
        let svc = setup().await;
        let high = svc
            .update(SettingsPatch {
                max_concurrent_reencodes: Some(99),
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(high.max_concurrent_reencodes, 2);
        let low = svc
            .update(SettingsPatch {
                max_concurrent_reencodes: Some(0),
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(low.max_concurrent_reencodes, 1);
    }

    #[tokio::test]
    async fn cpu_throttle_thresholds_clamp_into_range() {
        let svc = setup().await;
        let out = svc
            .update(SettingsPatch {
                cpu_throttle_high_threshold: Some(2.0),
                cpu_throttle_low_threshold: Some(-1.0),
                ..Default::default()
            })
            .await
            .unwrap();
        assert!((out.cpu_throttle_high_threshold - 0.9).abs() < 1e-6);
        assert!((out.cpu_throttle_low_threshold - 0.3).abs() < 1e-6);
    }

    #[tokio::test]
    async fn cpu_throttle_low_must_be_below_high_by_5pp() {
        let svc = setup().await;
        let err = svc
            .update(SettingsPatch {
                cpu_throttle_high_threshold: Some(0.6),
                cpu_throttle_low_threshold: Some(0.59),
                ..Default::default()
            })
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::InvalidInput { .. }));
    }

    #[tokio::test]
    async fn record_encoder_capability_round_trip() {
        use crate::domain::quality::{EncoderCapability, EncoderKind};
        let svc = setup().await;
        let cap = EncoderCapability {
            primary: EncoderKind::VideoToolbox,
            available: vec![EncoderKind::VideoToolbox, EncoderKind::Software],
            h265: true,
            h264: true,
            tested_at: 1_700_000_000,
        };
        svc.record_encoder_capability(&cap).await.unwrap();
        let out = svc.get().await.unwrap();
        assert_eq!(out.encoder_capability, Some(cap));
    }

    // --- Phase 8 distribution-mode tests (ADR-0030). ---

    #[tokio::test]
    async fn phase_8_distribution_defaults() {
        let svc = setup().await;
        let s = svc.get().await.unwrap();
        // Empty downloads table → migration leaves DEFAULT 'pull'.
        assert_eq!(s.distribution_mode, DistributionMode::Pull);
        assert_eq!(s.sliding_window_size, 2);
        assert!(s.prefetch_enabled);
    }

    #[tokio::test]
    async fn distribution_mode_round_trips() {
        let svc = setup().await;
        let out = svc
            .update(SettingsPatch {
                distribution_mode: Some(DistributionMode::Auto),
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(out.distribution_mode, DistributionMode::Auto);
    }

    #[tokio::test]
    async fn sliding_window_size_clamped_1_to_20() {
        let svc = setup().await;
        let high = svc
            .update(SettingsPatch {
                sliding_window_size: Some(99),
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(high.sliding_window_size, 20);
        let low = svc
            .update(SettingsPatch {
                sliding_window_size: Some(0),
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(low.sliding_window_size, 1);
    }

    #[tokio::test]
    async fn prefetch_enabled_round_trips() {
        let svc = setup().await;
        let off = svc
            .update(SettingsPatch {
                prefetch_enabled: Some(false),
                ..Default::default()
            })
            .await
            .unwrap();
        assert!(!off.prefetch_enabled);
    }
}
