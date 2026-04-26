//! Storage-forecast service (Phase 8 / v2.0.1, ADR-0032).
//!
//! Closes the AC9 surface deferred from Phase 8.  The math itself
//! is the pure helper in `domain::quality` (`quality_factor_gb_per_hour`);
//! this service composes it with three measured inputs from the
//! database (avg VOD length, stream frequency, sliding-window
//! size) plus the user's free disk to produce the two end-user-
//! meaningful numbers — weekly download GB and peak disk GB — and
//! a coloured watermark indicator.
//!
//! Pure decision step is exposed as
//! [`estimate`] so the frontend test suite can assert the math
//! without spinning up a database.  Database-backed wrappers
//! ([`estimate_streamer_footprint`] / [`estimate_global_footprint`])
//! handle the I/O side.

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use specta::Type;
use sqlx::Row;

use crate::domain::quality::VideoQualityProfile;
use crate::error::AppError;
use crate::infra::db::Db;
use crate::infra::fs::space::FreeSpaceProbe;
use crate::services::settings::SettingsService;

/// Lookback window for the avg-VOD-length / frequency probe.
/// Matches the 30-day window described in ADR-0032 §Inputs.
const FORECAST_LOOKBACK_SECONDS: i64 = 30 * 24 * 60 * 60;

/// Fallback duration when a streamer has fewer than 3 VODs in the
/// lookback window.  3 hours is the GTA-RP median per the data set
/// behind `quality_factor_gb_per_hour`.
const DEFAULT_AVG_VOD_HOURS: f64 = 3.0;

/// Fallback frequency for a fresh streamer with insufficient
/// history.  ~3.5 streams/week — a common cadence for the
/// streamers Sightline targets.
const DEFAULT_STREAMS_PER_DAY: f64 = 0.5;

/// Threshold below which the data-driven probe is too noisy to
/// trust (per ADR-0032).
const MIN_VODS_FOR_DATA_DRIVEN_AVG: i64 = 3;

/// Watermark risk indicator (ADR-0032 §Outputs).  Maps the peak
/// disk forecast against the user's free disk to one of three
/// buckets that the UI can render as green/amber/red.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum WatermarkRisk {
    /// Peak forecast occupies < 50 % of free disk.
    Green,
    /// Peak forecast occupies 50–80 % of free disk.
    Amber,
    /// Peak forecast occupies > 80 % of free disk — would
    /// trip the auto-cleanup high watermark.
    Red,
}

/// Per-streamer (or global) forecast.  Numbers are best-effort
/// estimates; ADR-0032 §Known inaccuracies documents the fuzz.
#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ForecastResult {
    /// GB the streamer is expected to download in a typical week
    /// at the user's current quality + window settings.
    pub weekly_download_gb: f64,
    /// Peak GB the streamer's content can occupy on disk
    /// simultaneously (i.e., `sliding_window_size × avg_vod_gb`).
    pub peak_disk_gb: f64,
    /// Coloured indicator combining `peak_disk_gb` with the user's
    /// free disk on the library partition.
    pub watermark_risk: WatermarkRisk,
    /// Rounded average VOD length in hours used for the math.
    pub avg_vod_hours: f64,
    /// Streams per day used for the math.
    pub streams_per_day: f64,
    /// Free disk on the library partition at probe time, in GB.
    /// Useful for the UI's "X GB free" line so the renderer
    /// doesn't have to round-trip.
    pub free_disk_gb: f64,
    /// Whether the avg / frequency numbers came from real history
    /// (`true`) or the global defaults (`false`).
    pub data_driven: bool,
}

/// Per-streamer entry inside the global forecast.  Surfaces the
/// streamer's identity alongside the forecast row so the UI can
/// render a breakdown table without an extra round-trip.
#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct StreamerForecast {
    pub twitch_user_id: String,
    pub display_name: String,
    pub login: String,
    pub forecast: ForecastResult,
}

/// Global forecast: combined totals + per-streamer breakdown.  The
/// `combined` field is the SUM of every active streamer's forecast,
/// not a re-derivation from aggregated history.  This matches what
/// the user actually sees on disk at peak.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct GlobalForecast {
    pub combined: ForecastResult,
    pub per_streamer: Vec<StreamerForecast>,
}

