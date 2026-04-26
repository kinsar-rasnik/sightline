import { renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import {
  PREFETCH_PROGRESS_THRESHOLD,
  PREFETCH_REMAINING_SECONDS_THRESHOLD,
  shouldTriggerPrefetch,
  usePrefetchHook,
} from "./use-prefetch-hook";

vi.mock("@/ipc", () => ({
  commands: {
    prefetchCheck: vi.fn(async () => ({
      triggered: false,
      prefetchedVodId: null,
    })),
  },
}));

import { commands } from "@/ipc";

describe("shouldTriggerPrefetch", () => {
  it("does not trigger before the progress threshold", () => {
    expect(
      shouldTriggerPrefetch({
        currentSeconds: 100,
        durationSeconds: 1000,
        alreadyTriggered: false,
      })
    ).toBe(false);
  });

  it("triggers at exactly the progress threshold", () => {
    expect(
      shouldTriggerPrefetch({
        currentSeconds: PREFETCH_PROGRESS_THRESHOLD * 1000,
        durationSeconds: 1000,
        alreadyTriggered: false,
      })
    ).toBe(true);
  });

  it("triggers when remaining time falls below the floor", () => {
    // Short clip: 5-min video at 4 min in (80% progress) — would
    // trigger via the progress branch too, but the remaining-floor
    // branch handles short-clip edge cases where the user lands
    // near the end before hitting 70%.
    expect(
      shouldTriggerPrefetch({
        currentSeconds: 240,
        durationSeconds: 360,
        alreadyTriggered: false,
      })
    ).toBe(true);

    // Boundary case: remaining exactly at the floor — must trigger
    // (we use `<=` so the boundary fires on the first sample at the
    // boundary).
    expect(
      shouldTriggerPrefetch({
        currentSeconds: 1000 - PREFETCH_REMAINING_SECONDS_THRESHOLD,
        durationSeconds: 1000,
        alreadyTriggered: false,
      })
    ).toBe(true);
  });

  it("does not trigger when alreadyTriggered (session throttle)", () => {
    expect(
      shouldTriggerPrefetch({
        currentSeconds: 999,
        durationSeconds: 1000,
        alreadyTriggered: true,
      })
    ).toBe(false);
  });

  it("does not trigger when duration is zero or non-finite", () => {
    expect(
      shouldTriggerPrefetch({
        currentSeconds: 0,
        durationSeconds: 0,
        alreadyTriggered: false,
      })
    ).toBe(false);
    expect(
      shouldTriggerPrefetch({
        currentSeconds: 100,
        durationSeconds: Number.NaN,
        alreadyTriggered: false,
      })
    ).toBe(false);
    expect(
      shouldTriggerPrefetch({
        currentSeconds: 100,
        durationSeconds: Number.POSITIVE_INFINITY,
        alreadyTriggered: false,
      })
    ).toBe(false);
  });

  it("does not trigger on negative or non-finite currentSeconds", () => {
    expect(
      shouldTriggerPrefetch({
        currentSeconds: -10,
        durationSeconds: 1000,
        alreadyTriggered: false,
      })
    ).toBe(false);
    expect(
      shouldTriggerPrefetch({
        currentSeconds: Number.NaN,
        durationSeconds: 1000,
        alreadyTriggered: false,
      })
    ).toBe(false);
  });
});

describe("usePrefetchHook", () => {
  beforeEach(() => {
    vi.mocked(commands.prefetchCheck).mockClear();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("invokes prefetchCheck once when the threshold is crossed", () => {
    const { rerender } = renderHook(
      ({ current }: { current: number }) =>
        usePrefetchHook({
          vodId: "v1",
          currentSeconds: current,
          durationSeconds: 1000,
        }),
      { initialProps: { current: 100 } }
    );
    expect(commands.prefetchCheck).not.toHaveBeenCalled();

    rerender({ current: 800 });
    expect(commands.prefetchCheck).toHaveBeenCalledTimes(1);
    expect(commands.prefetchCheck).toHaveBeenCalledWith({ vodId: "v1" });
  });

  it("throttles to one call per VOD per session", () => {
    const { rerender } = renderHook(
      ({ current }: { current: number }) =>
        usePrefetchHook({
          vodId: "v1",
          currentSeconds: current,
          durationSeconds: 1000,
        }),
      { initialProps: { current: 800 } }
    );
    expect(commands.prefetchCheck).toHaveBeenCalledTimes(1);

    // Subsequent ticks must not re-trigger even though the
    // condition still holds.
    rerender({ current: 850 });
    rerender({ current: 900 });
    rerender({ current: 950 });
    expect(commands.prefetchCheck).toHaveBeenCalledTimes(1);
  });

  it("re-triggers when the VOD changes (new session-key)", () => {
    const { rerender } = renderHook(
      ({ vodId, current }: { vodId: string; current: number }) =>
        usePrefetchHook({
          vodId,
          currentSeconds: current,
          durationSeconds: 1000,
        }),
      { initialProps: { vodId: "v1", current: 800 } }
    );
    expect(commands.prefetchCheck).toHaveBeenCalledTimes(1);

    rerender({ vodId: "v2", current: 800 });
    expect(commands.prefetchCheck).toHaveBeenCalledTimes(2);
    expect(commands.prefetchCheck).toHaveBeenLastCalledWith({ vodId: "v2" });
  });

  it("never fires when vodId is null", () => {
    renderHook(() =>
      usePrefetchHook({
        vodId: null,
        currentSeconds: 800,
        durationSeconds: 1000,
      })
    );
    expect(commands.prefetchCheck).not.toHaveBeenCalled();
  });

  it("swallows backend errors (best-effort hook)", async () => {
    vi.mocked(commands.prefetchCheck).mockRejectedValueOnce(
      new Error("backend unavailable")
    );
    const { rerender: _ } = renderHook(
      ({ current }: { current: number }) =>
        usePrefetchHook({
          vodId: "v1",
          currentSeconds: current,
          durationSeconds: 1000,
        }),
      { initialProps: { current: 800 } }
    );
    // Drain microtasks so the rejected promise's `.catch` runs
    // before we assert on the unhandled-rejection state.
    await new Promise((r) => setTimeout(r, 0));
    expect(commands.prefetchCheck).toHaveBeenCalledTimes(1);
  });
});
