//! Application settings service — game filter, poll interval knobs,
//! and a typed view of the credentials meta row.

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use specta::Type;
use sqlx::Row;

use crate::domain::poll_schedule::PollIntervals;
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
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CredentialsStatus {
    pub configured: bool,
    pub client_id_masked: Option<String>,
    pub last_token_acquired_at: Option<i64>,
}

/// Partial settings update — any subset may be supplied by the frontend.
#[derive(Debug, Default, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SettingsPatch {
    pub enabled_game_ids: Option<Vec<String>>,
    pub poll_floor_seconds: Option<i64>,
    pub poll_recent_seconds: Option<i64>,
    pub poll_ceiling_seconds: Option<i64>,
    pub concurrency_cap: Option<i64>,
    pub first_backfill_limit: Option<i64>,
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
                    poll_ceiling_seconds, concurrency_cap, first_backfill_limit
             FROM app_settings WHERE id = 1",
        )
        .fetch_one(self.db.pool())
        .await?;

        let games_json: String = row.try_get(0)?;
        let enabled_game_ids: Vec<String> =
            serde_json::from_str(&games_json).map_err(AppError::from)?;

        let credentials = self.read_credentials_meta().await?;

        Ok(AppSettings {
            enabled_game_ids,
            poll_floor_seconds: row.try_get(1)?,
            poll_recent_seconds: row.try_get(2)?,
            poll_ceiling_seconds: row.try_get(3)?,
            concurrency_cap: row.try_get(4)?,
            first_backfill_limit: row.try_get(5)?,
            credentials,
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
                 updated_at = ?
             WHERE id = 1",
        )
        .bind(&games_json)
        .bind(desired_intervals.floor_seconds)
        .bind(desired_intervals.recent_seconds)
        .bind(desired_intervals.ceiling_seconds)
        .bind(concurrency)
        .bind(backfill)
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
}
