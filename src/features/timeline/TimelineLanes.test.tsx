/**
 * TimelineLanes tests — D6 viewport virtualisation and the D7 / DV9
 * lane-by-lane, chronological-within Tab order.
 *
 * Cards far outside the render-buffer must be unmounted; the cards that
 * remain must appear in DOM order matching keyboard Tab order.
 */

import { afterEach, describe, expect, test, vi } from "vitest";
import { cleanup, render } from "@testing-library/react";

import type { Interval } from "@/ipc";

import type { LaneAssignment } from "./lane-packing";
import {
  isWithinRenderBuffer,
  TimelineLanes,
  type TimelineRenderable,
} from "./TimelineLanes";

vi.mock("@/features/vods/hooks/useVodSummary", () => {
  function makeSummary(vodId: string) {
    return {
      vod: {
        vod: {
          twitchVideoId: vodId,
          twitchUserId: `u_${vodId}`,
          title: `VOD ${vodId}`,
          status: "ready",
          durationSeconds: 7200,
          streamStartedAt: 1_700_000_000,
          thumbnailUrl: null,
        },
        chapters: [],
        streamerDisplayName: `Streamer ${vodId}`,
        streamerLogin: vodId,
      },
      streamer: null,
      assets: null,
      watchProgress: null,
      retention: { stage: 1, retentionAt: 0, remainingSeconds: 999_999 },
    };
  }
  return {
    useVodSummary: (vodId: string | null) => ({
      data: vodId === null ? null : makeSummary(vodId),
      isLoading: false,
      error: null,
    }),
  };
});

function renderable(
  vodId: string,
  startAt: number,
  endAt: number,
  assignment: LaneAssignment,
): TimelineRenderable {
  const interval: Interval = { vodId, streamerId: `s_${vodId}`, startAt, endAt };
  return { interval, assignment, group: null };
}

const NOOP = {
  onActivate: vi.fn(),
  onToggleSelect: vi.fn(),
  onToggleSelectGroup: vi.fn(),
};

afterEach(cleanup);

describe("isWithinRenderBuffer (D6)", () => {
  test("the buffer extends half a viewport-width past each edge", () => {
    // viewport [1000, 1100]; buffer ±50 → rendered window [950, 1150].
    const within = { vodId: "a", streamerId: "s", startAt: 1140, endAt: 1160 };
    const past = { vodId: "b", streamerId: "s", startAt: 0, endAt: 100 };
    expect(isWithinRenderBuffer(within, 1000, 100)).toBe(true);
    expect(isWithinRenderBuffer(past, 1000, 100)).toBe(false);
  });
});

describe("TimelineLanes — D6 viewport virtualisation", () => {
  test("intervals outside the render-buffer are not mounted", () => {
    const renderables = [
      renderable("inside", 1010, 1050, { vodId: "inside", side: "above", laneIdx: 0 }),
      renderable("far-future", 9000, 9100, { vodId: "far-future", side: "above", laneIdx: 0 }),
      renderable("far-past", 0, 50, { vodId: "far-past", side: "below", laneIdx: 0 }),
    ];
    const { container } = render(
      <TimelineLanes
        renderables={renderables}
        viewportSince={1000}
        viewportSpanSeconds={100}
        pxPerSecond={1}
        {...NOOP}
      />,
    );
    expect(container.querySelector('[data-timeline-card="inside"]')).not.toBeNull();
    expect(container.querySelector('[data-timeline-card="far-future"]')).toBeNull();
    expect(container.querySelector('[data-timeline-card="far-past"]')).toBeNull();
  });
});

describe("TimelineLanes — D7 / DV9 Tab order", () => {
  test("cards render lane-by-lane, chronological within each lane", () => {
    // Two cards share above[0]; one each in above[1] and below[0].
    const renderables = [
      renderable("a2", 1030, 1050, { vodId: "a2", side: "above", laneIdx: 0 }),
      renderable("c1", 1000, 1020, { vodId: "c1", side: "below", laneIdx: 0 }),
      renderable("a1", 1010, 1025, { vodId: "a1", side: "above", laneIdx: 0 }),
      renderable("b1", 1005, 1040, { vodId: "b1", side: "above", laneIdx: 1 }),
    ];
    const { container } = render(
      <TimelineLanes
        renderables={renderables}
        viewportSince={1000}
        viewportSpanSeconds={100}
        pxPerSecond={1}
        {...NOOP}
      />,
    );
    const order = [...container.querySelectorAll("[data-timeline-card]")].map(
      (el) => el.getAttribute("data-timeline-card"),
    );
    // above[0] by startAt (a1, a2) → above[1] (b1) → below[0] (c1).
    expect(order).toEqual(["a1", "a2", "b1", "c1"]);
  });
});
