//! Chapter domain types + merger.
//!
//! A VOD's chapters are a list of `GAME_CHANGE` moments from the Twitch
//! GraphQL endpoint (see ADR-0008). Each moment has a `position_ms` +
//! `duration_ms` + `game_id`/`game_name`. The merger here takes a raw
//! moment list, sorts it, and — if the list is empty — synthesizes a
//! single chapter spanning the whole VOD using the Helix top-level
//! `game_id` as a fallback.

use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ChapterType {
    GameChange,
    Synthetic,
    Other,
}

impl ChapterType {
    pub fn as_db_str(self) -> &'static str {
        match self {
            ChapterType::GameChange => "GAME_CHANGE",
            ChapterType::Synthetic => "SYNTHETIC",
            ChapterType::Other => "OTHER",
        }
    }

    pub fn from_db_str(s: &str) -> Option<Self> {
        Some(match s {
            "GAME_CHANGE" => ChapterType::GameChange,
            "SYNTHETIC" => ChapterType::Synthetic,
            "OTHER" => ChapterType::Other,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Chapter {
    pub position_ms: i64,
    pub duration_ms: i64,
    pub game_id: Option<String>,
    pub game_name: String,
    pub chapter_type: ChapterType,
}

/// Merge a raw moment list into the canonical Chapter vector stored in the
/// `chapters` table. If `raw_moments` is empty, fall back to a single
/// synthetic chapter spanning the whole VOD. Inputs with negative
/// durations / positions are clamped to zero; out-of-order moments are
/// sorted ascending.
pub fn merge_chapters(
    raw_moments: &[Chapter],
    vod_duration_seconds: i64,
    helix_game_id: Option<&str>,
    helix_game_name: Option<&str>,
) -> Vec<Chapter> {
    if raw_moments.is_empty() {
        let game_id = helix_game_id.map(str::to_owned);
        let game_name = helix_game_name.unwrap_or("").to_owned();
        let duration_ms = vod_duration_seconds.max(0).saturating_mul(1_000);
        return vec![Chapter {
            position_ms: 0,
            duration_ms,
            game_id,
            game_name,
            chapter_type: ChapterType::Synthetic,
        }];
    }

    let mut moments: Vec<Chapter> = raw_moments
        .iter()
        .map(|c| Chapter {
            position_ms: c.position_ms.max(0),
            duration_ms: c.duration_ms.max(0),
            game_id: c.game_id.clone(),
            game_name: c.game_name.clone(),
            chapter_type: c.chapter_type,
        })
        .collect();
    moments.sort_by_key(|c| c.position_ms);
    moments
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn sample(position_ms: i64, duration_ms: i64, game: &str) -> Chapter {
        Chapter {
            position_ms,
            duration_ms,
            game_id: Some(game.to_string()),
            game_name: game.to_string(),
            chapter_type: ChapterType::GameChange,
        }
    }

    #[test]
    fn empty_input_synthesizes_single_chapter() {
        let out = merge_chapters(&[], 300, Some("32982"), Some("GTA V"));
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].position_ms, 0);
        assert_eq!(out[0].duration_ms, 300_000);
        assert_eq!(out[0].game_id.as_deref(), Some("32982"));
        assert_eq!(out[0].chapter_type, ChapterType::Synthetic);
    }

    #[test]
    fn empty_input_without_helix_game_marks_unknown() {
        let out = merge_chapters(&[], 60, None, None);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].game_id, None);
        assert_eq!(out[0].game_name, "");
    }

    #[test]
    fn sorts_out_of_order_moments() {
        let raw = vec![sample(30_000, 60_000, "b"), sample(0, 30_000, "a")];
        let out = merge_chapters(&raw, 300, None, None);
        assert_eq!(out[0].game_id.as_deref(), Some("a"));
        assert_eq!(out[1].game_id.as_deref(), Some("b"));
    }

    #[test]
    fn clamps_negative_positions() {
        let raw = vec![sample(-10, -5, "a")];
        let out = merge_chapters(&raw, 300, None, None);
        assert_eq!(out[0].position_ms, 0);
        assert_eq!(out[0].duration_ms, 0);
    }

    #[test]
    fn preserves_non_negative_inputs_as_is() {
        let raw = vec![sample(0, 300_000, "gta")];
        let out = merge_chapters(&raw, 300, None, None);
        assert_eq!(out[0].duration_ms, 300_000);
        assert_eq!(out[0].chapter_type, ChapterType::GameChange);
    }

    #[test]
    fn chapter_type_db_roundtrip() {
        for t in [
            ChapterType::GameChange,
            ChapterType::Synthetic,
            ChapterType::Other,
        ] {
            assert_eq!(ChapterType::from_db_str(t.as_db_str()), Some(t));
        }
    }
}
