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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
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
                    quality_preset, auto_update_yt_dlp
             FROM app_settings WHERE id = 1",
        )
        .fetch_one(self.db.pool())
        .await?;

        let games_json: String = row.try_get(0)?;
        let enabled_game_ids: Vec<String> =
            serde_json::from_str(&games_json).map_err(AppError::from)?;

        let credentials = self.read_credentials_meta().await?;

        let library_layout_str: String = row.try_get(7)?;
        let library_layout = LibraryLayoutKind::from_db_str(&library_layout_str)
            .unwrap_or(LibraryLayoutKind::Plex);
        let quality_preset_str: String = row.try_get(11)?;
        let quality_preset =
            QualityPreset::from_db_str(&quality_preset_str).unwrap_or(QualityPreset::Source);
        let auto_update_raw: i64 = row.try_get(12)?;

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

        let library_root = patch.library_root.clone().or(current.library_root);
        let library_layout = patch.library_layout.unwrap_or(current.library_layout);
        let staging_path = patch.staging_path.clone().or(current.staging_path);
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
        let auto_update_yt_dlp = patch.auto_update_yt_dlp.unwrap_or(current.auto_update_yt_dlp);

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
        let out = svc
            .update(SettingsPatch {
                library_root: Some("/tmp/lib".into()),
                library_layout: Some(LibraryLayoutKind::Flat),
                max_concurrent_downloads: Some(3),
                bandwidth_limit_bps: Some(5_000_000),
                quality_preset: Some(QualityPreset::P720p60),
                auto_update_yt_dlp: Some(false),
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(out.library_root.as_deref(), Some("/tmp/lib"));
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
