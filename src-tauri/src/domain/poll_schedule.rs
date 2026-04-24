//! Pure poll-schedule decision logic.
//!
//! Given a streamer's last-live and last-polled timestamps plus the
//! configured floor/recent/ceiling intervals, decide when the next poll
//! should run. The scheduler service calls this for every streamer and
//! persists `next_poll_at` on the row; no tokio, no RNG side effects
//! leak out of here.
//!
//! The jitter implementation is deterministic given a seed so that tests
//! can assert exact intervals.

use serde::{Deserialize, Serialize};
use specta::Type;

/// Configurable interval knobs, plus a jitter ratio. Intervals are in
/// seconds; `jitter_ratio` is in the range [0.0, 1.0] and represents the
/// maximum fraction of the interval that may be added (positive or
/// negative) as jitter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PollIntervals {
    pub floor_seconds: i64,
    pub recent_seconds: i64,
    pub ceiling_seconds: i64,
}

impl PollIntervals {
    pub const DEFAULT_JITTER_BPS: i64 = 1000; // 10.00%

    /// Sightline's defaults. Mirrors the `app_settings` row.
    pub fn defaults() -> Self {
        Self {
            floor_seconds: 600,
            recent_seconds: 1800,
            ceiling_seconds: 7200,
        }
    }

    /// Saturating floor <= recent <= ceiling invariant. The caller is
    /// expected to validate on the persistence boundary; this method
    /// is a safety net.
    pub fn normalized(self) -> Self {
        let floor = self.floor_seconds.max(60);
        let recent = self.recent_seconds.max(floor);
        let ceiling = self.ceiling_seconds.max(recent);
        Self {
            floor_seconds: floor,
            recent_seconds: recent,
            ceiling_seconds: ceiling,
        }
    }
}

/// Streamer state that drives the decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StreamerState {
    pub now_unix: i64,
    pub last_live_at: Option<i64>,
    pub last_polled_at: Option<i64>,
    pub live_now: bool,
}

/// Decide the *base* interval (pre-jitter) for a streamer. Step function:
///   - `live_now == true`             → floor
///   - live within last 24h           → recent
///   - live > 24h ago / never observed → ceiling
pub fn base_interval_seconds(intervals: &PollIntervals, state: &StreamerState) -> i64 {
    let i = intervals.clone().normalized();

    if state.live_now {
        return i.floor_seconds;
    }

    match state.last_live_at {
        Some(last) if state.now_unix.saturating_sub(last) <= 24 * 3600 => i.recent_seconds,
        _ => i.ceiling_seconds,
    }
}

/// Apply deterministic jitter. `seed` is a u64 mixed with the streamer's
/// identity (caller's responsibility) so each streamer desynchronizes
/// from the others and the thundering herd is damped.
///
/// `jitter_bps` is expressed in basis points (1/100 of 1%): e.g. `1000`
/// means `±10%`.
pub fn apply_jitter(base: i64, seed: u64, jitter_bps: i64) -> i64 {
    let clamp = jitter_bps.clamp(0, 10_000) as u64;
    if clamp == 0 || base == 0 {
        return base;
    }
    // Deterministic PRNG: xorshift64 mixes the seed, projected into the
    // range [-clamp, +clamp] basis points.
    let mut x = seed | 1;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    let magnitude = (x % (2 * clamp + 1)) as i64 - clamp as i64;
    // delta = base * magnitude / 10_000
    let delta = base.saturating_mul(magnitude) / 10_000;
    (base.saturating_add(delta)).max(1)
}

/// Convenience: compute the absolute unix timestamp of the next poll.
pub fn next_poll_at(
    intervals: &PollIntervals,
    state: &StreamerState,
    seed: u64,
    jitter_bps: i64,
) -> i64 {
    let base = base_interval_seconds(intervals, state);
    let jittered = apply_jitter(base, seed, jitter_bps);
    state.now_unix.saturating_add(jittered)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn intervals() -> PollIntervals {
        PollIntervals::defaults()
    }

    #[test]
    fn live_uses_floor() {
        let out = base_interval_seconds(
            &intervals(),
            &StreamerState {
                now_unix: 1_000_000,
                last_live_at: Some(999_000),
                last_polled_at: None,
                live_now: true,
            },
        );
        assert_eq!(out, 600);
    }

    #[test]
    fn recent_uses_recent() {
        let out = base_interval_seconds(
            &intervals(),
            &StreamerState {
                now_unix: 1_000_000,
                last_live_at: Some(1_000_000 - 3_000), // 50 min ago
                last_polled_at: None,
                live_now: false,
            },
        );
        assert_eq!(out, 1800);
    }

    #[test]
    fn dormant_uses_ceiling() {
        let out = base_interval_seconds(
            &intervals(),
            &StreamerState {
                now_unix: 1_000_000,
                last_live_at: Some(1_000_000 - 48 * 3600),
                last_polled_at: None,
                live_now: false,
            },
        );
        assert_eq!(out, 7200);
    }

    #[test]
    fn never_seen_uses_ceiling() {
        let out = base_interval_seconds(
            &intervals(),
            &StreamerState {
                now_unix: 1_000_000,
                last_live_at: None,
                last_polled_at: None,
                live_now: false,
            },
        );
        assert_eq!(out, 7200);
    }

    #[test]
    fn normalization_enforces_monotone_bounds() {
        let out = PollIntervals {
            floor_seconds: 120,
            recent_seconds: 60,
            ceiling_seconds: 30,
        }
        .normalized();
        assert!(out.floor_seconds <= out.recent_seconds);
        assert!(out.recent_seconds <= out.ceiling_seconds);
    }

    #[test]
    fn jitter_stays_within_bounds() {
        let base = 1_800;
        for seed in 1u64..50 {
            let out = apply_jitter(base, seed, PollIntervals::DEFAULT_JITTER_BPS);
            let lower = base - (base * 10 / 100);
            let upper = base + (base * 10 / 100);
            assert!(
                (lower..=upper).contains(&out),
                "seed={seed}: {out} not in [{lower}, {upper}]"
            );
        }
    }

    #[test]
    fn jitter_is_deterministic() {
        let a = apply_jitter(1800, 42, 1000);
        let b = apply_jitter(1800, 42, 1000);
        assert_eq!(a, b);
    }

    #[test]
    fn zero_jitter_returns_base() {
        assert_eq!(apply_jitter(1800, 7, 0), 1800);
    }

    #[test]
    fn next_poll_at_respects_now() {
        let state = StreamerState {
            now_unix: 1_000_000,
            last_live_at: Some(1_000_000 - 300),
            last_polled_at: None,
            live_now: true,
        };
        let t = next_poll_at(&intervals(), &state, 7, PollIntervals::DEFAULT_JITTER_BPS);
        assert!(t > state.now_unix);
        assert!(t <= state.now_unix + 700, "live interval + 10% jitter");
    }
}
