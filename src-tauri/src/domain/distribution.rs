//! Phase 8 distribution model — pull-on-demand state machine
//! (ADR-0030).
//!
//! This module owns the pure decision logic: which transitions are
//! valid, what the sliding window means, how pre-fetch picks the
//! next candidate.  No I/O, no SQL, no async.  The
//! `services::distribution` module wraps these decisions in DB
//! reads/writes and event emission.

use serde::{Deserialize, Serialize};
use specta::Type;
use thiserror::Error;

/// Distribution mode — selects between v1.0 auto-download
/// behaviour and the v2.0 pull-on-demand model.  Persisted in
/// `app_settings.distribution_mode` (migration 0017).  New installs
/// default to `Pull`; existing installs are pinned to `Auto` by the
/// migration's backward-compat detection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum DistributionMode {
    /// v1.0 behaviour: polling auto-enqueues every newly-discovered
    /// VOD into the download queue.
    Auto,
    /// v2.0 default: polling produces `available` rows; the user
    /// picks explicitly (or pre-fetch picks one VOD ahead).
    Pull,
}

impl DistributionMode {
    pub fn as_db_str(self) -> &'static str {
        match self {
            DistributionMode::Auto => "auto",
            DistributionMode::Pull => "pull",
        }
    }

    pub fn from_db_str(s: &str) -> Option<Self> {
        Some(match s {
            "auto" => DistributionMode::Auto,
            "pull" => DistributionMode::Pull,
            _ => return None,
        })
    }
}

/// Lifecycle state of a single VOD row in the distribution model.
/// Persisted as `vods.status` (migration 0016).  CHECK constraint
/// in the migration file enforces the same closed set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum VodStatus {
    /// Polled by the background, metadata only — file not on disk.
    Available,
    /// User picked this VOD (or pre-fetch did); waiting for a
    /// download worker.
    Queued,
    /// Download in flight.
    Downloading,
    /// File on disk, ready to play.
    Ready,
    /// Watched (`watch_progress.state ∈ {completed, manually_watched}`)
    /// — eligible for sliding-window cleanup.
    Archived,
    /// Cleanup deleted the file.  Row stays so the user can re-pick
    /// for a fresh download.
    Deleted,
}

impl VodStatus {
    pub fn as_db_str(self) -> &'static str {
        match self {
            VodStatus::Available => "available",
            VodStatus::Queued => "queued",
            VodStatus::Downloading => "downloading",
            VodStatus::Ready => "ready",
            VodStatus::Archived => "archived",
            VodStatus::Deleted => "deleted",
        }
    }

    pub fn from_db_str(s: &str) -> Option<Self> {
        Some(match s {
            "available" => VodStatus::Available,
            "queued" => VodStatus::Queued,
            "downloading" => VodStatus::Downloading,
            "ready" => VodStatus::Ready,
            "archived" => VodStatus::Archived,
            "deleted" => VodStatus::Deleted,
            _ => return None,
        })
    }

    /// True iff the row currently consumes a slot in the
    /// per-streamer sliding window (i.e. there's a file or a
    /// pending download on disk that counts toward the cap).
    pub fn occupies_window_slot(self) -> bool {
        matches!(
            self,
            VodStatus::Queued | VodStatus::Downloading | VodStatus::Ready
        )
    }

    /// True iff the row is eligible for sliding-window enforcement
    /// (i.e. it's a candidate the enforcer can move to `Deleted` to
    /// free a slot).
    pub fn is_archived_for_window(self) -> bool {
        matches!(self, VodStatus::Archived)
    }
}

/// Distribution-state errors thrown by the pure transition logic.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum DistributionError {
    #[error("invalid status transition: {from:?} -> {to:?}")]
    InvalidTransition { from: VodStatus, to: VodStatus },
}

