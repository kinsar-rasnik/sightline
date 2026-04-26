//! VOD list / get service. Pure read layer on top of the `vods` table.

use serde::{Deserialize, Serialize};
use specta::Type;
use sqlx::Row;

use crate::domain::chapter::{Chapter, ChapterType};
use crate::domain::vod::{IngestStatus, MutedSegment, Vod};
use crate::error::AppError;
use crate::infra::db::Db;

/// VOD list filter. Every field is optional; omitted keys mean
/// "no constraint". `#[specta(optional)]` ensures the TS emission is
/// `T?: T` rather than `T | null` required keys — see ADR-0009.
#[derive(Debug, Default, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", default)]
pub struct VodFilters {
    #[specta(optional)]
    pub streamer_ids: Option<Vec<String>>,
    /// Mirrors `IngestStatus` db strings; caller may send any subset.
    #[specta(optional)]
    pub statuses: Option<Vec<String>>,
    #[specta(optional)]
    pub game_ids: Option<Vec<String>>,
    /// Unix seconds UTC.
    #[specta(optional)]
    pub since: Option<i64>,
    #[specta(optional)]
    pub until: Option<i64>,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum VodSort {
    #[default]
    StreamStartedAtDesc,
    StreamStartedAtAsc,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ListVodsInput {
    pub filters: VodFilters,
    pub sort: VodSort,
    pub limit: i64,
    pub offset: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct VodWithChapters {
    pub vod: Vod,
    pub chapters: Vec<Chapter>,
    /// Streamer display name, denormalized for convenience.
    pub streamer_display_name: String,
    pub streamer_login: String,
}

#[derive(Debug)]
pub struct VodReadService {
    db: Db,
}

impl VodReadService {
    pub fn new(db: Db) -> Self {
        Self { db }
    }

    pub async fn list(&self, input: &ListVodsInput) -> Result<Vec<VodWithChapters>, AppError> {
        let limit = input.limit.clamp(1, 500);
        let offset = input.offset.max(0);

        // Build the filter clause dynamically. We always bind parameters —
        // never concatenate user input into the SQL text.
        let mut clauses: Vec<String> = vec![String::from("1 = 1")];
        let f = &input.filters;

        if let Some(ids) = f.streamer_ids.as_ref()
            && !ids.is_empty()
        {
            clauses.push(format!(
                "v.twitch_user_id IN ({})",
                vec!["?"; ids.len()].join(",")
            ));
        }
        if let Some(statuses) = f.statuses.as_ref()
            && !statuses.is_empty()
        {
            clauses.push(format!(
                "v.ingest_status IN ({})",
                vec!["?"; statuses.len()].join(",")
            ));
        }
        if let Some(game_ids) = f.game_ids.as_ref()
            && !game_ids.is_empty()
        {
            clauses.push(format!(
                "EXISTS (SELECT 1 FROM chapters c WHERE c.twitch_video_id = v.twitch_video_id AND c.game_id IN ({}))",
                vec!["?"; game_ids.len()].join(",")
            ));
        }
        if f.since.is_some() {
            clauses.push(String::from("v.stream_started_at >= ?"));
        }
        if f.until.is_some() {
            clauses.push(String::from("v.stream_started_at <= ?"));
        }

        let order = match input.sort {
            VodSort::StreamStartedAtDesc => "DESC",
            VodSort::StreamStartedAtAsc => "ASC",
        };
        let sql = format!(
            "SELECT v.twitch_video_id, v.twitch_user_id, v.stream_id, v.title, v.description,
                    v.stream_started_at, v.published_at, v.url, v.thumbnail_url, v.duration_seconds,
                    v.view_count, v.language, v.muted_segments_json, v.is_sub_only,
                    v.helix_game_id, v.helix_game_name, v.ingest_status, v.status_reason,
                    v.first_seen_at, v.last_seen_at, v.status,
                    s.display_name, s.login
             FROM vods v JOIN streamers s ON s.twitch_user_id = v.twitch_user_id
             WHERE {}
             ORDER BY v.stream_started_at {order}
             LIMIT ? OFFSET ?",
            clauses.join(" AND ")
        );

        let mut q = sqlx::query(&sql);
        if let Some(ids) = f.streamer_ids.as_ref() {
            for id in ids {
                q = q.bind(id);
            }
        }
        if let Some(statuses) = f.statuses.as_ref() {
            for s in statuses {
                q = q.bind(s);
            }
        }
        if let Some(game_ids) = f.game_ids.as_ref() {
            for g in game_ids {
                q = q.bind(g);
            }
        }
        if let Some(since) = f.since {
            q = q.bind(since);
        }
        if let Some(until) = f.until {
            q = q.bind(until);
        }
        q = q.bind(limit).bind(offset);

        let rows = q.fetch_all(self.db.pool()).await?;

        let mut results = Vec::with_capacity(rows.len());
        for r in rows {
            let vod = row_to_vod(&r)?;
            let chapters = self.chapters_for(&vod.twitch_video_id).await?;
            let streamer_display_name: String = r.try_get(21)?;
            let streamer_login: String = r.try_get(22)?;
            results.push(VodWithChapters {
                vod,
                chapters,
                streamer_display_name,
                streamer_login,
            });
        }
        Ok(results)
    }

    pub async fn get(&self, twitch_video_id: &str) -> Result<VodWithChapters, AppError> {
        let row = sqlx::query(
            "SELECT v.twitch_video_id, v.twitch_user_id, v.stream_id, v.title, v.description,
                    v.stream_started_at, v.published_at, v.url, v.thumbnail_url, v.duration_seconds,
                    v.view_count, v.language, v.muted_segments_json, v.is_sub_only,
                    v.helix_game_id, v.helix_game_name, v.ingest_status, v.status_reason,
                    v.first_seen_at, v.last_seen_at, v.status,
                    s.display_name, s.login
             FROM vods v JOIN streamers s ON s.twitch_user_id = v.twitch_user_id
             WHERE v.twitch_video_id = ?",
        )
        .bind(twitch_video_id)
        .fetch_optional(self.db.pool())
        .await?
        .ok_or(AppError::NotFound)?;

        let vod = row_to_vod(&row)?;
        let chapters = self.chapters_for(twitch_video_id).await?;
        Ok(VodWithChapters {
            vod,
            chapters,
            streamer_display_name: row.try_get(21)?,
            streamer_login: row.try_get(22)?,
        })
    }

    async fn chapters_for(&self, twitch_video_id: &str) -> Result<Vec<Chapter>, AppError> {
        let rows = sqlx::query(
            "SELECT position_ms, duration_ms, game_id, game_name, chapter_type
             FROM chapters WHERE twitch_video_id = ? ORDER BY position_ms ASC",
        )
        .bind(twitch_video_id)
        .fetch_all(self.db.pool())
        .await?;

        rows.into_iter()
            .map(|r| {
                let chapter_type_str: String = r.try_get(4)?;
                Ok(Chapter {
                    position_ms: r.try_get(0)?,
                    duration_ms: r.try_get(1)?,
                    game_id: r.try_get(2)?,
                    game_name: r.try_get(3)?,
                    chapter_type: ChapterType::from_db_str(&chapter_type_str)
                        .unwrap_or(ChapterType::Other),
                })
            })
            .collect::<Result<Vec<_>, AppError>>()
    }
}

fn row_to_vod(row: &sqlx::sqlite::SqliteRow) -> Result<Vod, AppError> {
    let muted_segments_json: String = row.try_get(12)?;
    let muted_segments: Vec<MutedSegmentRow> =
        serde_json::from_str(&muted_segments_json).unwrap_or_default();
    let muted_segments = muted_segments
        .into_iter()
        .map(|s| MutedSegment {
            offset_seconds: s.offset,
            duration_seconds: s.duration,
        })
        .collect();

    let is_sub_only: i64 = row.try_get(13)?;
    let ingest_status_str: String = row.try_get(16)?;
    let vod_status_str: String = row.try_get(20)?;
    let vod_status = crate::domain::distribution::VodStatus::from_db_str(&vod_status_str)
        .unwrap_or(crate::domain::distribution::VodStatus::Available);

    Ok(Vod {
        twitch_video_id: row.try_get(0)?,
        twitch_user_id: row.try_get(1)?,
        stream_id: row.try_get(2)?,
        title: row.try_get(3)?,
        description: row.try_get(4)?,
        stream_started_at: row.try_get(5)?,
        published_at: row.try_get(6)?,
        url: row.try_get(7)?,
        thumbnail_url: row.try_get(8)?,
        duration_seconds: row.try_get(9)?,
        view_count: row.try_get(10)?,
        language: row.try_get(11)?,
        muted_segments,
        is_sub_only: is_sub_only != 0,
        helix_game_id: row.try_get(14)?,
        helix_game_name: row.try_get(15)?,
        ingest_status: IngestStatus::from_db_str(&ingest_status_str).unwrap_or(IngestStatus::Error),
        status_reason: row.try_get(17)?,
        first_seen_at: row.try_get(18)?,
        last_seen_at: row.try_get(19)?,
        status: vod_status,
    })
}

#[derive(Debug, Deserialize, Default)]
struct MutedSegmentRow {
    #[serde(default)]
    offset: i64,
    #[serde(default)]
    duration: i64,
}
