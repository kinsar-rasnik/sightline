//! Auto-cleanup domain types — pure data the service layer hands to
//! the IPC surface.  No I/O, no `tokio`, no `sqlx`.  See ADR-0024.
//!
//! The candidate-selection rules live here as documented thresholds
//! plus a comparator used by the service.  Tests in this module
//! cover the comparator and the watermark math; the service-layer
//! tests cover the SQL + filesystem side.

use serde::{Deserialize, Serialize};
use specta::Type;

/// One row in a cleanup plan — a VOD the service has flagged as
/// safe to delete.  Returned to the UI verbatim so the user can
/// review before confirming.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CleanupCandidate {
    pub vod_id: String,
    pub streamer_login: String,
    pub stream_started_at: i64,
    pub last_watched_at: i64,
    /// Mirrors `watch_progress.state` wire strings.  Only `completed`
    /// or `manually_watched` ever appear in a plan; the comparator
    /// rejects anything else.
    pub watch_state: String,
    pub size_bytes: i64,
    pub final_path: String,
}

/// What `compute_plan` returns: the candidates ranked in deletion
/// order plus the disk-usage snapshot the plan was built against.
/// `target_free_after_bytes` is non-negative even when no work is
/// projected — `0` simply means "the plan would not delete anything",
/// which is distinct from `candidates.is_empty() && disk pressure is
/// below the high watermark` (status: skipped).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CleanupPlan {
    pub candidates: Vec<CleanupCandidate>,
    pub total_bytes: i64,
    pub free_bytes_before: i64,
    /// Sum of `size_bytes` across `candidates`.
    pub projected_freed_bytes: i64,
    /// Used-fraction snapshot at plan time (0.0..=1.0).  A renderer
    /// can compare to the live watermarks without needing a separate
    /// settings query.
    pub used_fraction_before: f64,
    /// Settings snapshot used to build the plan.  Mirrored so the UI
    /// can label the plan ("would shrink to 75 % of disk capacity").
    pub high_watermark: f64,
    pub low_watermark: f64,
}

/// What `execute_plan` returns once it has actually written to disk.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CleanupResult {
    pub mode: CleanupMode,
    /// Mirrors `cleanup_log.status` wire strings.
    pub status: String,
    pub freed_bytes: i64,
    pub deleted_vod_count: i64,
    pub log_id: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum CleanupMode {
    Scheduled,
    Manual,
    DryRun,
}

impl CleanupMode {
    pub fn as_db_str(self) -> &'static str {
        match self {
            CleanupMode::Scheduled => "scheduled",
            CleanupMode::Manual => "manual",
            CleanupMode::DryRun => "dry_run",
        }
    }
}

/// One row from the `cleanup_log` audit table.  The History view in
/// Settings consumes this directly.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CleanupLogEntry {
    pub id: i64,
    pub ran_at: i64,
    pub mode: String,
    pub freed_bytes: i64,
    pub deleted_vod_count: i64,
    pub status: String,
}

/// Inputs to the candidate-selection comparator.  Borrowed from the
/// SQL row at fetch time; the comparator is pure.
#[derive(Debug, Clone)]
pub struct CandidateInput {
    pub vod_id: String,
    pub streamer_login: String,
    pub stream_started_at: i64,
    pub last_watched_at: i64,
    pub watch_state: WatchStateForCleanup,
    pub size_bytes: i64,
    pub final_path: String,
}

/// Watch-state vocabulary trimmed to what the cleanup service cares
/// about.  Mirrors `domain::watch_progress::WatchState` but kept
/// separate so this module has no dep on `watch_progress`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WatchStateForCleanup {
    Completed,
    ManuallyWatched,
    /// Anything else — `unwatched`, `in_progress`, or no row at all.
    /// Never a candidate.
    Other,
}

impl WatchStateForCleanup {
    pub fn from_db_str(s: &str) -> Self {
        match s {
            "completed" => WatchStateForCleanup::Completed,
            "manually_watched" => WatchStateForCleanup::ManuallyWatched,
            _ => WatchStateForCleanup::Other,
        }
    }