/// Validate a proposed `from -> to` transition.  The state diagram
/// (ADR-0030 §State machine) is encoded here.  Pure — exposed for
/// unit tests + the service layer's pre-write check.
///
/// Allowed transitions:
/// - `Available` -> `Queued` (user pick or pre-fetch)
/// - `Queued`    -> `Available` (user unpick)
/// - `Queued`    -> `Downloading` (worker picked up)
/// - `Downloading` -> `Ready` (download complete)
/// - `Downloading` -> `Available` (download cancelled or failed
///   non-fatally; the user can re-pick later)
/// - `Ready`     -> `Archived` (watch state crossed completion)
/// - `Archived`  -> `Deleted` (sliding-window enforcer / cleanup)
/// - `Ready`     -> `Deleted` (manual cleanup of a never-watched
///   download; cleanup-by-watermark path)
/// - `Deleted`   -> `Queued` (user re-picks an archived/deleted
///   VOD; goes through the queue again)
///
/// Same-state transitions (e.g. `Available -> Available`) are
/// rejected — the service layer is expected to skip the write
/// rather than no-op through the validator.
pub fn validate_transition(from: VodStatus, to: VodStatus) -> Result<(), DistributionError> {
    use VodStatus::*;
    let allowed = matches!(
        (from, to),
        (Available, Queued)
            | (Queued, Available)
            | (Queued, Downloading)
            | (Downloading, Ready)
            | (Downloading, Available)
            | (Ready, Archived)
            | (Archived, Deleted)
            | (Ready, Deleted)
            | (Deleted, Queued)
    );
    if allowed {
        Ok(())
    } else {
        Err(DistributionError::InvalidTransition { from, to })
    }
}

/// Pure helper: given the current per-streamer counts of
/// `Archived` rows, decide which (if any) candidate's vod_id to
/// transition to `Deleted` to honour the sliding-window cap.
///
/// `archived_oldest_first` is the list of archived rows for the
/// streamer, sorted by `last_watched_at` ASC (oldest first).
/// `current_window_count` is the number of rows currently in
/// occupies_window_slot() states (queued + downloading + ready).
/// `window_size` is the configured cap.
///
/// Returns `Some(vod_id)` when the oldest archived row should be
/// freed because adding ONE more occupied slot would breach the
/// cap.  Returns `None` when there's room.
pub fn sliding_window_pick_eviction(
    archived_oldest_first: &[String],
    current_window_count: usize,
    window_size: usize,
) -> Option<&str> {
    if current_window_count < window_size {
        return None;
    }
    archived_oldest_first.first().map(|s| s.as_str())
}

