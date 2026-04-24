//! Decide which `ingest_status` a VOD should end up with, given its
//! chapters, metadata, and the user's game filter.
//!
//! Pure function. No I/O. Invariants:
//!
//! 1. Sub-only always wins over game filter (user cannot download it, so
//!    game-matching is moot).
//! 2. Live VODs are deferred (`SkippedLive`) regardless of game match — we
//!    do not download anything that might still be growing.
//! 3. A VOD with chapters is `Eligible` iff at least one chapter matches
//!    an enabled game id. No chapters matching → `SkippedGame`.
//! 4. A VOD whose only chapter is `Synthetic` with `game_id = None`
//!    ("unknown — review") is NOT auto-classified. We return `Eligible`
//!    with a reason flag so the UI can surface it in a review queue
//!    rather than silently hiding it.

use std::collections::HashSet;

use crate::domain::chapter::Chapter;
use crate::domain::vod::IngestStatus;

/// Inputs the classifier needs, all pure values.
#[derive(Debug)]
pub struct ClassificationInput<'a> {
    pub is_sub_only: bool,
    pub streamer_live: bool,
    pub chapters: &'a [Chapter],
    pub enabled_game_ids: &'a HashSet<String>,
}

/// Classifier output — the status + a human-readable reason string that
/// the service persists verbatim into `vods.status_reason`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Classification {
    pub status: IngestStatus,
    pub reason: String,
}

pub fn classify(input: &ClassificationInput<'_>) -> Classification {
    if input.is_sub_only {
        return Classification {
            status: IngestStatus::SkippedSubOnly,
            reason: "sub-only VOD; re-checked on next poll".to_owned(),
        };
    }
    if input.streamer_live {
        return Classification {
            status: IngestStatus::SkippedLive,
            reason: "streamer currently live; deferred".to_owned(),
        };
    }

    let has_unknown_only = input.chapters.iter().all(|c| c.game_id.is_none());
    if has_unknown_only {
        return Classification {
            status: IngestStatus::Eligible,
            reason: "unknown game — review".to_owned(),
        };
    }

    let matched = input.chapters.iter().any(|c| {
        c.game_id
            .as_deref()
            .map(|id| input.enabled_game_ids.contains(id))
            .unwrap_or(false)
    });

    if matched {
        Classification {
            status: IngestStatus::Eligible,
            reason: String::new(),
        }
    } else {
        Classification {
            status: IngestStatus::SkippedGame,
            reason: "no chapter matches enabled games".to_owned(),
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::domain::chapter::ChapterType;

    fn make_chapter(game_id: Option<&str>) -> Chapter {
        Chapter {
            position_ms: 0,
            duration_ms: 300_000,
            game_id: game_id.map(str::to_owned),
            game_name: game_id.unwrap_or_default().to_owned(),
            chapter_type: ChapterType::GameChange,
        }
    }

    fn gta_set() -> HashSet<String> {
        HashSet::from(["32982".to_owned()])
    }

    #[test]
    fn sub_only_wins() {
        let c = classify(&ClassificationInput {
            is_sub_only: true,
            streamer_live: false,
            chapters: &[make_chapter(Some("32982"))],
            enabled_game_ids: &gta_set(),
        });
        assert_eq!(c.status, IngestStatus::SkippedSubOnly);
    }

    #[test]
    fn live_beats_game_match() {
        let c = classify(&ClassificationInput {
            is_sub_only: false,
            streamer_live: true,
            chapters: &[make_chapter(Some("32982"))],
            enabled_game_ids: &gta_set(),
        });
        assert_eq!(c.status, IngestStatus::SkippedLive);
    }

    #[test]
    fn gta_match_is_eligible() {
        let c = classify(&ClassificationInput {
            is_sub_only: false,
            streamer_live: false,
            chapters: &[make_chapter(Some("32982"))],
            enabled_game_ids: &gta_set(),
        });
        assert_eq!(c.status, IngestStatus::Eligible);
        assert!(c.reason.is_empty());
    }

    #[test]
    fn non_gta_is_skipped_game() {
        let c = classify(&ClassificationInput {
            is_sub_only: false,
            streamer_live: false,
            chapters: &[make_chapter(Some("509658"))], // Just Chatting
            enabled_game_ids: &gta_set(),
        });
        assert_eq!(c.status, IngestStatus::SkippedGame);
    }

    #[test]
    fn mixed_with_one_match_is_eligible() {
        let c = classify(&ClassificationInput {
            is_sub_only: false,
            streamer_live: false,
            chapters: &[
                make_chapter(Some("509658")),
                make_chapter(Some("32982")),
                make_chapter(Some("509658")),
            ],
            enabled_game_ids: &gta_set(),
        });
        assert_eq!(c.status, IngestStatus::Eligible);
    }

    #[test]
    fn unknown_only_goes_to_review() {
        let c = classify(&ClassificationInput {
            is_sub_only: false,
            streamer_live: false,
            chapters: &[make_chapter(None)],
            enabled_game_ids: &gta_set(),
        });
        assert_eq!(c.status, IngestStatus::Eligible);
        assert_eq!(c.reason, "unknown game — review");
    }
}
