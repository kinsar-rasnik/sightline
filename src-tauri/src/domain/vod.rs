//! VOD domain types.

use serde::{Deserialize, Serialize};
use specta::Type;

/// Ingest lifecycle. Matches the `CHECK` constraint on `vods.ingest_status`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum IngestStatus {
    Pending,
    ChaptersFetched,
    Eligible,
    SkippedGame,
    SkippedSubOnly,
    SkippedLive,
    Error,
}

impl IngestStatus {
    /// Stable wire string matching the `CHECK` constraint.
    pub fn as_db_str(self) -> &'static str {
        match self {
            IngestStatus::Pending => "pending",
            IngestStatus::ChaptersFetched => "chapters_fetched",
            IngestStatus::Eligible => "eligible",
            IngestStatus::SkippedGame => "skipped_game",
            IngestStatus::SkippedSubOnly => "skipped_sub_only",
            IngestStatus::SkippedLive => "skipped_live",
            IngestStatus::Error => "error",
        }
    }

    pub fn from_db_str(s: &str) -> Option<Self> {
        Some(match s {
            "pending" => IngestStatus::Pending,
            "chapters_fetched" => IngestStatus::ChaptersFetched,
            "eligible" => IngestStatus::Eligible,
            "skipped_game" => IngestStatus::SkippedGame,
            "skipped_sub_only" => IngestStatus::SkippedSubOnly,
            "skipped_live" => IngestStatus::SkippedLive,
            "error" => IngestStatus::Error,
            _ => return None,
        })
    }

    /// Terminal states (`Eligible`, `SkippedGame`, etc.) can be revisited
    /// on subsequent polls. `Pending` and `ChaptersFetched` are purely
    /// transient — any VOD in these states mid-poll is retried.
    pub fn is_terminal(self) -> bool {
        !matches!(self, IngestStatus::Pending | IngestStatus::ChaptersFetched)
    }
}

/// A muted segment returned by the Helix `videos` endpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct MutedSegment {
    pub offset_seconds: i64,
    pub duration_seconds: i64,
}

/// Storage-aligned VOD row. The `*_at` fields are unix seconds UTC.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Vod {
    pub twitch_video_id: String,
    pub twitch_user_id: String,
    pub stream_id: Option<String>,
    pub title: String,
    pub description: String,
    pub stream_started_at: i64,
    pub published_at: i64,
    pub url: String,
    pub thumbnail_url: Option<String>,
    pub duration_seconds: i64,
    pub view_count: i64,
    pub language: String,
    pub muted_segments: Vec<MutedSegment>,
    pub is_sub_only: bool,
    pub helix_game_id: Option<String>,
    pub helix_game_name: Option<String>,
    pub ingest_status: IngestStatus,
    pub status_reason: String,
    pub first_seen_at: i64,
    pub last_seen_at: i64,
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn status_roundtrips_db_strings() {
        for s in [
            IngestStatus::Pending,
            IngestStatus::ChaptersFetched,
            IngestStatus::Eligible,
            IngestStatus::SkippedGame,
            IngestStatus::SkippedSubOnly,
            IngestStatus::SkippedLive,
            IngestStatus::Error,
        ] {
            assert_eq!(IngestStatus::from_db_str(s.as_db_str()), Some(s));
        }
    }

    #[test]
    fn unknown_db_string_is_none() {
        assert_eq!(IngestStatus::from_db_str("unknown"), None);
    }

    #[test]
    fn pending_is_not_terminal() {
        assert!(!IngestStatus::Pending.is_terminal());
        assert!(!IngestStatus::ChaptersFetched.is_terminal());
    }

    #[test]
    fn terminals_are_terminal() {
        for s in [
            IngestStatus::Eligible,
            IngestStatus::SkippedGame,
            IngestStatus::SkippedSubOnly,
            IngestStatus::SkippedLive,
            IngestStatus::Error,
        ] {
            assert!(s.is_terminal(), "{s:?} should be terminal");
        }
    }
}
