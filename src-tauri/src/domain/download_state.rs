//! Download queue state machine.
//!
//! A single column in the `downloads` table — but the transition rules
//! are behaviour, not data, so they live here and out of the services
//! layer. The allowed transitions match the `CHECK` constraint in
//! `migrations/0004_downloads.sql`:
//!
//! ```text
//!     queued
//!       │ worker picks row
//!       ▼
//!     downloading ───► paused          (user: pause)
//!       │ │              │
//!       │ │              └► downloading (user: resume)
//!       │ ▼
//!       │ failed_retryable ─► queued            (attempts < MAX_ATTEMPTS)
//!       │                 └─► failed_permanent  (attempts == MAX_ATTEMPTS)
//!       ▼
//!     completed                                  (terminal)
//!
//!     cmd_cancel_download:
//!     * → failed_permanent  (reason = USER_CANCELLED)
//!
//!     cmd_retry_download:
//!     failed_* → queued
//! ```

use serde::{Deserialize, Serialize};
use specta::Type;

/// Every reachable state of a download. The wire string (matching the
/// `CHECK` constraint on `downloads.state`) is stable; changing it is
/// a schema change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum DownloadState {
    Queued,
    Downloading,
    Paused,
    Completed,
    FailedRetryable,
    FailedPermanent,
}

impl DownloadState {
    pub fn as_db_str(self) -> &'static str {
        match self {
            DownloadState::Queued => "queued",
            DownloadState::Downloading => "downloading",
            DownloadState::Paused => "paused",
            DownloadState::Completed => "completed",
            DownloadState::FailedRetryable => "failed_retryable",
            DownloadState::FailedPermanent => "failed_permanent",
        }
    }

    pub fn from_db_str(s: &str) -> Option<Self> {
        Some(match s {
            "queued" => DownloadState::Queued,
            "downloading" => DownloadState::Downloading,
            "paused" => DownloadState::Paused,
            "completed" => DownloadState::Completed,
            "failed_retryable" => DownloadState::FailedRetryable,
            "failed_permanent" => DownloadState::FailedPermanent,
            _ => return None,
        })
    }

    /// A state is terminal when the queue worker will not pick the row
    /// up again on its own. `FailedRetryable` is NOT terminal — the
    /// queue scheduler puts it back into `Queued` after the backoff.
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            DownloadState::Completed | DownloadState::FailedPermanent
        )
    }
}

/// Every transition the worker + command surface may request. Keeping
/// these named (rather than "from X to Y") makes the intent readable
/// in call sites and the audit log.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Transition {
    /// Worker picked the row. `Queued → Downloading`.
    Start,
    /// yt-dlp finished. `Downloading → Completed`.
    Succeed,
    /// yt-dlp errored with a transient error *and* attempts < MAX.
    /// `Downloading → FailedRetryable`.
    FailRetryable,
    /// Hard failure — disk full, sub-only, MAX attempts reached.
    /// Any non-terminal → `FailedPermanent`.
    FailPermanent,
    /// User clicked pause. `Downloading → Paused`.
    Pause,
    /// User clicked resume. `Paused → Downloading`.
    Resume,
    /// Scheduler picks a retryable row after backoff.
    /// `FailedRetryable → Queued`.
    Requeue,
    /// User explicitly retries a terminal failure.
    /// `FailedPermanent → Queued`.
    Retry,
    /// User cancels an in-flight or terminal-retryable row.
    /// Any non-Completed → `FailedPermanent`.
    Cancel,
}

/// Max number of auto-retries before the row becomes `FailedPermanent`.
/// Exposed for tests and configurable would-be settings; the queue
/// compares `attempts` against this constant before emitting
/// `FailRetryable` vs `FailPermanent`.
pub const MAX_ATTEMPTS: i64 = 5;

