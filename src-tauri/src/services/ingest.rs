//! VOD ingest pipeline. Glues Helix pagination, GQL chapters, the game
//! filter, the live gate, and persistence.
//!
//! Flow per streamer:
//!
//! 1. Helix live check → drives the gate + `last_live_at`.
//! 2. Helix `videos?user_id=X&type=archive`, cursor-paginated.
//!    * First-ever poll of a streamer: fetch up to `first_backfill_limit`
//!      (default 100) and store them all.
//!    * Subsequent poll: stop on the first already-seen VOD id unless
//!      `ignore_seen = true` (manual rescan).
//! 3. For each new / updated VOD, fetch chapter moments from GQL.
//!    Chapters that fail GQL fall into `ingest_status = error` and are
//!    retried on the next poll.
//! 4. Classify against the streamer's live state + sub-only + game
//!    filter (see `domain::game_filter::classify`).
//! 5. Persist VOD + chapters (+ status) transactionally. Emit the
//!    `vod:ingested` / `vod:updated` event.
//!
//! The service is intentionally single-threaded per streamer — the
//! concurrency cap lives one level up, in the poller.

use std::collections::HashSet;
use std::sync::Arc;

use tracing::{debug, info, instrument, warn};

use crate::domain::chapter::{Chapter, ChapterType};
use crate::domain::duration::parse_helix_duration;
use crate::domain::game_filter::{ClassificationInput, classify};
use crate::domain::vod::IngestStatus;
use crate::error::AppError;
use crate::infra::clock::Clock;
use crate::infra::db::Db;
use crate::infra::twitch::gql::GqlClient;
use crate::infra::twitch::helix::{HelixClient, HelixVideo};
use crate::services::settings::SettingsService;
use crate::services::streamers::StreamerService;
use crate::services::time_util::parse_iso_to_unix;

/// Outcome of a single streamer's poll cycle.
#[derive(Debug, Default, Clone)]
pub struct IngestReport {
    pub vods_seen: i64,
    pub vods_new: i64,
    pub vods_updated: i64,
    pub chapters_fetched: i64,
    pub errors: Vec<String>,
    pub rate_limited: bool,
    pub live_now: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct IngestOptions {
    pub ignore_seen: bool,
    /// When the streamer has never been polled before, pull up to this many
    /// VODs. Subsequent polls cap at the incremental page size and stop on
    /// the first already-seen id.
    pub first_backfill_limit: u32,
    pub page_size: u32,
}

impl Default for IngestOptions {
    fn default() -> Self {
        Self {
            ignore_seen: false,
            first_backfill_limit: 100,
            page_size: 25,
        }
    }
}

/// Persisted events emitted by the ingest service. The commands layer
/// converts these into Tauri events.
#[derive(Debug, Clone)]
pub enum IngestEvent {
    VodIngested {
        twitch_video_id: String,
        twitch_user_id: String,
        ingest_status: String,
        stream_started_at: i64,
    },
    VodUpdated {
        twitch_video_id: String,
        ingest_status: String,
    },
}

#[derive(Debug)]
pub struct IngestService {
    db: Db,
    helix: Arc<HelixClient>,
    gql: Arc<GqlClient>,
    clock: Arc<dyn Clock>,
    settings: SettingsService,
    streamers: Arc<StreamerService>,
}

impl IngestService {
    pub fn new(
        db: Db,
        helix: Arc<HelixClient>,
        gql: Arc<GqlClient>,
        clock: Arc<dyn Clock>,
        settings: SettingsService,
        streamers: Arc<StreamerService>,
    ) -> Self {
        Self {
            db,
            helix,
            gql,
            clock,
            settings,
            streamers,
        }
    }