    pub fn as_db_str(self) -> &'static str {
        match self {
            WatchStateForCleanup::Completed => "completed",
            WatchStateForCleanup::ManuallyWatched => "manually_watched",
            WatchStateForCleanup::Other => "other",
        }
    }
}

/// Settings snapshot consumed by `compute_plan_pure`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CleanupSettingsSnapshot {
    pub enabled: bool,
    pub high_watermark: f64,
    pub low_watermark: f64,
}

/// Disk-usage snapshot from the free-space probe at plan-time.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DiskSnapshot {
    pub total_bytes: u64,
    pub free_bytes: u64,
}

impl DiskSnapshot {
    pub fn used_fraction(&self) -> f64 {
        if self.total_bytes == 0 {
            return 0.0;
        }
        let used = self.total_bytes.saturating_sub(self.free_bytes);
        used as f64 / self.total_bytes as f64
    }
}

/// Maximum candidates per plan.  Keeps a single tick from doing too
/// much work; a sustained-pressure scenario is handled by repeated
/// daily ticks.  See ADR-0024 §Candidate selection.
pub const MAX_CANDIDATES_PER_PLAN: usize = 200;

/// Anything watched in the last 24 h is held back from a plan.  See
/// ADR-0024 §Candidate selection #1.
pub const RECENTLY_TOUCHED_GUARD_SECONDS: i64 = 86_400;

/// Pure comparator + projection used by the service layer.  Returns
/// the candidates in deletion order, capped at the per-plan max and
/// stopping once projected free space crosses the low watermark.
///
/// `now_unix` is wall-clock seconds, supplied by the caller via the
/// shared `Clock` trait — never `SystemTime::now` directly.  This
/// keeps the comparator deterministic in tests.
pub fn compute_plan_pure(
    snapshot: DiskSnapshot,
    settings: CleanupSettingsSnapshot,
    inputs: Vec<CandidateInput>,
    now_unix: i64,
) -> PlanProjection {
    let used_fraction_before = snapshot.used_fraction();
    let mut filtered: Vec<CandidateInput> = inputs
        .into_iter()
        .filter(|c| match c.watch_state {
            WatchStateForCleanup::Other => false,
            _ => c.last_watched_at < now_unix - RECENTLY_TOUCHED_GUARD_SECONDS,
        })
        .collect();
    filtered.sort_by(rank_candidate);

    // Project the deletions until the disk would drop below the low
    // watermark or the per-plan cap kicks in.
    let target_used_bytes = (snapshot.total_bytes as f64 * settings.low_watermark) as u64;
    let mut projected_used = snapshot.total_bytes.saturating_sub(snapshot.free_bytes);
    let mut chosen: Vec<CleanupCandidate> = Vec::new();
    for input in filtered {
        if chosen.len() >= MAX_CANDIDATES_PER_PLAN {
            break;
        }
        if projected_used <= target_used_bytes {
            break;
        }
        // Subtract the file size from projected used; saturating to
        // avoid underflow when the candidate is absurdly large for
        // the disk (test scenarios with synthetic sizes).
        projected_used = projected_used.saturating_sub(input.size_bytes.max(0) as u64);
        chosen.push(CleanupCandidate {
            vod_id: input.vod_id,
            streamer_login: input.streamer_login,
            stream_started_at: input.stream_started_at,
            last_watched_at: input.last_watched_at,
            watch_state: input.watch_state.as_db_str().to_owned(),
            size_bytes: input.size_bytes,
            final_path: input.final_path,
        });
    }

    let projected_freed_bytes: i64 = chosen.iter().map(|c| c.size_bytes).sum();
    PlanProjection {
        plan: CleanupPlan {
            candidates: chosen,
            total_bytes: snapshot.total_bytes as i64,
            free_bytes_before: snapshot.free_bytes as i64,
            projected_freed_bytes,
            used_fraction_before,
            high_watermark: settings.high_watermark,
            low_watermark: settings.low_watermark,
        },
        pressure_above_high: used_fraction_before >= settings.high_watermark,
    }
}

/// Result of `compute_plan_pure`.  `pressure_above_high` lets the
/// scheduled tick decide between "execute the plan" and "log a
/// skipped row".
pub struct PlanProjection {
    pub plan: CleanupPlan,
    pub pressure_above_high: bool,
}

