/**
 * timeline-view-store tests — D2 zoom levels and D3 Now-indicator
 * visibility.
 */

import { beforeEach, describe, expect, test } from "vitest";

import {
  DEFAULT_ZOOM_LEVEL,
  ZOOM_LEVEL_SPAN_SECONDS,
  useTimelineViewStore,
} from "./timeline-view-store";

const KNOWN_START = new Date("2026-05-01T00:00:00Z");

beforeEach(() => {
  useTimelineViewStore.setState({
    zoomLevel: 4,
    viewportStartAt: KNOWN_START,
    nowIndicatorVisible: false,
  });
});

describe("D2 zoom-level spans", () => {
  test("the span table matches the D2 level definitions", () => {
    expect(ZOOM_LEVEL_SPAN_SECONDS[1]).toBe(15 * 60);
    expect(ZOOM_LEVEL_SPAN_SECONDS[4]).toBe(24 * 60 * 60);
    expect(ZOOM_LEVEL_SPAN_SECONDS[6]).toBe(30 * 24 * 60 * 60);
  });

  test("the default zoom level is 24 h day-view", () => {
    expect(DEFAULT_ZOOM_LEVEL).toBe(4);
  });
});

describe("setZoom", () => {
  test("changes the level while holding the viewport centre fixed", () => {
    const before = useTimelineViewStore.getState();
    const centreBefore =
      before.viewportStartAt.getTime() +
      ZOOM_LEVEL_SPAN_SECONDS[before.zoomLevel] * 500;

    useTimelineViewStore.getState().setZoom(2);

    const after = useTimelineViewStore.getState();
    const centreAfter =
      after.viewportStartAt.getTime() +
      ZOOM_LEVEL_SPAN_SECONDS[after.zoomLevel] * 500;
    expect(after.zoomLevel).toBe(2);
    expect(centreAfter).toBe(centreBefore);
  });
});

describe("panTo", () => {
  test("moves the viewport left edge to the given instant", () => {
    const target = new Date("2026-04-20T12:00:00Z");
    useTimelineViewStore.getState().panTo(target);
    expect(useTimelineViewStore.getState().viewportStartAt).toBe(target);
  });
});

describe("centerOnNow", () => {
  test("re-centres so the Now-indicator is visible (D3)", () => {
    useTimelineViewStore.getState().centerOnNow();
    const state = useTimelineViewStore.getState();
    expect(state.nowIndicatorVisible).toBe(true);

    // Now must fall inside the recentred window.
    const startSec = Math.floor(state.viewportStartAt.getTime() / 1000);
    const nowSec = Math.floor(Date.now() / 1000);
    expect(nowSec).toBeGreaterThanOrEqual(startSec);
    expect(nowSec).toBeLessThanOrEqual(
      startSec + ZOOM_LEVEL_SPAN_SECONDS[state.zoomLevel],
    );
  });
});
