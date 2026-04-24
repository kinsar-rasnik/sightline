//! Cross-streamer deep-link math (Phase 5 / ADR-0020).
//!
//! The detail-drawer's co-stream panel has a "Watch this perspective
//! at HH:MM:SS" action that opens the *other* streamer's VOD seeked
//! to the shared wall-clock moment. The math is tiny — we compute
//! the offset of the current moment from the current VOD's start,
//! and apply it to the other VOD's start — but it has to be pure and
//! testable so the Phase-6 multi-view sync engine can reuse it.
//!
//! Wall-clock inputs are UTC unix seconds. The output is a position
//! in seconds into the target VOD, clamped into
//! `[0, other.duration_seconds]`. A negative offset (target started
//! later than the moment we care about) clamps to 0; an offset past
//! the end clamps to the final frame.

/// Input context for `resolve_deep_link_target`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DeepLinkContext {
    /// Unix seconds of the moment the user clicked "watch this
    /// perspective at". Usually `current_vod.stream_started_at +
    /// current_position_seconds`.
    pub moment_unix_seconds: i64,
    /// Unix seconds when the *target* VOD's stream started.
    pub target_stream_started_at: i64,
    /// Duration of the target VOD, in seconds. Used to clamp the
    /// result — a deep-link that lands past the end of the target
    /// VOD shouldn't leave the player in an undefined state.
    pub target_duration_seconds: i64,
}

/// Resolve the seek position the player should jump to on the
/// target VOD. Returns a non-negative value clamped to
/// `[0, target_duration_seconds]`.
pub fn resolve_deep_link_target(ctx: DeepLinkContext) -> f64 {
    let offset = (ctx.moment_unix_seconds - ctx.target_stream_started_at).max(0) as f64;
    let duration = (ctx.target_duration_seconds).max(0) as f64;
    offset.min(duration)
}

/// Formatter for the detail-drawer's action label. Separate from the
/// seek math so the UI text is unit-testable.
pub fn format_deep_link_label(seek_seconds: f64) -> String {
    let total = seek_seconds.floor() as i64;
    let hours = total / 3600;
    let minutes = (total % 3600) / 60;
    let seconds = total % 60;
    if hours > 0 {
        format!("Watch this perspective at {hours:02}:{minutes:02}:{seconds:02}")
    } else {
        format!("Watch this perspective at {minutes:02}:{seconds:02}")
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn simple_offset_is_preserved() {
        let ctx = DeepLinkContext {
            moment_unix_seconds: 1_700_000_100, // 100 s after the shared start
            target_stream_started_at: 1_700_000_000,
            target_duration_seconds: 3600,
        };
        assert_eq!(resolve_deep_link_target(ctx), 100.0);
    }

    #[test]
    fn negative_offset_clamps_to_zero() {
        // The target streamer started 30 s later than the moment we
        // want — we can't seek to before the beginning.
        let ctx = DeepLinkContext {
            moment_unix_seconds: 1_700_000_000,
            target_stream_started_at: 1_700_000_030,
            target_duration_seconds: 3600,
        };
        assert_eq!(resolve_deep_link_target(ctx), 0.0);
    }

    #[test]
    fn offset_past_end_clamps_to_duration() {
        let ctx = DeepLinkContext {
            moment_unix_seconds: 1_700_010_000, // 10 000 s in
            target_stream_started_at: 1_700_000_000,
            target_duration_seconds: 3600,
        };
        assert_eq!(resolve_deep_link_target(ctx), 3600.0);
    }

    #[test]
    fn zero_duration_pins_to_zero() {
        let ctx = DeepLinkContext {
            moment_unix_seconds: 1_700_000_100,
            target_stream_started_at: 1_700_000_000,
            target_duration_seconds: 0,
        };
        assert_eq!(resolve_deep_link_target(ctx), 0.0);
    }

    #[test]
    fn label_formats_hours_when_non_zero() {
        assert_eq!(
            format_deep_link_label(3725.7),
            "Watch this perspective at 01:02:05"
        );
    }

    #[test]
    fn label_drops_hours_when_under_an_hour() {
        assert_eq!(
            format_deep_link_label(125.0),
            "Watch this perspective at 02:05"
        );
    }

    #[test]
    fn label_handles_zero() {
        assert_eq!(
            format_deep_link_label(0.0),
            "Watch this perspective at 00:00"
        );
    }

    // Timezone / DST edge-case coverage: the math is UTC-seconds-based
    // so there are no DST transitions to worry about — but we keep a
    // test that exercises a DST-crossing moment to lock that invariant
    // in case someone ever introduces a timezone-aware path.
    #[test]
    fn dst_transition_is_a_no_op_at_the_unix_layer() {
        // 2026-03-08 02:00:00 America/Los_Angeles crosses into PDT.
        // The unix seconds are monotonically increasing either way.
        let pre = 1_773_187_200; // 2026-03-08 01:00:00 PST
        let post = 1_773_190_800; // 2026-03-08 03:00:00 PDT (skipped 2am)
        let ctx = DeepLinkContext {
            moment_unix_seconds: post,
            target_stream_started_at: pre,
            target_duration_seconds: 7200,
        };
        // 3600 s between them — the DST skip doesn't change the
        // elapsed wall-clock duration, only what a local clock reads.
        assert_eq!(resolve_deep_link_target(ctx), 3600.0);
    }
}