/// Pure pre-fetch decision (ADR-0031).  Given the streamer's VOD
/// list and the currently-watching VOD, return the next candidate
/// to pre-fetch.
///
/// Inputs:
/// - `currently_watching` — the VOD the user is actively watching.
/// - `streamer_vods` — every VOD for that streamer, sorted by
///   `stream_started_at` ASC, paired with their `VodStatus`.
/// - `window_room` — how many additional slots the per-streamer
///   sliding window has free (i.e. `window_size - current_count`).
///
/// Returns `Some(vod_id)` when there's a strictly-newer
/// `Available` VOD AND the window has at least 1 free slot.
/// Returns `None` otherwise (no candidate, no room, etc.).
pub fn prefetch_pick_next<'a>(
    currently_watching: &str,
    streamer_vods: &'a [(String, VodStatus)],
    window_room: usize,
) -> Option<&'a str> {
    if window_room == 0 {
        return None;
    }
    let current_pos = streamer_vods
        .iter()
        .position(|(id, _)| id == currently_watching)?;
    streamer_vods
        .iter()
        .skip(current_pos + 1)
        .find(|(_, status)| matches!(status, VodStatus::Available))
        .map(|(id, _)| id.as_str())
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn vod_status_round_trip() {
        for s in [
            VodStatus::Available,
            VodStatus::Queued,
            VodStatus::Downloading,
            VodStatus::Ready,
            VodStatus::Archived,
            VodStatus::Deleted,
        ] {
            assert_eq!(VodStatus::from_db_str(s.as_db_str()), Some(s));
        }
        assert_eq!(VodStatus::from_db_str("paused"), None);
    }

    #[test]
    fn distribution_mode_round_trip() {
        assert_eq!(
            DistributionMode::from_db_str("auto"),
            Some(DistributionMode::Auto)
        );
        assert_eq!(
            DistributionMode::from_db_str("pull"),
            Some(DistributionMode::Pull)
        );
        assert_eq!(DistributionMode::from_db_str("hybrid"), None);
    }

    #[test]
    fn occupies_window_slot_includes_in_flight_states() {
        assert!(VodStatus::Queued.occupies_window_slot());
        assert!(VodStatus::Downloading.occupies_window_slot());
        assert!(VodStatus::Ready.occupies_window_slot());
        assert!(!VodStatus::Available.occupies_window_slot());
        assert!(!VodStatus::Archived.occupies_window_slot());
        assert!(!VodStatus::Deleted.occupies_window_slot());
    }

    #[test]
    fn validate_transition_accepts_documented_paths() {
        for (from, to) in [
            (VodStatus::Available, VodStatus::Queued),
            (VodStatus::Queued, VodStatus::Available),
            (VodStatus::Queued, VodStatus::Downloading),
            (VodStatus::Downloading, VodStatus::Ready),
            (VodStatus::Downloading, VodStatus::Available),
            (VodStatus::Ready, VodStatus::Archived),
            (VodStatus::Archived, VodStatus::Deleted),
            (VodStatus::Ready, VodStatus::Deleted),
            (VodStatus::Deleted, VodStatus::Queued),
        ] {
            validate_transition(from, to).unwrap();
        }
    }

    #[test]
    fn validate_transition_rejects_undocumented_paths() {
        // Same-state.
        assert!(validate_transition(VodStatus::Ready, VodStatus::Ready).is_err());
        // Skipping queued.
        assert!(validate_transition(VodStatus::Available, VodStatus::Downloading).is_err());
        // Going backward in lifecycle.
        assert!(validate_transition(VodStatus::Ready, VodStatus::Queued).is_err());
        assert!(validate_transition(VodStatus::Archived, VodStatus::Ready).is_err());
        // Re-pick from archived must go through deleted (or stay).
        assert!(validate_transition(VodStatus::Archived, VodStatus::Queued).is_err());
    }

    #[test]
    fn sliding_window_returns_oldest_when_capacity_breached() {
        let archived = vec!["v1".to_string(), "v2".to_string(), "v3".to_string()];
        // window_size=2, current=2 (already at cap), adding 1 more would breach.
        let pick = sliding_window_pick_eviction(&archived, 2, 2);
        assert_eq!(pick, Some("v1"));
    }

    #[test]
    fn sliding_window_returns_none_when_room_available() {
        let archived = vec!["v1".to_string()];
        let pick = sliding_window_pick_eviction(&archived, 1, 5);
        assert_eq!(pick, None);
    }

    #[test]
    fn sliding_window_returns_none_when_no_archived_rows() {
        let pick = sliding_window_pick_eviction(&[], 5, 2);
        assert_eq!(pick, None);
    }

    #[test]
    fn prefetch_picks_next_available_after_current() {
        let vods = vec![
            ("v1".to_string(), VodStatus::Archived),
            ("v2".to_string(), VodStatus::Ready), // currently watching
            ("v3".to_string(), VodStatus::Available),
            ("v4".to_string(), VodStatus::Available),
        ];
        let pick = prefetch_pick_next("v2", &vods, 2);
        assert_eq!(pick, Some("v3"));
    }

    #[test]
    fn prefetch_skips_already_queued_or_ready() {
        let vods = vec![
            ("v1".to_string(), VodStatus::Ready),  // currently watching
            ("v2".to_string(), VodStatus::Queued), // already pre-fetched
            ("v3".to_string(), VodStatus::Available),
        ];
        let pick = prefetch_pick_next("v1", &vods, 5);
        assert_eq!(pick, Some("v3"));
    }

    #[test]
    fn prefetch_returns_none_when_window_is_full() {
        let vods = vec![
            ("v1".to_string(), VodStatus::Ready),
            ("v2".to_string(), VodStatus::Available),
        ];
        let pick = prefetch_pick_next("v1", &vods, 0);
        assert_eq!(pick, None);
    }

    #[test]
    fn prefetch_returns_none_when_no_newer_available() {
        let vods = vec![
            ("v1".to_string(), VodStatus::Available),
            ("v2".to_string(), VodStatus::Ready), // last in chronology
        ];
        let pick = prefetch_pick_next("v2", &vods, 5);
        assert_eq!(pick, None);
    }

    #[test]
    fn prefetch_returns_none_when_currently_watching_unknown() {
        let vods = vec![
            ("v1".to_string(), VodStatus::Available),
            ("v2".to_string(), VodStatus::Available),
        ];
        let pick = prefetch_pick_next("v999", &vods, 5);
        assert_eq!(pick, None);
    }
}