/// Measured inputs to the pure forecast function.  All four are
/// resolved from the database / settings before this struct is
/// passed in, so [`estimate`] stays pure and trivially testable.
#[derive(Debug, Clone, Copy)]
pub struct ForecastInputs {
    /// Average VOD length in hours.
    pub avg_vod_hours: f64,
    /// Stream frequency in streams/day.
    pub streams_per_day: f64,
    /// User's chosen quality profile (drives the GB-per-hour table).
    pub quality_profile: VideoQualityProfile,
    /// Per-streamer sliding-window cap.
    pub sliding_window_size: i64,
    /// Free disk on the library partition in GB (used to compute
    /// the watermark risk indicator).
    pub free_disk_gb: f64,
    /// True iff the avg / frequency came from real 30-day data
    /// rather than the global fallback constants.
    pub data_driven: bool,
}

/// Pure forecast computation (ADR-0032 §Decision).  Returns a
/// rounded-to-the-tenth GB number for both weekly download and peak
/// disk, plus the watermark-risk indicator that maps peak vs free.
pub fn estimate(inputs: ForecastInputs) -> ForecastResult {
    let quality_factor = inputs.quality_profile.quality_factor_gb_per_hour();
    let avg_vod_gb = inputs.avg_vod_hours.max(0.0) * quality_factor;
    let weekly = inputs.streams_per_day.max(0.0) * 7.0 * avg_vod_gb;
    let peak = (inputs.sliding_window_size.max(0) as f64) * avg_vod_gb;
    let risk = classify_risk(peak, inputs.free_disk_gb);
    ForecastResult {
        weekly_download_gb: round_tenth(weekly),
        peak_disk_gb: round_tenth(peak),
        watermark_risk: risk,
        avg_vod_hours: round_tenth(inputs.avg_vod_hours),
        streams_per_day: round_tenth(inputs.streams_per_day),
        free_disk_gb: round_tenth(inputs.free_disk_gb),
        data_driven: inputs.data_driven,
    }
}

/// Classify peak-vs-free into the three colour buckets.  Defensive
/// against `free_disk_gb <= 0` (no library configured / probe
/// failure): treats any positive peak as `Red` so the UI surfaces
/// the issue rather than rendering as `Green`.
fn classify_risk(peak_gb: f64, free_gb: f64) -> WatermarkRisk {
    if free_gb <= 0.0 {
        return if peak_gb > 0.0 {
            WatermarkRisk::Red
        } else {
            WatermarkRisk::Green
        };
    }
    let fraction = peak_gb / free_gb;
    if fraction < 0.5 {
        WatermarkRisk::Green
    } else if fraction < 0.8 {
        WatermarkRisk::Amber
    } else {
        WatermarkRisk::Red
    }
}

fn round_tenth(v: f64) -> f64 {
    (v * 10.0).round() / 10.0
}

/// DB-backed wrapper: probe history for the given streamer + read
/// settings + read free disk + call [`estimate`].
#[derive(Debug)]
pub struct ForecastService {
    db: Db,
    settings: SettingsService,
    space_probe: Arc<dyn FreeSpaceProbe>,
}

impl ForecastService {
    pub fn new(db: Db, settings: SettingsService, space_probe: Arc<dyn FreeSpaceProbe>) -> Self {
        Self {
            db,
            settings,
            space_probe,
        }
    }

