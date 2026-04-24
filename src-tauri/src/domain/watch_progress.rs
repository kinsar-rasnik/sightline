//! Watch-progress state machine + pure helpers (Phase 5 / ADR-0018).
//!
//! The state machine is small but it has enough off-by-one risks —
//! completion threshold, manually-watched vs. completed, resume
//! pre-roll, "if you're within the last 30 s treat this as done" —
//! that every transition lives in the domain layer with exhaustive
//! test coverage. The services layer only owns the DB I/O + the
//! debounced writer; it never decides a transition on its own.

use serde::{Deserialize, Serialize};
use specta::Type;

/// Canonical watch state. Wire strings match the CHECK constraint in
/// migration 0008.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum WatchState {
    Unwatched,
    InProgress,
    Completed,
    ManuallyWatched,
}

impl WatchState {
    pub fn as_db_str(self) -> &'static str {
        match self {
            WatchState::Unwatched => "unwatched",
            WatchState::InProgress => "in_progress",
            WatchState::Completed => "completed",
            WatchState::ManuallyWatched => "manually_watched",
        }
    }

    pub fn from_db_str(s: &str) -> Option<Self> {
        Some(match s {
            "unwatched" => WatchState::Unwatched,
            "in_progress" => WatchState::InProgress,
            "completed" => WatchState::Completed,
            "manually_watched" => WatchState::ManuallyWatched,
            _ => return None,
        })
    }

    /// True when the user has either finished the VOD organically or
    /// told Sightline to treat it as done. The Continue Watching row
    /// and the grid's watched-check icon both use this predicate.
    pub fn is_done(self) -> bool {
        matches!(self, WatchState::Completed | WatchState::ManuallyWatched)
    }
}

/// Settings inputs to the state machine. Kept as a single struct so
/// the transition function doesn't balloon into a 5-argument contract
/// and so the completion threshold can be unit-tested across the 70 %
/// – 100 % configurable range.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProgressSettings {
    /// Fraction (0.0..=1.0) at which `in_progress` transitions to
    /// `completed`. Mirrors `AppSettings.completion_threshold` /
    /// `app_settings.completion_threshold` (default 0.9).
    pub completion_threshold: f64,
    /// Pre-roll seconds subtracted at resume time. Clamped 0..=30.
    pub pre_roll_seconds: f64,
    /// If `position_seconds >= duration_seconds - restart_threshold_seconds`,
    /// `resume_position_for` returns 0 so the user starts from the top
    /// instead of landing in the last N seconds. The mission spec
    /// defaults this to 30 s.
    pub restart_threshold_seconds: f64,
}

impl Default for ProgressSettings {
    fn default() -> Self {
        Self {
            completion_threshold: 0.9,
            pre_roll_seconds: 5.0,
            restart_threshold_seconds: 30.0,
        }
    }
}

impl ProgressSettings {
    /// Sanitize the settings tuple so downstream code can assume in-
    /// range inputs. Clamping rather than erroring matches the UI
    /// invariant — the Settings page sliders already bound the inputs
    /// to the same ranges, so this is belt-and-braces.
    pub fn clamp(mut self) -> Self {
        self.completion_threshold = self.completion_threshold.clamp(0.7, 1.0);
        self.pre_roll_seconds = self.pre_roll_seconds.clamp(0.0, 30.0);
        self.restart_threshold_seconds = self.restart_threshold_seconds.max(0.0);
        self
    }
}

/// Snapshot of a progress update arriving from the player. The
/// service constructs one of these from a debounced `timeupdate` and
/// feeds it through `transition_on_update` to get the new state.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UpdateContext {
    pub current: WatchState,
    pub position_seconds: f64,
    pub duration_seconds: f64,
    pub settings: ProgressSettings,
}

