import { describe, expect, it } from "vitest";

import fixture from "./sync-math.fixture.json";
import {
  computeExpectedFollowerPosition,
  computeOverlap,
  decideDriftCorrection,
  isMemberOutOfRange,
  overlapDurationSeconds,
  overlapIsNonEmpty,
  resolveDeepLinkTarget,
  SYNC_CORRECTION_COOLDOWN_MS,
} from "./sync-math";

describe("resolveDeepLinkTarget — parity with Rust deep_link math", () => {
  for (const tc of fixture.deepLink) {
    it(tc.name, () => {
      const got = resolveDeepLinkTarget({
        momentUnixSeconds: tc.momentUnixSeconds,
        targetStreamStartedAt: tc.targetStreamStartedAt,
        targetDurationSeconds: tc.targetDurationSeconds,
      });
      expect(got).toBe(tc.expected);
    });
  }
});

describe("computeOverlap — parity with Rust domain::sync::compute_overlap", () => {
  for (const tc of fixture.overlap) {
    it(tc.name, () => {
      const got = computeOverlap(tc.members);
      expect(got.startAt).toBe(tc.expectedStartAt);
      expect(got.endAt).toBe(tc.expectedEndAt);
    });
  }

  it("empty input collapses to zero window", () => {
    const w = computeOverlap([]);
    expect(w.startAt).toBe(0);
    expect(w.endAt).toBe(0);
    expect(overlapIsNonEmpty(w)).toBe(false);
  });

  it("duration_seconds matches end - start", () => {
    expect(overlapDurationSeconds({ startAt: 100, endAt: 1000 })).toBe(900);
    expect(overlapDurationSeconds({ startAt: 100, endAt: 100 })).toBe(0);
  });
});

describe("isMemberOutOfRange — parity with Rust domain::sync", () => {
  for (const tc of fixture.outOfRange) {
    it(tc.name, () => {
      expect(
        isMemberOutOfRange(tc.moment, tc.memberStart, tc.memberDuration)
      ).toBe(tc.expected);
    });
  }
});

describe("computeExpectedFollowerPosition", () => {
  it("matches resolveDeepLinkTarget by construction", () => {
    const a = computeExpectedFollowerPosition(1700000100, 1700000000, 3600);
    const b = resolveDeepLinkTarget({
      momentUnixSeconds: 1700000100,
      targetStreamStartedAt: 1700000000,
      targetDurationSeconds: 3600,
    });
    expect(a).toBe(b);
  });
});

describe("decideDriftCorrection", () => {
  // Helper to build a follower at exactly the leader's expected
  // position (no drift) — leader at moment 1500, follower started
  // at 1000 with 5s duration → expected 500, mock follower
  // currentTime to test drift scenarios.
  function build(overrides: Partial<Parameters<typeof decideDriftCorrection>[0]>) {
    return {
      leaderMomentUnixSeconds: 1500,
      followerStreamStartedAt: 1000,
      followerDurationSeconds: 600,
      followerCurrentTimeSeconds: 500,
      thresholdMs: 250,
      msSinceLastCorrection: SYNC_CORRECTION_COOLDOWN_MS,
      ...overrides,
    };
  }

  it("zero drift does not correct", () => {
    const d = decideDriftCorrection(build({ followerCurrentTimeSeconds: 500 }));
    expect(d.shouldCorrect).toBe(false);
    expect(d.driftMs).toBe(0);
    expect(d.expectedPositionSeconds).toBe(500);
  });

  it("drift above threshold corrects when cooled down", () => {
    // 600 ms ahead — over the 250 ms threshold.
    const d = decideDriftCorrection(
      build({ followerCurrentTimeSeconds: 500.6 })
    );
    expect(d.shouldCorrect).toBe(true);
    expect(d.driftMs).toBeCloseTo(600);
  });

  it("drift above threshold does NOT correct during cooldown", () => {
    const d = decideDriftCorrection(
      build({
        followerCurrentTimeSeconds: 500.6,
        msSinceLastCorrection: 200, // < 1000 ms cooldown
      })
    );
    expect(d.shouldCorrect).toBe(false);
    expect(d.driftMs).toBeCloseTo(600);
  });

  it("drift below threshold never corrects", () => {
    const d = decideDriftCorrection(
      build({ followerCurrentTimeSeconds: 500.1 })
    );
    expect(d.shouldCorrect).toBe(false);
    expect(d.driftMs).toBeCloseTo(100);
  });

  it("negative drift (follower behind) corrects", () => {
    const d = decideDriftCorrection(
      build({ followerCurrentTimeSeconds: 499.5 })
    );
    expect(d.shouldCorrect).toBe(true);
    expect(d.driftMs).toBeCloseTo(-500);
  });

  it("expected position clamps to follower duration", () => {
    const d = decideDriftCorrection(
      build({
        leaderMomentUnixSeconds: 5000, // way past follower's end (1000+600 = 1600)
        followerCurrentTimeSeconds: 600,
      })
    );
    // expected = min(5000-1000, 600) = 600
    expect(d.expectedPositionSeconds).toBe(600);
  });
});