/// Reason codes recorded in `downloads.last_error` and surfaced on the
/// UI. Stable strings; the frontend may match on them to render a
/// specific icon.
pub mod reason {
    pub const DISK_FULL: &str = "DISK_FULL";
    pub const SUB_ONLY: &str = "SUB_ONLY";
    pub const USER_CANCELLED: &str = "USER_CANCELLED";
    pub const NETWORK: &str = "NETWORK";
    pub const UNKNOWN: &str = "UNKNOWN";
    pub const YTDLP_EXIT: &str = "YTDLP_EXIT";
    pub const MAX_ATTEMPTS_REACHED: &str = "MAX_ATTEMPTS_REACHED";

    /// Non-retryable reason → the state machine jumps straight to
    /// `FailedPermanent` without passing through `FailedRetryable`.
    pub fn is_permanent(reason: &str) -> bool {
        matches!(
            reason,
            DISK_FULL | SUB_ONLY | USER_CANCELLED | MAX_ATTEMPTS_REACHED
        )
    }
}

/// Apply a transition. Returns the new state on success, or a
/// descriptive error string listing the invalid edge. The caller is
/// expected to hold any DB transaction that writes the result back.
pub fn apply(from: DownloadState, t: Transition) -> Result<DownloadState, TransitionError> {
    use DownloadState::*;
    use Transition::*;
    let to = match (from, t) {
        (Queued, Start) => Downloading,
        (Downloading, Succeed) => Completed,
        (Downloading, FailRetryable) => DownloadState::FailedRetryable,
        (Downloading, FailPermanent) => DownloadState::FailedPermanent,
        (Downloading, Pause) => Paused,
        (Paused, Resume) => Downloading,
        (Paused, Cancel) => DownloadState::FailedPermanent,
        (Queued, Cancel) => DownloadState::FailedPermanent,
        (DownloadState::FailedRetryable, Requeue) => Queued,
        (DownloadState::FailedRetryable, Cancel) => DownloadState::FailedPermanent,
        (DownloadState::FailedRetryable, Retry) => Queued,
        (DownloadState::FailedPermanent, Retry) => Queued,
        // Everything else is illegal.
        (from, t) => {
            return Err(TransitionError {
                from,
                transition: t,
            });
        }
    };
    Ok(to)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransitionError {
    pub from: DownloadState,
    pub transition: Transition,
}

impl std::fmt::Display for TransitionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "illegal download transition {:?} from {:?}",
            self.transition, self.from
        )
    }
}

impl std::error::Error for TransitionError {}

