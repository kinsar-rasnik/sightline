//! Multi-view sync engine domain types (Phase 6 / ADR-0021..0023).
//!
//! Pure, I/O-free.  These shapes describe a sync session — which VODs
//! are mounted in which panes, which pane is leader, the layout choice
//! — plus the helpers (`compute_overlap`,
//! `compute_expected_follower_position`) that tie the sync loop's
//! frontend math to the canonical Rust implementation.
//!
//! The wall-clock offset math itself lives in
//! [`crate::domain::deep_link`]; this module wraps it with the sync-
//! specific surface (drift detection inputs, overlap window queries).

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::domain::deep_link::{DeepLinkContext, resolve_deep_link_target};

/// Database-backed identity of a sync session.  Aliased so the
/// services layer doesn't traffic in raw `i64` everywhere.
pub type SyncSessionId = i64;

/// Pane index within a session.  v1 is fixed at `0` (primary) or `1`
/// (secondary) — see ADR-0021.  Kept as a small named type so a v2
/// expansion to N panes is a single grep.
pub type PaneIndex = i64;

/// Layout vocabulary mirroring `sync_sessions.layout`.  v1 only ships
/// one variant; the enum exists so the frontend's `MultiViewPage` can
/// switch on the discriminant when v2 adds PiP / 2x2 grid options.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
pub enum SyncLayout {
    /// Two panes, fixed 50/50 horizontal split.  ADR-0021.
    #[serde(rename = "split-50-50")]
    Split5050,
}

impl SyncLayout {
    pub fn as_db_str(self) -> &'static str {
        match self {
            SyncLayout::Split5050 => "split-50-50",
        }
    }

    pub fn from_db_str(s: &str) -> Option<Self> {
        match s {
            "split-50-50" => Some(SyncLayout::Split5050),
            _ => None,
        }
    }
}

/// Lifecycle of a `sync_sessions` row.  `Active` is the steady state;
/// `Closed` is terminal — no transitions back, by design.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum SyncStatus {
    Active,
    Closed,
}

impl SyncStatus {
    pub fn as_db_str(self) -> &'static str {
        match self {
            SyncStatus::Active => "active",
            SyncStatus::Closed => "closed",
        }
    }

    pub fn from_db_str(s: &str) -> Option<Self> {
        match s {
            "active" => Some(SyncStatus::Active),
            "closed" => Some(SyncStatus::Closed),
            _ => None,
        }
    }
}

/// One pane's membership inside a session.  Mirrors a
/// `sync_session_panes` row plus the joined `vod_id` in raw form
/// (the services layer dereferences it via `vods` for any UI surface
/// that wants the title / streamer).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SyncMember {
    pub pane_index: PaneIndex,
    pub vod_id: String,
    pub volume: f64,
    pub muted: bool,
    pub joined_at: i64,
}

/// Snapshot of a sync session as exposed to the frontend.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SyncSession {
    pub id: SyncSessionId,
    pub created_at: i64,
    pub closed_at: Option<i64>,
    pub layout: SyncLayout,
    /// `None` only during the brief window between a session's
    /// `INSERT` and the follow-up `UPDATE` that sets the leader. The
    /// services layer never returns a `None` leader to the frontend
    /// — the open path is a single transaction.
    pub leader_pane_index: Option<PaneIndex>,
    pub status: SyncStatus,
    pub panes: Vec<SyncMember>,
}

/// One pane's drift measurement, fed back from the frontend sync
/// loop.  The values describe a single observation; the services
/// layer turns these into `sync:drift_corrected` events when the
/// magnitude exceeds the configured threshold.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DriftMeasurement {
    pub pane_index: PaneIndex,
    pub follower_position_seconds: f64,
    pub expected_position_seconds: f64,
    pub drift_ms: f64,
}

