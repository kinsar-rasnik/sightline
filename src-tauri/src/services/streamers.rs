//! Streamer management service.
//!
//! Handles add / remove / list on top of the `streamers` table and the
//! Helix user lookup. Soft-deletes retain VOD rows so watched state
//! survives. A re-add after a soft delete clears `deleted_at` and
//! refreshes the metadata.

use std::sync::Arc;

use sqlx::Row;

use crate::domain::streamer::{Streamer, StreamerSummary, normalize_login};
use crate::error::AppError;
use crate::infra::clock::Clock;
use crate::infra::db::Db;
use crate::infra::twitch::helix::{HelixClient, HelixUser};
use crate::services::time_util::parse_iso_to_unix;

#[derive(Debug)]
pub struct StreamerService {
    db: Db,
    helix: Arc<HelixClient>,
    clock: Arc<dyn Clock>,
}

impl StreamerService {
    pub fn new(db: Db, helix: Arc<HelixClient>, clock: Arc<dyn Clock>) -> Self {
        Self { db, helix, clock }
    }

    /// Add a streamer by login. Validates the login, resolves via
    /// Helix, and upserts into `streamers`. Returns the full
    /// `StreamerSummary` for the UI.
    pub async fn add(&self, raw_login: &str) -> Result<StreamerSummary, AppError> {
        let login = normalize_login(raw_login).map_err(|e| AppError::InvalidInput {
            detail: e.to_string(),
        })?;

        let user = self.helix.get_user_by_login(&login).await?.ok_or_else(|| {
            AppError::TwitchNotFound {
                detail: format!("no Twitch user for login {login}"),
            }
        })?;
        let twitch_created_at = parse_iso_to_unix(&user.created_at)?;
        let now = self.clock.unix_seconds();

        // Upsert, resurrecting a soft-deleted row if present.
        let mut tx = self.db.pool().begin().await?;
        sqlx::query(
            "INSERT INTO streamers (
                twitch_user_id, login, display_name, profile_image_url,
                broadcaster_type, twitch_created_at, added_at, deleted_at,
                last_polled_at, next_poll_at, last_live_at
             ) VALUES (?, ?, ?, ?, ?, ?, ?, NULL, NULL, ?, NULL)
             ON CONFLICT(twitch_user_id) DO UPDATE SET
                login = excluded.login,
                display_name = excluded.display_name,
                profile_image_url = excluded.profile_image_url,
                broadcaster_type = excluded.broadcaster_type,
                twitch_created_at = excluded.twitch_created_at,
                deleted_at = NULL,
                next_poll_at = excluded.next_poll_at",
        )
        .bind(&user.id)
        .bind(&user.login)
        .bind(&user.display_name)
        .bind(&user.profile_image_url)
        .bind(&user.broadcaster_type)
        .bind(twitch_created_at)
        .bind(now)
        .bind(now)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;