/// Strict comparator from ADR-0024 §Candidate selection.  Lower
/// `Ordering` = deleted first.
fn rank_candidate(a: &CandidateInput, b: &CandidateInput) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    // Rule 2: completed before manually_watched.
    let state_rank = |s: WatchStateForCleanup| match s {
        WatchStateForCleanup::Completed => 0,
        WatchStateForCleanup::ManuallyWatched => 1,
        WatchStateForCleanup::Other => 2,
    };
    let by_state = state_rank(a.watch_state).cmp(&state_rank(b.watch_state));
    if by_state != Ordering::Equal {
        return by_state;
    }
    // Rule 4: older last_watched first.
    let by_age = a.last_watched_at.cmp(&b.last_watched_at);
    if by_age != Ordering::Equal {
        return by_age;
    }
    // Rule 5: larger files first when ages tie.
    b.size_bytes.cmp(&a.size_bytes)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn input(
        vod_id: &str,
        state: WatchStateForCleanup,
        last_watched: i64,
        size: i64,
    ) -> CandidateInput {
        CandidateInput {
            vod_id: vod_id.into(),
            streamer_login: "streamer".into(),
            stream_started_at: last_watched - 3600,
            last_watched_at: last_watched,
            watch_state: state,
            size_bytes: size,
            final_path: format!("/lib/{vod_id}.mp4"),
        }
    }

    #[test]
    fn used_fraction_zero_total_returns_zero() {
        let snap = DiskSnapshot {
            total_bytes: 0,
            free_bytes: 0,
        };
        assert_eq!(snap.used_fraction(), 0.0);
    }

    #[test]
    fn used_fraction_simple() {
        let snap = DiskSnapshot {
            total_bytes: 1000,
            free_bytes: 250,
        };
        assert!((snap.used_fraction() - 0.75).abs() < f64::EPSILON);
    }

    #[test]
    fn excludes_recently_touched() {
        let now = 1_000_000;
        let inputs = vec![
            input("recent", WatchStateForCleanup::Completed, now - 3600, 1000),
            input("old", WatchStateForCleanup::Completed, now - 200_000, 1000),
        ];
        let proj = compute_plan_pure(
            DiskSnapshot {
                total_bytes: 10_000,
                free_bytes: 100,
            },
            CleanupSettingsSnapshot {
                enabled: true,
                high_watermark: 0.9,
                low_watermark: 0.75,
            },
            inputs,
            now,
        );
        let chosen: Vec<&str> = proj
            .plan
            .candidates
            .iter()
            .map(|c| c.vod_id.as_str())
            .collect();
        assert_eq!(chosen, vec!["old"]);
    }

    #[test]
    fn excludes_unwatched_and_in_progress() {
        let now = 1_000_000;
        let inputs = vec![
            input(
                "unwatched",
                WatchStateForCleanup::Other,
                now - 200_000,
                1000,
            ),
            input("done", WatchStateForCleanup::Completed, now - 200_000, 1000),
        ];
        let proj = compute_plan_pure(
            DiskSnapshot {
                total_bytes: 10_000,
                free_bytes: 100,
            },
            CleanupSettingsSnapshot {
                enabled: true,
                high_watermark: 0.9,
                low_watermark: 0.75,
            },
            inputs,
            now,
        );
        assert_eq!(proj.plan.candidates.len(), 1);
        assert_eq!(proj.plan.candidates[0].vod_id, "done");
    }

    #[test]
    fn ranks_completed_before_manually_watched() {
        let now = 1_000_000;
        let inputs = vec![
            input(
                "manual",
                WatchStateForCleanup::ManuallyWatched,
                now - 1_000_000,
                1000,
            ),
            input(
                "completed",
                WatchStateForCleanup::Completed,
                now - 200_000,
                1000,
            ),
        ];
        let proj = compute_plan_pure(
            DiskSnapshot {
                total_bytes: 10_000,
                free_bytes: 100,
            },
            CleanupSettingsSnapshot {
                enabled: true,
                high_watermark: 0.9,
                low_watermark: 0.0,
            },
            inputs,
            now,
        );
        let chosen: Vec<&str> = proj
            .plan
            .candidates
            .iter()
            .map(|c| c.vod_id.as_str())
            .collect();
        assert_eq!(chosen, vec!["completed", "manual"]);
    }

    #[test]
    fn stops_when_low_watermark_reached() {
        let now = 1_000_000;
        let inputs = (0..10)
            .map(|i| {
                input(
                    &format!("v{i}"),
                    WatchStateForCleanup::Completed,
                    now - 200_000 - i as i64,
                    1_000,
                )
            })
            .collect();
        // total 10 000, free 1000 ⇒ used 9000 (90 %).  Target 75 %
        // ⇒ used 7500 ⇒ need to free 1500 ⇒ 2 candidates suffice.
        let proj = compute_plan_pure(
            DiskSnapshot {
                total_bytes: 10_000,
                free_bytes: 1000,
            },
            CleanupSettingsSnapshot {
                enabled: true,
                high_watermark: 0.9,
                low_watermark: 0.75,
            },
            inputs,
            now,
        );
        assert_eq!(proj.plan.candidates.len(), 2);
    }

    #[test]
    fn caps_at_max_candidates_per_plan() {
        let now = 1_000_000;
        // 250 candidates, all eligible, with low watermark 0 — ranking
        // would otherwise eat everything.  The cap clips at 200.
        let inputs = (0..250)
            .map(|i| {
                input(
                    &format!("v{i}"),
                    WatchStateForCleanup::Completed,
                    now - 200_000 - i as i64,
                    1,
                )
            })
            .collect();
        let proj = compute_plan_pure(
            DiskSnapshot {
                total_bytes: 1_000_000,
                free_bytes: 1,
            },
            CleanupSettingsSnapshot {
                enabled: true,
                high_watermark: 0.9,
                low_watermark: 0.0,
            },
            inputs,
            now,
        );
        assert_eq!(proj.plan.candidates.len(), MAX_CANDIDATES_PER_PLAN);
    }

    #[test]
    fn pressure_above_high_set_when_used_meets_threshold() {
        let proj = compute_plan_pure(
            DiskSnapshot {
                total_bytes: 1000,
                free_bytes: 100,
            },
            CleanupSettingsSnapshot {
                enabled: true,
                high_watermark: 0.9,
                low_watermark: 0.75,
            },
            vec![],
            1,
        );
        assert!(proj.pressure_above_high);
    }

    #[test]
    fn pressure_above_high_false_when_below_threshold() {
        let proj = compute_plan_pure(
            DiskSnapshot {
                total_bytes: 1000,
                free_bytes: 500,
            },
            CleanupSettingsSnapshot {
                enabled: true,
                high_watermark: 0.9,
                low_watermark: 0.75,
            },
            vec![],
            1,
        );
        assert!(!proj.pressure_above_high);
    }

    #[test]
    fn ties_break_by_size_largest_first() {
        let now = 1_000_000;
        let same_age = now - 200_000;
        let inputs = vec![
            input("small", WatchStateForCleanup::Completed, same_age, 100),
            input("large", WatchStateForCleanup::Completed, same_age, 1000),
        ];
        let proj = compute_plan_pure(
            DiskSnapshot {
                total_bytes: 10_000,
                free_bytes: 100,
            },
            CleanupSettingsSnapshot {
                enabled: true,
                high_watermark: 0.9,
                low_watermark: 0.0,
            },
            inputs,
            now,
        );
        let chosen: Vec<&str> = proj
            .plan
            .candidates
            .iter()
            .map(|c| c.vod_id.as_str())
            .collect();
        assert_eq!(chosen, vec!["large", "small"]);
    }

    #[test]
    fn watch_state_db_round_trip() {
        for &s in &["completed", "manually_watched"] {
            let parsed = WatchStateForCleanup::from_db_str(s);
            assert_eq!(parsed.as_db_str(), s);
        }
        let other = WatchStateForCleanup::from_db_str("unwatched");
        assert!(matches!(other, WatchStateForCleanup::Other));
    }
}