    /// Run the ingest pipeline for a streamer. Returns a report + any
    /// events to fan out. Errors in subordinate calls are captured into
    /// `report.errors`; only a hard DB failure short-circuits the run.
    #[instrument(skip(self), fields(twitch_user_id = %twitch_user_id))]
    pub async fn run(
        &self,
        twitch_user_id: &str,
        options: IngestOptions,
    ) -> Result<(IngestReport, Vec<IngestEvent>), AppError> {
        let mut report = IngestReport::default();
        let mut events = Vec::new();

        let settings = self.settings.get().await?;
        let enabled_games: HashSet<String> = settings.enabled_game_ids.iter().cloned().collect();

        let live_now = match self.helix.is_streamer_live(twitch_user_id).await {
            Ok(v) => v,
            Err(AppError::TwitchRateLimit { .. }) => {
                report.rate_limited = true;
                report.errors.push("live check rate-limited".to_owned());
                return Ok((report, events));
            }
            Err(other) => {
                report.errors.push(format!("live check: {other}"));
                false
            }
        };
        report.live_now = live_now;
        if live_now {
            self.streamers
                .set_live_flag(twitch_user_id, true)
                .await
                .ok();
        }

        let videos = match self.collect_videos(twitch_user_id, &options).await {
            Ok(v) => v,
            Err(AppError::TwitchRateLimit { .. }) => {
                report.rate_limited = true;
                report.errors.push("videos fetch rate-limited".to_owned());
                return Ok((report, events));
            }
            Err(other) => return Err(other),
        };
        report.vods_seen = videos.len() as i64;

        for video in videos {
            match self.ingest_one(&video, live_now, &enabled_games).await {
                Ok((was_new, chapters_fetched, ev)) => {
                    if was_new {
                        report.vods_new += 1;
                    } else {
                        report.vods_updated += 1;
                    }
                    report.chapters_fetched += chapters_fetched;
                    events.push(ev);
                }
                Err(e) => {
                    warn!(video_id = %video.id, error = %e, "vod ingest failed");
                    report.errors.push(format!("vod {}: {e}", video.id));
                }
            }
        }

        info!(
            vods_new = report.vods_new,
            vods_updated = report.vods_updated,
            "ingest run complete"
        );
        Ok((report, events))
    }

    async fn collect_videos(
        &self,
        twitch_user_id: &str,
        options: &IngestOptions,
    ) -> Result<Vec<HelixVideo>, AppError> {
        let first_poll = self.first_poll(twitch_user_id).await?;
        let cap = if first_poll {
            options.first_backfill_limit.clamp(1, 500)
        } else {
            options.page_size.clamp(1, 100)
        };
        let stop_on_seen = !options.ignore_seen && !first_poll;

        let mut out = Vec::new();
        let mut cursor: Option<String> = None;
        loop {
            let page = self
                .helix
                .list_videos_archive(twitch_user_id, cursor.as_deref(), options.page_size)
                .await?;
            for v in page.data {
                if stop_on_seen && self.vod_known(&v.id).await? {
                    return Ok(out);
                }
                out.push(v);
                if out.len() as u32 >= cap {
                    return Ok(out);
                }
            }
            match page.pagination.cursor {
                Some(c) if !c.is_empty() => cursor = Some(c),
                _ => break,
            }
        }
        Ok(out)
    }