/// State-machine step for a natural `timeupdate`. Returns the new
/// `WatchState` given the previous state + the fresh position.
pub fn transition_on_update(ctx: UpdateContext) -> WatchState {
    let settings = ctx.settings.clamp();
    let fraction = watched_fraction(ctx.position_seconds, ctx.duration_seconds);
    match ctx.current {
        // Once the user manually marks a row, organic updates never
        // push it back. This is a stronger guarantee than "completed
        // sticks" — mark-as-watched is a user choice.
        WatchState::ManuallyWatched => WatchState::ManuallyWatched,
        // Completed is sticky under timeupdate — only mark-as-unwatched
        // clears it (see `on_mark_unwatched`).
        WatchState::Completed => WatchState::Completed,
        WatchState::Unwatched if ctx.position_seconds > 0.0 => {
            if fraction >= settings.completion_threshold {
                WatchState::Completed
            } else {
                WatchState::InProgress
            }
        }
        WatchState::Unwatched => WatchState::Unwatched,
        WatchState::InProgress => {
            if fraction >= settings.completion_threshold {
                WatchState::Completed
            } else {
                WatchState::InProgress
            }
        }
    }
}

/// Manually-watched transition: user clicked "Mark as watched".
/// Always moves to `ManuallyWatched` and bumps the stored position
/// to the duration so the Continue Watching row drops the entry.
pub fn on_mark_watched(duration_seconds: f64) -> (WatchState, f64) {
    (WatchState::ManuallyWatched, duration_seconds.max(0.0))
}

/// Mark-as-unwatched transition: user clicked "Mark as unwatched".
/// Always moves to `Unwatched` and resets the position to 0.
pub fn on_mark_unwatched() -> (WatchState, f64) {
    (WatchState::Unwatched, 0.0)
}

/// Pure helper — watched fraction.
pub fn watched_fraction(position: f64, duration: f64) -> f64 {
    if duration > 0.0 {
        (position / duration).clamp(0.0, 1.0)
    } else {
        0.0
    }
}

/// Resume position math: on open, read `stored_position` and seek to
/// `max(0, stored - pre_roll)`; if we'd otherwise land in the last
/// `restart_threshold_seconds` of the VOD, return 0 so the user
/// starts from the top instead. Idiomatic pre-roll convention.
pub fn resume_position_for(
    stored_position: f64,
    duration_seconds: f64,
    settings: ProgressSettings,
) -> f64 {
    let settings = settings.clamp();
    if duration_seconds <= 0.0 {
        return 0.0;
    }
    if stored_position >= (duration_seconds - settings.restart_threshold_seconds).max(0.0) {
        return 0.0;
    }
    (stored_position - settings.pre_roll_seconds).max(0.0)
}

/// Round a position to 0.5-second resolution. Mirrors the mission
/// spec's "rounds to 0.5s resolution in storage to reduce write
/// amplification" — called by the services layer before every DB
/// write.
pub fn round_to_half_second(position: f64) -> f64 {
    (position * 2.0).round() / 2.0
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::float_cmp)]
mod tests {
    use super::*;

    fn ctx(current: WatchState, position: f64, duration: f64) -> UpdateContext {
        UpdateContext {
            current,
            position_seconds: position,
            duration_seconds: duration,
            settings: ProgressSettings::default(),
        }
    }

    #[test]
    fn unwatched_stays_put_on_zero_position() {
        assert_eq!(
            transition_on_update(ctx(WatchState::Unwatched, 0.0, 1000.0)),
            WatchState::Unwatched
        );
    }

    #[test]
    fn unwatched_flips_to_in_progress_on_first_tick() {
        assert_eq!(
            transition_on_update(ctx(WatchState::Unwatched, 1.0, 1000.0)),
            WatchState::InProgress
        );
    }

    #[test]
    fn in_progress_flips_to_completed_at_threshold() {
        // Default threshold is 0.9 — 900 s of a 1000 s VOD is the
        // exact boundary.
        assert_eq!(
            transition_on_update(ctx(WatchState::InProgress, 900.0, 1000.0)),
            WatchState::Completed
        );
    }

    #[test]
    fn in_progress_stays_in_progress_below_threshold() {
        assert_eq!(
            transition_on_update(ctx(WatchState::InProgress, 899.0, 1000.0)),
            WatchState::InProgress
        );
    }