    /// Forecast for a single streamer.  `twitch_user_id` is the
    /// Twitch numeric ID (the same column the rest of the schema
    /// keys on).  Returns NotFound if no streamer row exists.
    pub async fn estimate_streamer_footprint(
        &self,
        twitch_user_id: &str,
        now_unix: i64,
    ) -> Result<ForecastResult, AppError> {
        // Confirm the streamer exists so the call surface is stable
        // (otherwise we'd return a "default" forecast for nothing).
        let exists: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM streamers WHERE twitch_user_id = ? AND deleted_at IS NULL",
        )
        .bind(twitch_user_id)
        .fetch_one(self.db.pool())
        .await?;
        if exists == 0 {
            return Err(AppError::NotFound);
        }
        let inputs = self.build_inputs(twitch_user_id, now_unix).await?;
        Ok(estimate(inputs))
    }

    /// Global forecast: sum of every active streamer's forecast +
    /// per-streamer breakdown for the UI.  Free-disk read happens
    /// once, not per streamer, since they all share the partition.
    pub async fn estimate_global_footprint(
        &self,
        now_unix: i64,
    ) -> Result<GlobalForecast, AppError> {
        let settings = self.settings.get().await?;
        let free_gb = self.read_free_disk_gb(&settings.library_root).await;
        let rows = sqlx::query(
            "SELECT twitch_user_id, login, display_name FROM streamers
              WHERE deleted_at IS NULL ORDER BY login ASC",
        )
        .fetch_all(self.db.pool())
        .await?;

        let mut per_streamer: Vec<StreamerForecast> = Vec::with_capacity(rows.len());
        let mut total_weekly = 0.0f64;
        let mut total_peak = 0.0f64;
        let mut data_driven_any = false;

        for row in rows {
            let twitch_user_id: String = row.try_get("twitch_user_id")?;
            let login: String = row.try_get("login")?;
            let display_name: String = row.try_get("display_name")?;
            let inputs = ForecastInputs {
                free_disk_gb: free_gb,
                ..self
                    .build_inputs_with_free(&twitch_user_id, now_unix, free_gb, &settings)
                    .await?
            };
            let forecast = estimate(inputs);
            total_weekly += forecast.weekly_download_gb;
            total_peak += forecast.peak_disk_gb;
            data_driven_any |= forecast.data_driven;
            per_streamer.push(StreamerForecast {
                twitch_user_id,
                login,
                display_name,
                forecast,
            });
        }

        let combined = ForecastResult {
            weekly_download_gb: round_tenth(total_weekly),
            peak_disk_gb: round_tenth(total_peak),
            watermark_risk: classify_risk(total_peak, free_gb),
            avg_vod_hours: 0.0, // not meaningful for the combined view
            streams_per_day: 0.0,
            free_disk_gb: round_tenth(free_gb),
            data_driven: data_driven_any,
        };
        Ok(GlobalForecast {
            combined,
            per_streamer,
        })
    }

    async fn build_inputs(
        &self,
        twitch_user_id: &str,
        now_unix: i64,
    ) -> Result<ForecastInputs, AppError> {
        let settings = self.settings.get().await?;
        let free_gb = self.read_free_disk_gb(&settings.library_root).await;
        self.build_inputs_with_free(twitch_user_id, now_unix, free_gb, &settings)
            .await
    }

    async fn build_inputs_with_free(
        &self,
        twitch_user_id: &str,
        now_unix: i64,
        free_gb: f64,
        settings: &crate::services::settings::AppSettings,
    ) -> Result<ForecastInputs, AppError> {
        let cutoff = now_unix - FORECAST_LOOKBACK_SECONDS;
        let row = sqlx::query(
            "SELECT
                COUNT(*) AS vod_count,
                COALESCE(CAST(AVG(duration_seconds) AS REAL), 0.0) AS avg_seconds
              FROM vods
             WHERE twitch_user_id = ?
               AND stream_started_at >= ?
               AND duration_seconds > 0",
        )
        .bind(twitch_user_id)
        .bind(cutoff)
        .fetch_one(self.db.pool())
        .await?;
        let count: i64 = row.try_get("vod_count")?;
        let avg_seconds: f64 = row.try_get("avg_seconds")?;
        let (avg_hours, streams_per_day, data_driven) = if count >= MIN_VODS_FOR_DATA_DRIVEN_AVG {
            (avg_seconds / 3600.0, count as f64 / 30.0, true)
        } else {
            (DEFAULT_AVG_VOD_HOURS, DEFAULT_STREAMS_PER_DAY, false)
        };
        Ok(ForecastInputs {
            avg_vod_hours: avg_hours,
            streams_per_day,
            quality_profile: settings.video_quality_profile,
            sliding_window_size: settings.sliding_window_size,
            free_disk_gb: free_gb,
            data_driven,
        })
    }

    /// Free disk on the library partition (or a fallback path).
    /// Returns 0.0 on probe failure or unconfigured library so the
    /// classifier surfaces a Red badge rather than misleading the
    /// user.
    async fn read_free_disk_gb(&self, library_root: &Option<String>) -> f64 {
        let Some(root) = library_root else {
            return 0.0;
        };
        match self
            .space_probe
            .free_bytes(std::path::Path::new(root.as_str()))
            .await
        {
            Ok(bytes) => (bytes as f64) / 1_073_741_824.0, // 1024^3
            Err(_) => 0.0,
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::infra::clock::{Clock, FixedClock};
    use crate::infra::fs::space::FakeFreeSpace;

    fn base_inputs() -> ForecastInputs {
        ForecastInputs {
            avg_vod_hours: 6.0,
            streams_per_day: 1.0,
            quality_profile: VideoQualityProfile::P720p30,
            sliding_window_size: 2,
            free_disk_gb: 100.0,
            data_driven: true,
        }
    }

    #[test]
    fn estimate_with_default_quality_matches_adr_table() {
        // 6 h × 0.7 GB/h = 4.2 GB per VOD.  Window = 2 → peak 8.4 GB.
        // 1 stream/day × 7 days × 4.2 GB = 29.4 GB / week.
        let res = estimate(base_inputs());
        assert!((res.weekly_download_gb - 29.4).abs() < 0.05);
        assert!((res.peak_disk_gb - 8.4).abs() < 0.05);
        assert_eq!(res.watermark_risk, WatermarkRisk::Green);
        assert!(res.data_driven);
    }

    #[test]
    fn watermark_risk_amber_at_50_to_80_percent_of_free() {
        // Peak ≈ 70 GB / 100 GB free = 0.7 → Amber.  At default
        // 720p30 (0.7 GB/h) the math is 50 h × 0.7 × 2 = 70 GB.
        let res = estimate(ForecastInputs {
            avg_vod_hours: 50.0,
            free_disk_gb: 100.0,
            sliding_window_size: 2,
            ..base_inputs()
        });
        assert_eq!(res.watermark_risk, WatermarkRisk::Amber);
    }

    #[test]
    fn watermark_risk_red_above_80_percent() {
        // 65 h × 0.7 × 2 = 91 GB > 80% of 100 GB free.
        let res = estimate(ForecastInputs {
            avg_vod_hours: 65.0,
            free_disk_gb: 100.0,
            sliding_window_size: 2,
            ..base_inputs()
        });
        assert_eq!(res.watermark_risk, WatermarkRisk::Red);
    }

    #[test]
    fn watermark_risk_red_when_no_library_configured_and_peak_positive() {
        // free_disk_gb = 0 (unconfigured); any positive peak → Red
        // so the UI doesn't mislead with a Green badge.
        let res = estimate(ForecastInputs {
            free_disk_gb: 0.0,
            ..base_inputs()
        });
        assert_eq!(res.watermark_risk, WatermarkRisk::Red);
    }

    #[test]
    fn watermark_risk_green_when_no_library_and_zero_peak() {
        // 0 streams or 0 hours → 0 peak; even unconfigured library
        // shouldn't badge Red because there's nothing to download.
        let res = estimate(ForecastInputs {
            avg_vod_hours: 0.0,
            sliding_window_size: 0,
            free_disk_gb: 0.0,
            ..base_inputs()
        });
        assert_eq!(res.watermark_risk, WatermarkRisk::Green);
    }

    #[test]
    fn negative_inputs_are_clamped_to_zero() {
        // Defensive guard: if the DB returns a degenerate negative
        // (e.g., duration_seconds got nullified somehow), the
        // forecast must not produce negative GB numbers.
        let res = estimate(ForecastInputs {
            avg_vod_hours: -1.0,
            streams_per_day: -2.0,
            sliding_window_size: -3,
            ..base_inputs()
        });
        assert_eq!(res.weekly_download_gb, 0.0);
        assert_eq!(res.peak_disk_gb, 0.0);
    }

    #[test]
    fn source_quality_factor_is_4x_default() {
        // Sanity check that the quality factor flows through
        // correctly: source profile is 4 GB/h, default is 0.7 GB/h.
        let default = estimate(base_inputs());
        let src = estimate(ForecastInputs {
            quality_profile: VideoQualityProfile::Source,
            ..base_inputs()
        });
        // 4.0 / 0.7 ≈ 5.71x
        let ratio = src.weekly_download_gb / default.weekly_download_gb;
        assert!((ratio - 5.71).abs() < 0.1, "source/default ratio = {ratio}");
    }

    async fn setup_service() -> (ForecastService, Db, Arc<dyn Clock>) {
        let db = Db::open_in_memory().await.unwrap();
        db.migrate().await.unwrap();
        let clock: Arc<dyn Clock> = Arc::new(FixedClock::at(2_000_000));
        let settings = SettingsService::new(db.clone(), clock.clone());
        let probe: Arc<dyn FreeSpaceProbe> = Arc::new(FakeFreeSpace(u64::MAX));
        let svc = ForecastService::new(db.clone(), settings, probe);
        (svc, db, clock)
    }

    async fn seed_streamer(db: &Db, twitch_user_id: &str, login: &str) {
        sqlx::query(
            "INSERT INTO streamers (twitch_user_id, login, display_name,
                 broadcaster_type, twitch_created_at, added_at)
             VALUES (?, ?, ?, '', 0, 0)",
        )
        .bind(twitch_user_id)
        .bind(login)
        .bind(login)
        .execute(db.pool())
        .await
        .unwrap();
    }

    async fn seed_vod(db: &Db, twitch_user_id: &str, vod_id: &str, started_at: i64, duration: i64) {
        sqlx::query(
            "INSERT INTO vods (twitch_video_id, twitch_user_id, title, stream_started_at,
                 published_at, url, duration_seconds, ingest_status, first_seen_at, last_seen_at)
             VALUES (?, ?, 'title', ?, ?, ?, ?, 'eligible', 0, 0)",
        )
        .bind(vod_id)
        .bind(twitch_user_id)
        .bind(started_at)
        .bind(started_at)
        .bind(format!("https://twitch.tv/videos/{vod_id}"))
        .bind(duration)
        .execute(db.pool())
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn streamer_footprint_falls_back_to_defaults_for_fresh_streamer() {
        let (svc, db, _clock) = setup_service().await;
        seed_streamer(&db, "100", "fresh").await;
        let res = svc
            .estimate_streamer_footprint("100", 2_000_000)
            .await
            .unwrap();
        // Fresh streamer (no VODs) hits the fallback path —
        // data_driven must be false.
        assert!(!res.data_driven);
        assert!((res.avg_vod_hours - DEFAULT_AVG_VOD_HOURS).abs() < 0.1);
        assert!((res.streams_per_day - DEFAULT_STREAMS_PER_DAY).abs() < 0.05);
    }

    #[tokio::test]
    async fn streamer_footprint_uses_30_day_history_when_available() {
        let (svc, db, _clock) = setup_service().await;
        seed_streamer(&db, "100", "active").await;
        let now = 30 * 24 * 60 * 60; // 30 days in seconds, picked so all seeds land within window
        // Seed 6 VODs in the last 30 days, each 4 hours (14_400 s).
        for i in 1..=6 {
            seed_vod(&db, "100", &format!("v{i}"), now - i * 86_400, 14_400).await;
        }
        let res = svc
            .estimate_streamer_footprint("100", now + 86_400)
            .await
            .unwrap();
        assert!(res.data_driven);
        assert!((res.avg_vod_hours - 4.0).abs() < 0.1);
        // 6 VODs / 30 days = 0.2/day
        assert!((res.streams_per_day - 0.2).abs() < 0.05);
    }

    #[tokio::test]
    async fn streamer_footprint_returns_not_found_for_unknown_id() {
        let (svc, _db, _clock) = setup_service().await;
        let err = svc.estimate_streamer_footprint("999", 1).await.unwrap_err();
        assert!(matches!(err, AppError::NotFound));
    }

    #[tokio::test]
    async fn global_footprint_combines_all_streamers() {
        let (svc, db, _clock) = setup_service().await;
        seed_streamer(&db, "100", "alice").await;
        seed_streamer(&db, "200", "bob").await;
        let res = svc.estimate_global_footprint(2_000_000).await.unwrap();
        // Two streamers, each falling back to defaults.
        assert_eq!(res.per_streamer.len(), 2);
        // Combined weekly should be 2× a single streamer's weekly
        // forecast (modulo rounding).
        let single = &res.per_streamer[0].forecast;
        let combined = &res.combined;
        assert!((combined.weekly_download_gb - single.weekly_download_gb * 2.0).abs() < 0.5);
    }
}
