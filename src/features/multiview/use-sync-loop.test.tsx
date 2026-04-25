/**
 * Wiring test for `useSyncLoop`. The pure decision math has its own
 * coverage in `sync-math.test.ts`; this file checks that the hook
 * reads currentTime, mutates it on a corrective seek, and invokes
 * the right side-effect callbacks.
 */
import { act, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type { PaneRuntime } from "./sync-store";
import { useSyncLoop } from "./use-sync-loop";

vi.mock("@/ipc", () => ({
  commands: {
    recordSyncDrift: vi.fn().mockResolvedValue(undefined),
    reportSyncOutOfRange: vi.fn().mockResolvedValue(undefined),
  },
}));

import { commands } from "@/ipc";

const mockedRecord = (commands as unknown as {
  recordSyncDrift: ReturnType<typeof vi.fn>;
}).recordSyncDrift;
const mockedReport = (commands as unknown as {
  reportSyncOutOfRange: ReturnType<typeof vi.fn>;
}).reportSyncOutOfRange;

beforeEach(() => {
  vi.useFakeTimers();
  mockedRecord.mockClear();
  mockedReport.mockClear();
});

afterEach(() => {
  vi.useRealTimers();
});

interface FakeVideo {
  currentTime: number;
  paused: boolean;
  pause: ReturnType<typeof vi.fn>;
}

function makeVideo(currentTime: number, paused = false): FakeVideo {
  return {
    currentTime,
    paused,
    pause: vi.fn(function (this: FakeVideo) {
      this.paused = true;
    }),
  };
}

function makePane(overrides: Partial<PaneRuntime>): PaneRuntime {
  return {
    paneIndex: overrides.paneIndex ?? 0,
    vodId: overrides.vodId ?? "v",
    volume: 1,
    muted: false,
    streamStartedAt: overrides.streamStartedAt ?? 0,
    durationSeconds: overrides.durationSeconds ?? 1000,
    outOfRange: overrides.outOfRange ?? false,
    lastDriftMs: null,
  };
}

describe("useSyncLoop wiring", () => {
  it("issues a corrective seek when drift exceeds the threshold", () => {
    const onOutOfRange = vi.fn();
    const onDriftCorrected = vi.fn();
    const leaderVideo = makeVideo(150); // moment = 1000 + 150 = 1150
    const followerVideo = makeVideo(150.6); // 600 ms ahead → drift
    const refs = new Map<number, HTMLVideoElement | null>([
      [0, leaderVideo as unknown as HTMLVideoElement],
      [1, followerVideo as unknown as HTMLVideoElement],
    ]);
    const videoRefs = { current: refs };

    renderHook(() =>
      useSyncLoop({
        sessionId: 42,
        leaderPaneIndex: 0,
        panes: [
          makePane({ paneIndex: 0, streamStartedAt: 1000, durationSeconds: 1000 }),
          makePane({ paneIndex: 1, streamStartedAt: 1000, durationSeconds: 1000 }),
        ],
        thresholdMs: 250,
        videoRefs,
        onOutOfRange,
        onDriftCorrected,
      })
    );

    act(() => {
      vi.advanceTimersByTime(250);
    });

    expect(followerVideo.currentTime).toBe(150);
    expect(onDriftCorrected).toHaveBeenCalledWith(1, expect.any(Number));
    expect(mockedRecord).toHaveBeenCalledTimes(1);
    expect(mockedRecord.mock.calls[0]?.[0]).toMatchObject({
      sessionId: 42,
      measurement: { paneIndex: 1 },
    });
  });

  it("does not correct twice within the cooldown window", () => {
    const onOutOfRange = vi.fn();
    const onDriftCorrected = vi.fn();
    const leaderVideo = makeVideo(150);
    const followerVideo = makeVideo(150.6);
    const refs = new Map<number, HTMLVideoElement | null>([
      [0, leaderVideo as unknown as HTMLVideoElement],
      [1, followerVideo as unknown as HTMLVideoElement],
    ]);
    const videoRefs = { current: refs };

    renderHook(() =>
      useSyncLoop({
        sessionId: 1,
        leaderPaneIndex: 0,
        panes: [
          makePane({ paneIndex: 0, streamStartedAt: 1000, durationSeconds: 1000 }),
          makePane({ paneIndex: 1, streamStartedAt: 1000, durationSeconds: 1000 }),
        ],
        thresholdMs: 250,
        videoRefs,
        onOutOfRange,
        onDriftCorrected,
      })
    );

    // First tick: corrects.
    act(() => {
      vi.advanceTimersByTime(250);
    });
    expect(onDriftCorrected).toHaveBeenCalledTimes(1);
    // Re-introduce drift so the next decision is "would correct,
    // but cooldown not elapsed".
    followerVideo.currentTime = 150.6;
    // 250 ms after the first correction → still inside the 1000 ms
    // cooldown window.
    act(() => {
      vi.advanceTimersByTime(250);
    });
    expect(onDriftCorrected).toHaveBeenCalledTimes(1);
  });

  it("flags + reports a follower as out-of-range and pauses it", () => {
    const onOutOfRange = vi.fn();
    const onDriftCorrected = vi.fn();
    const leaderVideo = makeVideo(900); // moment = 1000 + 900 = 1900
    const followerVideo = makeVideo(0, false);
    const refs = new Map<number, HTMLVideoElement | null>([
      [0, leaderVideo as unknown as HTMLVideoElement],
      [1, followerVideo as unknown as HTMLVideoElement],
    ]);
    const videoRefs = { current: refs };

    renderHook(() =>
      useSyncLoop({
        sessionId: 99,
        leaderPaneIndex: 0,
        // Follower covers 0..100, leader is at moment 1900 → out of range.
        panes: [
          makePane({ paneIndex: 0, streamStartedAt: 1000, durationSeconds: 1000 }),
          makePane({ paneIndex: 1, streamStartedAt: 0, durationSeconds: 100 }),
        ],
        thresholdMs: 250,
        videoRefs,
        onOutOfRange,
        onDriftCorrected,
      })
    );

    act(() => {
      vi.advanceTimersByTime(250);
    });

    expect(onOutOfRange).toHaveBeenCalledWith(1, true);
    expect(followerVideo.pause).toHaveBeenCalledTimes(1);
    expect(mockedReport).toHaveBeenCalledWith({
      sessionId: 99,
      paneIndex: 1,
    });
    expect(onDriftCorrected).not.toHaveBeenCalled();
  });

  it("pauses the loop while document is hidden", () => {
    const onDriftCorrected = vi.fn();
    const leaderVideo = makeVideo(150);
    const followerVideo = makeVideo(150.6);
    const refs = new Map<number, HTMLVideoElement | null>([
      [0, leaderVideo as unknown as HTMLVideoElement],
      [1, followerVideo as unknown as HTMLVideoElement],
    ]);
    const videoRefs = { current: refs };

    Object.defineProperty(document, "visibilityState", {
      configurable: true,
      get: () => "hidden",
    });

    renderHook(() =>
      useSyncLoop({
        sessionId: 1,
        leaderPaneIndex: 0,
        panes: [
          makePane({ paneIndex: 0, streamStartedAt: 1000, durationSeconds: 1000 }),
          makePane({ paneIndex: 1, streamStartedAt: 1000, durationSeconds: 1000 }),
        ],
        thresholdMs: 250,
        videoRefs,
        onOutOfRange: vi.fn(),
        onDriftCorrected,
      })
    );

    act(() => {
      vi.advanceTimersByTime(1000);
    });

    expect(onDriftCorrected).not.toHaveBeenCalled();

    // Reset for the next test.
    Object.defineProperty(document, "visibilityState", {
      configurable: true,
      get: () => "visible",
    });
  });
});