    async fn first_poll(&self, twitch_user_id: &str) -> Result<bool, AppError> {
        let has_vod: i64 = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM vods WHERE twitch_user_id = ? LIMIT 1)",
        )
        .bind(twitch_user_id)
        .fetch_one(self.db.pool())
        .await?;
        Ok(has_vod == 0)
    }

    async fn vod_known(&self, video_id: &str) -> Result<bool, AppError> {
        let n: i64 = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM vods WHERE twitch_video_id = ? LIMIT 1)",
        )
        .bind(video_id)
        .fetch_one(self.db.pool())
        .await?;
        Ok(n != 0)
    }

    async fn ingest_one(
        &self,
        video: &HelixVideo,
        live_now: bool,
        enabled_games: &HashSet<String>,
    ) -> Result<(bool, i64, IngestEvent), AppError> {
        let now = self.clock.unix_seconds();
        let was_new = !self.vod_known(&video.id).await?;

        let duration_seconds = parse_helix_duration(&video.duration).unwrap_or(0);
        let stream_started_at = parse_iso_to_unix(&video.created_at)?;
        let published_at = parse_iso_to_unix(&video.published_at).unwrap_or(stream_started_at);

        let is_sub_only = video.viewable != "public";
        let mut status: IngestStatus;
        let mut status_reason: String;

        // Chapter fetch via GQL. A failure leaves chapters empty and
        // flags the VOD as `error`; we'll retry on the next poll.
        let mut chapters_fetched_count: i64 = 0;
        let (chapter_records, gql_error) = match self.gql.fetch_video_moments(&video.id).await {
            Ok(list) => {
                chapters_fetched_count = list.len() as i64;
                (list, None)
            }
            Err(e) => {
                debug!(video_id = %video.id, error = %e, "gql moments fetch failed");
                (Vec::new(), Some(e.to_string()))
            }
        };
        let merged = crate::domain::chapter::merge_chapters(
            &chapter_records,
            duration_seconds,
            video.game_id_from_helix(),
            video.game_name_from_helix(),
        );

        if is_sub_only {
            status = IngestStatus::SkippedSubOnly;
            status_reason = "sub-only VOD; re-checked on next poll".to_owned();
        } else if live_now {
            status = IngestStatus::SkippedLive;
            status_reason = "streamer currently live; deferred".to_owned();
        } else if video.kind != "archive" {
            status = IngestStatus::SkippedLive;
            status_reason = format!("vod type {} is not archive", video.kind);
        } else {
            let classification = classify(&ClassificationInput {
                is_sub_only,
                streamer_live: live_now,
                chapters: &merged,
                enabled_game_ids: enabled_games,
            });
            status = classification.status;
            status_reason = classification.reason;
        }

        if gql_error.is_some()
            && matches!(status, IngestStatus::Eligible | IngestStatus::SkippedGame)
        {
            // We classified with Helix fallback chapters. Surface the
            // GQL error in the reason but preserve the classification.
            status_reason = if status_reason.is_empty() {
                format!(
                    "gql chapters unavailable: {}",
                    gql_error.as_deref().unwrap_or("unknown")
                )
            } else {
                format!(
                    "{status_reason}; gql chapters unavailable: {}",
                    gql_error.as_deref().unwrap_or("unknown")
                )
            };
            status = IngestStatus::Error;
        }

        let muted_segments_json =
            serde_json::to_string(&video.muted_segments).map_err(AppError::from)?;

        let mut tx = self.db.pool().begin().await?;

        sqlx::query(
            "INSERT INTO vods (
                twitch_video_id, twitch_user_id, stream_id, title, description,
                stream_started_at, published_at, url, thumbnail_url,
                duration_seconds, view_count, language, muted_segments_json,
                is_sub_only, helix_game_id, helix_game_name, ingest_status,
                status_reason, first_seen_at, last_seen_at
             ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(twitch_video_id) DO UPDATE SET
                stream_id = excluded.stream_id,
                title = excluded.title,
                description = excluded.description,
                published_at = excluded.published_at,
                url = excluded.url,
                thumbnail_url = excluded.thumbnail_url,
                duration_seconds = excluded.duration_seconds,
                view_count = excluded.view_count,
                language = excluded.language,
                muted_segments_json = excluded.muted_segments_json,
                is_sub_only = excluded.is_sub_only,
                helix_game_id = excluded.helix_game_id,
                helix_game_name = excluded.helix_game_name,
                ingest_status = excluded.ingest_status,
                status_reason = excluded.status_reason,
                last_seen_at = excluded.last_seen_at",
        )
        .bind(&video.id)
        .bind(&video.user_id)
        .bind(&video.stream_id)
        .bind(&video.title)
        .bind(&video.description)
        .bind(stream_started_at)
        .bind(published_at)
        .bind(&video.url)
        .bind(&video.thumbnail_url)
        .bind(duration_seconds)
        .bind(video.view_count)
        .bind(&video.language)
        .bind(&muted_segments_json)
        .bind(if is_sub_only { 1 } else { 0 })
        .bind(video.game_id_from_helix())
        .bind(video.game_name_from_helix())
        .bind(status.as_db_str())
        .bind(&status_reason)
        .bind(now)
        .bind(now)
        .execute(&mut *tx)
        .await?;

        // Refresh chapters: delete+insert inside the same tx.
        sqlx::query("DELETE FROM chapters WHERE twitch_video_id = ?")
            .bind(&video.id)
            .execute(&mut *tx)
            .await?;
        for c in &merged {
            insert_chapter(&mut tx, &video.id, c).await?;
        }

        tx.commit().await?;

        let ev = if was_new {
            IngestEvent::VodIngested {
                twitch_video_id: video.id.clone(),
                twitch_user_id: video.user_id.clone(),
                ingest_status: status.as_db_str().to_owned(),
                stream_started_at,
            }
        } else {
            IngestEvent::VodUpdated {
                twitch_video_id: video.id.clone(),
                ingest_status: status.as_db_str().to_owned(),
            }
        };
        Ok((was_new, chapters_fetched_count, ev))
    }
}

async fn insert_chapter(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    video_id: &str,
    c: &Chapter,
) -> Result<(), AppError> {
    let chapter_type_str = match c.chapter_type {
        ChapterType::GameChange => "GAME_CHANGE",
        ChapterType::Synthetic => "SYNTHETIC",
        ChapterType::Other => "OTHER",
    };
    sqlx::query(
        "INSERT INTO chapters (twitch_video_id, position_ms, duration_ms, game_id, game_name, chapter_type)
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(video_id)
    .bind(c.position_ms)
    .bind(c.duration_ms)
    .bind(&c.game_id)
    .bind(&c.game_name)
    .bind(chapter_type_str)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

/// Helix `HelixVideo` doesn't actually expose `game_id` at the top
/// level — the Helix `videos` response only carries that for
/// highlights / clips. These helpers are future-proofing so callers
/// don't need to know which struct member to reach for; we accept
/// `None` today and let GQL drive chapter inference.
trait HelixVideoGameHints {
    fn game_id_from_helix(&self) -> Option<&str>;
    fn game_name_from_helix(&self) -> Option<&str>;
}

impl HelixVideoGameHints for HelixVideo {
    fn game_id_from_helix(&self) -> Option<&str> {
        None
    }

    fn game_name_from_helix(&self) -> Option<&str> {
        None
    }
}