/// Group-wide transport intent.  ADR-0023 — every variant fans to
/// every pane in the session.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SyncTransportCommand {
    Play,
    Pause,
    /// `wall_clock_ts` is the unix-second moment the user dragged
    /// the seek slider to.  The leader's `currentTime` becomes
    /// `clamp(wall_clock_ts - leader.vodStartedAt, 0, leader.duration)`.
    Seek {
        wall_clock_ts: i64,
    },
    /// Mirrors the seven supported playback rates from
    /// `player-constants.ts::PLAYBACK_SPEEDS`.
    SetSpeed {
        speed: f64,
    },
}

/// Wall-clock window every member of a sync group has video for.  The
/// `/multiview` route uses this to bound its seek slider.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct OverlapWindow {
    /// Maximum of all members' `stream_started_at`s.
    pub start_at: i64,
    /// Minimum of all members' (`stream_started_at + duration`)s.
    pub end_at: i64,
}

impl OverlapWindow {
    /// Duration of the overlap in seconds.  Always `>= 0`; an empty
    /// window has `start_at == end_at` which means "no shared
    /// wall-clock window between the panes".
    pub fn duration_seconds(self) -> i64 {
        (self.end_at - self.start_at).max(0)
    }

    /// True when the window is non-empty.
    pub fn is_non_empty(self) -> bool {
        self.end_at > self.start_at
    }
}

/// One member's wall-clock range.  Inputs to [`compute_overlap`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemberRange {
    pub stream_started_at: i64,
    pub duration_seconds: i64,
}

impl MemberRange {
    pub fn end_at(self) -> i64 {
        self.stream_started_at + self.duration_seconds.max(0)
    }
}

/// Compute the wall-clock overlap (intersection of all member ranges).
/// An empty input slice or a disjoint member set yields a degenerate
/// window with `start_at == end_at` — callers should check
/// [`OverlapWindow::is_non_empty`] before treating it as a sliderable
/// range.
pub fn compute_overlap(members: &[MemberRange]) -> OverlapWindow {
    if members.is_empty() {
        return OverlapWindow {
            start_at: 0,
            end_at: 0,
        };
    }
    let mut start_at = i64::MIN;
    let mut end_at = i64::MAX;
    for m in members {
        if m.stream_started_at > start_at {
            start_at = m.stream_started_at;
        }
        let me = m.end_at();
        if me < end_at {
            end_at = me;
        }
    }
    if end_at < start_at {
        // Disjoint: collapse to an empty window anchored at the
        // latest start_at.  That preserves "where would the union
        // start if the streams aligned" without a negative duration.
        end_at = start_at;
    }
    OverlapWindow { start_at, end_at }
}

/// Compute the expected `currentTime` for a follower pane given the
/// leader's wall-clock moment.  Wraps
/// [`crate::domain::deep_link::resolve_deep_link_target`] with the
/// sync-loop's input vocabulary.
///
/// Returns the seek position in seconds, clamped to
/// `[0, follower_duration_seconds]`.
pub fn compute_expected_follower_position(
    leader_moment_unix_seconds: i64,
    follower_stream_started_at: i64,
    follower_duration_seconds: i64,
) -> f64 {
    resolve_deep_link_target(DeepLinkContext {
        moment_unix_seconds: leader_moment_unix_seconds,
        target_stream_started_at: follower_stream_started_at,
        target_duration_seconds: follower_duration_seconds,
    })
}

/// Whether a follower pane is "out of range" for the given leader
/// moment — i.e. the moment falls outside the follower's own VOD's
/// wall-clock window.  ADR-0022 §Out-of-range behaviour.
pub fn is_member_out_of_range(
    leader_moment_unix_seconds: i64,
    member_stream_started_at: i64,
    member_duration_seconds: i64,
) -> bool {
    let end_at = member_stream_started_at + member_duration_seconds.max(0);
    leader_moment_unix_seconds < member_stream_started_at || leader_moment_unix_seconds > end_at
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::float_cmp)]
mod tests {
    use super::*;