    #[test]
    fn threshold_is_configurable() {
        let mut c = ctx(WatchState::InProgress, 700.0, 1000.0);
        c.settings.completion_threshold = 0.7;
        assert_eq!(transition_on_update(c), WatchState::Completed);
    }

    #[test]
    fn threshold_clamps_to_70_100() {
        // Settings below 70 % should clamp up; settings above 100 %
        // should clamp down.
        let mut c = ctx(WatchState::InProgress, 500.0, 1000.0);
        c.settings.completion_threshold = 0.5;
        assert_eq!(transition_on_update(c), WatchState::InProgress);
        c.settings.completion_threshold = 2.0;
        c.position_seconds = 999.9;
        assert_eq!(transition_on_update(c), WatchState::InProgress);
        c.position_seconds = 1000.0;
        assert_eq!(transition_on_update(c), WatchState::Completed);
    }

    #[test]
    fn completed_is_sticky_under_timeupdate() {
        // Scrubbing back into the middle of the VOD shouldn't un-complete it.
        assert_eq!(
            transition_on_update(ctx(WatchState::Completed, 100.0, 1000.0)),
            WatchState::Completed
        );
    }

    #[test]
    fn manually_watched_is_sticky_under_timeupdate() {
        assert_eq!(
            transition_on_update(ctx(WatchState::ManuallyWatched, 100.0, 1000.0)),
            WatchState::ManuallyWatched
        );
    }

    #[test]
    fn mark_watched_sets_position_to_duration() {
        let (state, pos) = on_mark_watched(1234.5);
        assert_eq!(state, WatchState::ManuallyWatched);
        assert_eq!(pos, 1234.5);
    }

    #[test]
    fn mark_unwatched_resets_everything() {
        let (state, pos) = on_mark_unwatched();
        assert_eq!(state, WatchState::Unwatched);
        assert_eq!(pos, 0.0);
    }

    #[test]
    fn resume_position_applies_pre_roll() {
        let s = ProgressSettings::default();
        assert_eq!(resume_position_for(100.0, 3600.0, s), 95.0);
    }

    #[test]
    fn resume_position_never_negative() {
        let s = ProgressSettings::default();
        assert_eq!(resume_position_for(3.0, 3600.0, s), 0.0);
    }

    #[test]
    fn resume_position_restarts_near_end() {
        let s = ProgressSettings::default();
        // 3590 s into a 3600 s VOD is inside the last 30 s — start from 0.
        assert_eq!(resume_position_for(3590.0, 3600.0, s), 0.0);
        // Exactly at the boundary also restarts (mission spec).
        assert_eq!(resume_position_for(3570.0, 3600.0, s), 0.0);
    }

    #[test]
    fn resume_position_handles_zero_duration() {
        let s = ProgressSettings::default();
        assert_eq!(resume_position_for(10.0, 0.0, s), 0.0);
    }

    #[test]
    fn round_to_half_second_is_idempotent() {
        assert_eq!(round_to_half_second(1.23), 1.0);
        assert_eq!(round_to_half_second(1.26), 1.5);
        assert_eq!(round_to_half_second(1.74), 1.5);
        assert_eq!(round_to_half_second(1.75), 2.0);
        assert_eq!(round_to_half_second(round_to_half_second(1.23)), 1.0);
    }

    #[test]
    fn state_db_round_trip() {
        for s in [
            WatchState::Unwatched,
            WatchState::InProgress,
            WatchState::Completed,
            WatchState::ManuallyWatched,
        ] {
            assert_eq!(WatchState::from_db_str(s.as_db_str()), Some(s));
        }
        assert_eq!(WatchState::from_db_str("bogus"), None);
    }

    #[test]
    fn is_done_matches_spec() {
        assert!(!WatchState::Unwatched.is_done());
        assert!(!WatchState::InProgress.is_done());
        assert!(WatchState::Completed.is_done());
        assert!(WatchState::ManuallyWatched.is_done());
    }
}
