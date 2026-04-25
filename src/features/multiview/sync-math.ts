/**
 * Frontend mirror of `src-tauri/src/domain/deep_link.rs` and the
 * sync-specific helpers in `src-tauri/src/domain/sync.rs`. The Rust
 * implementations are canonical; these JS versions exist so the
 * 250 ms sync loop can run without an IPC round-trip per tick (see
 * ADR-0020 + ADR-0022).
 *
 * The shared fixture in `sync-math.fixture.json` is the contract:
 * any divergence between this file and the Rust math fails the
 * `parity-with-rust` test.
 */

/** Default drift tolerance in milliseconds; mirrored from the
 *  `app_settings.sync_drift_threshold_ms` default (ADR-0022). The
 *  runtime value comes from settings — this is the fallback when
 *  the settings query is still loading. */
export const DEFAULT_SYNC_DRIFT_THRESHOLD_MS = 250;

/** Sync-loop tick cadence in milliseconds. Matches the threshold by
 *  design — see ADR-0022 §Sync-loop cadence and the decision-log
 *  entry from 2026-04-25. */
export const SYNC_LOOP_TICK_MS = 250;

/** Minimum gap between corrective seeks per pane. Stops a runaway
 *  feedback loop where a seek triggers a measurement that triggers
 *  another seek. ADR-0022 §Correction strategy. */
export const SYNC_CORRECTION_COOLDOWN_MS = 1000;

export interface DeepLinkContext {
  momentUnixSeconds: number;
  targetStreamStartedAt: number;
  targetDurationSeconds: number;
}

/**
 * Resolve the seek position the player should jump to on the target
 * VOD. Returns a non-negative value clamped to
 * `[0, targetDurationSeconds]`. Mirrors
 * `domain::deep_link::resolve_deep_link_target`.
 */
export function resolveDeepLinkTarget(ctx: DeepLinkContext): number {
  const offset = Math.max(0, ctx.momentUnixSeconds - ctx.targetStreamStartedAt);
  const duration = Math.max(0, ctx.targetDurationSeconds);
  return Math.min(offset, duration);
}

export interface MemberRange {
  streamStartedAt: number;
  durationSeconds: number;
}

export interface OverlapWindow {
  startAt: number;
  endAt: number;
}

/**
 * Wall-clock intersection of N member ranges. Mirrors
 * `domain::sync::compute_overlap`. An empty input or disjoint
 * member set yields a degenerate window (`startAt === endAt`).
 */
export function computeOverlap(members: MemberRange[]): OverlapWindow {
  if (members.length === 0) {
    return { startAt: 0, endAt: 0 };
  }
  let startAt = -Infinity;
  let endAt = Infinity;
  for (const m of members) {
    if (m.streamStartedAt > startAt) startAt = m.streamStartedAt;
    const end = m.streamStartedAt + Math.max(0, m.durationSeconds);
    if (end < endAt) endAt = end;
  }
  if (endAt < startAt) endAt = startAt;
  return { startAt, endAt };
}

export function overlapDurationSeconds(window: OverlapWindow): number {
  return Math.max(0, window.endAt - window.startAt);
}

export function overlapIsNonEmpty(window: OverlapWindow): boolean {
  return window.endAt > window.startAt;
}

/**
 * Compute the expected `currentTime` for a follower pane given the
 * leader's wall-clock moment. Wraps `resolveDeepLinkTarget` with
 * the sync-loop's input vocabulary. Mirrors
 * `domain::sync::compute_expected_follower_position`.
 */
export function computeExpectedFollowerPosition(
  leaderMomentUnixSeconds: number,
  followerStreamStartedAt: number,
  followerDurationSeconds: number
): number {
  return resolveDeepLinkTarget({
    momentUnixSeconds: leaderMomentUnixSeconds,
    targetStreamStartedAt: followerStreamStartedAt,
    targetDurationSeconds: followerDurationSeconds,
  });
}

/**
 * Whether a follower pane is "out of range" for the given leader
 * moment — the moment falls outside the follower's own VOD's
 * wall-clock window. Mirrors `domain::sync::is_member_out_of_range`.
 */
export function isMemberOutOfRange(
  leaderMomentUnixSeconds: number,
  memberStreamStartedAt: number,
  memberDurationSeconds: number
): boolean {
  const endAt = memberStreamStartedAt + Math.max(0, memberDurationSeconds);
  return (
    leaderMomentUnixSeconds < memberStreamStartedAt ||
    leaderMomentUnixSeconds > endAt
  );
}

export interface DriftDecision {
  /** Whether the loop should issue a corrective seek on this tick. */
  shouldCorrect: boolean;
  /** Where the follower should land if we correct. */
  expectedPositionSeconds: number;
  /** Magnitude in milliseconds (signed). Positive = follower is
   *  ahead of expected; negative = follower is behind. */
  driftMs: number;
}

export interface DriftDecisionInput {
  /** Leader pane's own currentTime + stream_started_at give us the
   *  shared wall-clock moment. */
  leaderMomentUnixSeconds: number;
  followerStreamStartedAt: number;
  followerDurationSeconds: number;
  followerCurrentTimeSeconds: number;
  /** Drift tolerance in milliseconds (default 250 ms). */
  thresholdMs: number;
  /** Time since the last corrective seek on this pane, in ms. */
  msSinceLastCorrection: number;
}

/**
 * Decide whether to issue a corrective seek on a follower pane this
 * tick. Pure function — no side effects; the caller does the
 * `<video>` mutation.
 */
export function decideDriftCorrection(input: DriftDecisionInput): DriftDecision {
  const expected = computeExpectedFollowerPosition(
    input.leaderMomentUnixSeconds,
    input.followerStreamStartedAt,
    input.followerDurationSeconds
  );
  const driftMs = (input.followerCurrentTimeSeconds - expected) * 1000;
  const overThreshold = Math.abs(driftMs) > input.thresholdMs;
  const cooledDown = input.msSinceLastCorrection >= SYNC_CORRECTION_COOLDOWN_MS;
  return {
    shouldCorrect: overThreshold && cooledDown,
    expectedPositionSeconds: expected,
    driftMs,
  };
}