    fn mr(start: i64, dur: i64) -> MemberRange {
        MemberRange {
            stream_started_at: start,
            duration_seconds: dur,
        }
    }

    #[test]
    fn layout_round_trips_through_db_string() {
        assert_eq!(
            SyncLayout::from_db_str(SyncLayout::Split5050.as_db_str()),
            Some(SyncLayout::Split5050)
        );
        assert_eq!(SyncLayout::from_db_str("nope"), None);
    }

    #[test]
    fn status_round_trips_through_db_string() {
        for s in [SyncStatus::Active, SyncStatus::Closed] {
            assert_eq!(SyncStatus::from_db_str(s.as_db_str()), Some(s));
        }
    }

    #[test]
    fn overlap_of_two_full_overlapping_members() {
        let w = compute_overlap(&[mr(100, 1000), mr(200, 1000)]);
        // Latest start = 200; earliest end = min(1100, 1200) = 1100.
        assert_eq!(w.start_at, 200);
        assert_eq!(w.end_at, 1100);
        assert_eq!(w.duration_seconds(), 900);
        assert!(w.is_non_empty());
    }

    #[test]
    fn overlap_of_disjoint_members_collapses() {
        let w = compute_overlap(&[mr(0, 100), mr(500, 100)]);
        assert!(!w.is_non_empty());
        assert_eq!(w.duration_seconds(), 0);
    }

    #[test]
    fn overlap_of_one_member_is_that_member() {
        let w = compute_overlap(&[mr(1000, 600)]);
        assert_eq!(w.start_at, 1000);
        assert_eq!(w.end_at, 1600);
    }

    #[test]
    fn overlap_of_empty_slice_is_zero_window() {
        let w = compute_overlap(&[]);
        assert!(!w.is_non_empty());
        assert_eq!(w.duration_seconds(), 0);
    }

    #[test]
    fn overlap_with_nested_member() {
        // Outer covers 0..1000, inner covers 200..400.  Overlap == inner.
        let w = compute_overlap(&[mr(0, 1000), mr(200, 200)]);
        assert_eq!(w.start_at, 200);
        assert_eq!(w.end_at, 400);
    }

    #[test]
    fn expected_follower_uses_deep_link_math() {
        // Leader's wall-clock moment is 100 s after the *target's*
        // stream start — follower should land at 100.0.
        let pos = compute_expected_follower_position(
            1_700_000_100, // leader moment
            1_700_000_000, // follower stream start
            3600,          // follower duration
        );
        assert_eq!(pos, 100.0);
    }

    #[test]
    fn expected_follower_clamps_below_zero_to_zero() {
        let pos = compute_expected_follower_position(
            1_700_000_000, // moment = exactly the target start
            1_700_000_030, // follower started 30 s LATER than the moment
            3600,
        );
        assert_eq!(pos, 0.0);
    }

    #[test]
    fn expected_follower_clamps_above_duration() {
        let pos = compute_expected_follower_position(
            1_700_010_000, // moment is 10 000 s after target start
            1_700_000_000,
            3600, // but target only has 3600 s
        );
        assert_eq!(pos, 3600.0);
    }

    #[test]
    fn out_of_range_when_moment_before_member_start() {
        assert!(is_member_out_of_range(1_000, 1_500, 100));
    }

    #[test]
    fn out_of_range_when_moment_after_member_end() {
        assert!(is_member_out_of_range(1_300, 1_000, 100));
    }

    #[test]
    fn in_range_when_moment_inside_member() {
        assert!(!is_member_out_of_range(1_050, 1_000, 100));
    }

    #[test]
    fn in_range_at_exact_endpoints() {
        // Half-open elsewhere, but for the multi-view purpose a
        // moment at `start_at` or `end_at` exactly is "in range" —
        // the follower pane will seek to 0 or to its duration, both
        // of which are valid playback positions.
        assert!(!is_member_out_of_range(1_000, 1_000, 100));
        assert!(!is_member_out_of_range(1_100, 1_000, 100));
    }
}