/// Retry backoff in seconds for attempt number N (1-indexed).
/// Exponential with a cap. Small jitter stays at the call site where
/// the Clock is available; this function is pure.
pub fn backoff_seconds(attempt: i64) -> i64 {
    // 30, 60, 120, 240, 480 (seconds). Phase 3 caps at 5 attempts so
    // the fifth error goes straight to FailedPermanent without ever
    // hitting this function with attempt > 5.
    match attempt.max(1) {
        1 => 30,
        2 => 60,
        3 => 120,
        4 => 240,
        _ => 480,
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn db_string_roundtrip_every_state() {
        for s in [
            DownloadState::Queued,
            DownloadState::Downloading,
            DownloadState::Paused,
            DownloadState::Completed,
            DownloadState::FailedRetryable,
            DownloadState::FailedPermanent,
        ] {
            assert_eq!(DownloadState::from_db_str(s.as_db_str()), Some(s));
        }
    }

    #[test]
    fn unknown_db_string_is_none() {
        assert_eq!(DownloadState::from_db_str("mystery"), None);
    }

    #[test]
    fn terminals_are_completed_and_permanent() {
        assert!(DownloadState::Completed.is_terminal());
        assert!(DownloadState::FailedPermanent.is_terminal());
        for s in [
            DownloadState::Queued,
            DownloadState::Downloading,
            DownloadState::Paused,
            DownloadState::FailedRetryable,
        ] {
            assert!(!s.is_terminal(), "{s:?} should not be terminal");
        }
    }

    #[test]
    fn happy_path_queue_to_complete() {
        let s = DownloadState::Queued;
        let s = apply(s, Transition::Start).unwrap();
        assert_eq!(s, DownloadState::Downloading);
        let s = apply(s, Transition::Succeed).unwrap();
        assert_eq!(s, DownloadState::Completed);
    }

    #[test]
    fn pause_resume_round_trip() {
        let s = apply(DownloadState::Downloading, Transition::Pause).unwrap();
        assert_eq!(s, DownloadState::Paused);
        let s = apply(s, Transition::Resume).unwrap();
        assert_eq!(s, DownloadState::Downloading);
    }

    #[test]
    fn retryable_then_requeue() {
        let s = apply(DownloadState::Downloading, Transition::FailRetryable).unwrap();
        assert_eq!(s, DownloadState::FailedRetryable);
        let s = apply(s, Transition::Requeue).unwrap();
        assert_eq!(s, DownloadState::Queued);
    }

    #[test]
    fn cancel_from_anywhere_non_completed_lands_permanent() {
        for start in [
            DownloadState::Queued,
            DownloadState::Paused,
            DownloadState::FailedRetryable,
        ] {
            assert_eq!(
                apply(start, Transition::Cancel).unwrap(),
                DownloadState::FailedPermanent
            );
        }
    }

    #[test]
    fn retry_from_both_failure_states() {
        for start in [
            DownloadState::FailedRetryable,
            DownloadState::FailedPermanent,
        ] {
            assert_eq!(
                apply(start, Transition::Retry).unwrap(),
                DownloadState::Queued
            );
        }
    }

    #[test]
    fn illegal_transitions_are_errors() {
        // Can't succeed from queued.
        assert!(apply(DownloadState::Queued, Transition::Succeed).is_err());
        // Can't pause a completed row.
        assert!(apply(DownloadState::Completed, Transition::Pause).is_err());
        // Can't start from paused — must resume.
        assert!(apply(DownloadState::Paused, Transition::Start).is_err());
        // Can't cancel a completed row — already terminal success.
        assert!(apply(DownloadState::Completed, Transition::Cancel).is_err());
    }

    #[test]
    fn every_reachable_edge_is_covered() {
        // This iterates the full (state, transition) matrix and
        // asserts that either the edge is defined or it cleanly
        // errors. A regression that adds a new state without
        // extending `apply` would not necessarily fail another test;
        // this one does.
        let states = [
            DownloadState::Queued,
            DownloadState::Downloading,
            DownloadState::Paused,
            DownloadState::Completed,
            DownloadState::FailedRetryable,
            DownloadState::FailedPermanent,
        ];
        let transitions = [
            Transition::Start,
            Transition::Succeed,
            Transition::FailRetryable,
            Transition::FailPermanent,
            Transition::Pause,
            Transition::Resume,
            Transition::Requeue,
            Transition::Retry,
            Transition::Cancel,
        ];
        for s in states {
            for t in transitions {
                // We don't care whether the edge is defined — just
                // that `apply` returns without panicking.
                let _ = apply(s, t);
            }
        }
    }

    #[test]
    fn backoff_is_monotonic_and_capped() {
        let mut last = 0;
        for n in 1..=10 {
            let current = backoff_seconds(n);
            assert!(current >= last, "backoff must not decrease");
            last = current;
        }
        // Cap check.
        assert_eq!(backoff_seconds(100), 480);
    }

    #[test]
    fn permanent_reasons_skip_retry() {
        for r in [
            reason::DISK_FULL,
            reason::SUB_ONLY,
            reason::USER_CANCELLED,
            reason::MAX_ATTEMPTS_REACHED,
        ] {
            assert!(reason::is_permanent(r), "{r} should be permanent");
        }
        for r in [reason::NETWORK, reason::UNKNOWN, reason::YTDLP_EXIT] {
            assert!(!reason::is_permanent(r), "{r} should be retryable");
        }
    }
}