        self.summary(&user.id).await
    }

    /// Soft-delete. The row and its VODs stay in the DB — VOD history is
    /// what the user wants preserved — but `deleted_at` hides the
    /// streamer from active lists.
    pub async fn remove(&self, twitch_user_id: &str) -> Result<(), AppError> {
        let now = self.clock.unix_seconds();
        sqlx::query("UPDATE streamers SET deleted_at = ? WHERE twitch_user_id = ?")
            .bind(now)
            .bind(twitch_user_id)
            .execute(self.db.pool())
            .await?;
        Ok(())
    }

    /// List active streamers with enrichment fields.
    pub async fn list_active(&self) -> Result<Vec<StreamerSummary>, AppError> {
        let rows = sqlx::query(
            "SELECT twitch_user_id FROM streamers WHERE deleted_at IS NULL ORDER BY login",
        )
        .fetch_all(self.db.pool())
        .await?;

        let mut out = Vec::with_capacity(rows.len());
        for r in rows {
            let id: String = r.try_get(0)?;
            out.push(self.summary(&id).await?);
        }
        Ok(out)
    }

    /// Record that a streamer was just observed to be live. Updates
    /// `last_live_at`; the caller persists `last_polled_at` separately
    /// with `mark_polled`.
    pub async fn set_live_flag(
        &self,
        twitch_user_id: &str,
        live_now: bool,
    ) -> Result<(), AppError> {
        if !live_now {
            return Ok(());
        }
        let now = self.clock.unix_seconds();
        sqlx::query("UPDATE streamers SET last_live_at = ? WHERE twitch_user_id = ?")
            .bind(now)
            .bind(twitch_user_id)
            .execute(self.db.pool())
            .await?;
        Ok(())
    }

    /// Update the polling schedule fields after a successful poll.
    pub async fn mark_polled(
        &self,
        twitch_user_id: &str,
        next_poll_at: i64,
    ) -> Result<(), AppError> {
        let now = self.clock.unix_seconds();
        sqlx::query(
            "UPDATE streamers
             SET last_polled_at = ?, next_poll_at = ?
             WHERE twitch_user_id = ?",
        )
        .bind(now)
        .bind(next_poll_at)
        .bind(twitch_user_id)
        .execute(self.db.pool())
        .await?;
        Ok(())
    }

    /// Toggle the `favorite` flag on a streamer. Returns the fresh
    /// summary so the caller can update its cache. The boolean newly
    /// persisted is returned alongside for event fan-out.
    pub async fn set_favorite(
        &self,
        twitch_user_id: &str,
        favorite: bool,
    ) -> Result<StreamerSummary, AppError> {
        let rows = sqlx::query("UPDATE streamers SET favorite = ? WHERE twitch_user_id = ?")
            .bind(if favorite { 1 } else { 0 })
            .bind(twitch_user_id)
            .execute(self.db.pool())
            .await?;
        if rows.rows_affected() == 0 {
            return Err(AppError::NotFound);
        }
        self.summary(twitch_user_id).await
    }

    /// Toggle whichever state the streamer is currently in. Returns
    /// (new_summary, new_favorite_flag).
    pub async fn toggle_favorite(
        &self,
        twitch_user_id: &str,
    ) -> Result<(StreamerSummary, bool), AppError> {
        let current = self.summary(twitch_user_id).await?;
        let next = !current.streamer.favorite;
        let updated = self.set_favorite(twitch_user_id, next).await?;
        Ok((updated, next))
    }

    /// List streamer IDs due for polling, respecting the `next_poll_at`
    /// index. Callers (the poller) apply the concurrency cap.
    pub async fn due_for_poll(&self, limit: i64) -> Result<Vec<String>, AppError> {
        let now = self.clock.unix_seconds();
        let rows = sqlx::query(
            "SELECT twitch_user_id FROM streamers
             WHERE deleted_at IS NULL AND (next_poll_at IS NULL OR next_poll_at <= ?)
             ORDER BY next_poll_at ASC NULLS FIRST
             LIMIT ?",
        )
        .bind(now)
        .bind(limit)
        .fetch_all(self.db.pool())
        .await?;

        rows.into_iter()
            .map(|r| r.try_get::<String, _>(0).map_err(AppError::from))
            .collect()
    }

    pub async fn summary(&self, twitch_user_id: &str) -> Result<StreamerSummary, AppError> {
        let row = sqlx::query(
            "SELECT twitch_user_id, login, display_name, profile_image_url,
                    broadcaster_type, twitch_created_at, added_at, deleted_at,
                    last_polled_at, next_poll_at, last_live_at, favorite
             FROM streamers WHERE twitch_user_id = ?",
        )
        .bind(twitch_user_id)
        .fetch_one(self.db.pool())
        .await?;

        let favorite_raw: i64 = row.try_get(11).unwrap_or(0);
        let streamer = Streamer {
            twitch_user_id: row.try_get(0)?,
            login: row.try_get(1)?,
            display_name: row.try_get(2)?,
            profile_image_url: row.try_get(3)?,
            broadcaster_type: row.try_get(4)?,
            twitch_created_at: row.try_get(5)?,
            added_at: row.try_get(6)?,
            deleted_at: row.try_get(7)?,
            last_polled_at: row.try_get(8)?,
            next_poll_at: row.try_get(9)?,
            last_live_at: row.try_get(10)?,
            favorite: favorite_raw != 0,
        };

        let vod_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM vods WHERE twitch_user_id = ?")
                .bind(twitch_user_id)
                .fetch_one(self.db.pool())
                .await?;
        let eligible_vod_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM vods WHERE twitch_user_id = ? AND ingest_status = 'eligible'",
        )
        .bind(twitch_user_id)
        .fetch_one(self.db.pool())
        .await?;

        let now = self.clock.unix_seconds();
        let live_now = streamer
            .last_live_at
            .map(|t| now.saturating_sub(t) < 600)
            .unwrap_or(false);
        let next_poll_eta_seconds = streamer.next_poll_at.map(|t| t.saturating_sub(now).max(0));

        Ok(StreamerSummary {
            streamer,
            vod_count,
            eligible_vod_count,
            live_now,
            next_poll_eta_seconds,
        })
    }
}

/// Produce a `HelixUser` from a stored streamer row — used when the
/// scheduler needs to pass a streamer to ingest without a round-trip.
pub fn streamer_to_helix_user(s: &Streamer) -> HelixUser {
    HelixUser {
        id: s.twitch_user_id.clone(),
        login: s.login.clone(),
        display_name: s.display_name.clone(),
        profile_image_url: s.profile_image_url.clone(),
        broadcaster_type: s.broadcaster_type.clone(),
        created_at: String::new(),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn streamer_to_helix_user_copies_fields() {
        let s = Streamer {
            twitch_user_id: "1".into(),
            login: "vader".into(),
            display_name: "Vader".into(),
            profile_image_url: Some("url".into()),
            broadcaster_type: "partner".into(),
            twitch_created_at: 0,
            added_at: 0,
            deleted_at: None,
            last_polled_at: None,
            next_poll_at: None,
            last_live_at: None,
            favorite: false,
        };
        let h = streamer_to_helix_user(&s);
        assert_eq!(h.login, "vader");
        assert_eq!(h.broadcaster_type, "partner");
    }
}
