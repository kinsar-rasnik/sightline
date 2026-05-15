// Timeline viewport state — zoom level + pan position + Now-indicator
// visibility for the v2.1 Single-Time-Axis timeline (ADR-0038 D2 + D3).
//
// R-ADR-01 note. The Mission-3 brief placed this store under
// `src/features/timeline/`; ADR-0038 D8's "Implementation Notes" lists
// it as `src/stores/timeline-view-store.ts`, and the frontend rule
// keeps cross-component Zustand stores in `src/stores/`. The spec
// source wins — the store lives here.

import { create } from "zustand";

/** D2: six discrete zoom levels. Level 4 (24 h) is the first-render default. */
export type ZoomLevel = 1 | 2 | 3 | 4 | 5 | 6;

/** D2: viewport-width of time per zoom level, in seconds. */
export const ZOOM_LEVEL_SPAN_SECONDS: Record<ZoomLevel, number> = {
  1: 15 * 60,
  2: 60 * 60,
  3: 6 * 60 * 60,
  4: 24 * 60 * 60,
  5: 7 * 24 * 60 * 60,
  6: 30 * 24 * 60 * 60,
};

export const MIN_ZOOM_LEVEL: ZoomLevel = 1;
export const MAX_ZOOM_LEVEL: ZoomLevel = 6;

/** D2: 24 h day-view is the default ("yesterday's roleplay" mental model). */
export const DEFAULT_ZOOM_LEVEL: ZoomLevel = 4;

/** D3: on centre, Now sits at 75 % of the viewport width. */
const NOW_ANCHOR_FRACTION = 0.75;

function spanSeconds(level: ZoomLevel): number {
  return ZOOM_LEVEL_SPAN_SECONDS[level];
}

/** Half the zoom-level span in milliseconds — the viewport-centre offset. */
function halfSpanMs(level: ZoomLevel): number {
  return (spanSeconds(level) * 1000) / 2;
}

/** `viewportStartAt` that places the wall-clock instant at the 75 % anchor. */
function centeredStart(level: ZoomLevel, nowMs: number): Date {
  return new Date(nowMs - spanSeconds(level) * NOW_ANCHOR_FRACTION * 1000);
}

/** Whether the wall-clock instant falls inside the visible window. */
function nowInWindow(start: Date, level: ZoomLevel, nowMs: number): boolean {
  const startMs = start.getTime();
  return nowMs >= startMs && nowMs <= startMs + spanSeconds(level) * 1000;
}

interface TimelineViewState {
  /** Active zoom level (D2). */
  zoomLevel: ZoomLevel;
  /** Left edge of the visible time window. */
  viewportStartAt: Date;
  /**
   * Whether the Now-indicator falls inside the window (D3). Recomputed
   * by every action; TimelinePage's Now-tick re-render keeps the
   * "Jump to now" CTA in sync between actions.
   */
  nowIndicatorVisible: boolean;
  /** Jump to a discrete zoom level, holding the viewport centre fixed. */
  setZoom: (level: ZoomLevel) => void;
  /** Pan the viewport so its left edge is `start`. */
  panTo: (start: Date) => void;
  /** Re-centre the viewport so Now sits at the 75 % anchor (D2 `Home`). */
  centerOnNow: () => void;
}

export const useTimelineViewStore = create<TimelineViewState>((set) => {
  const nowMs = Date.now();
  const initialStart = centeredStart(DEFAULT_ZOOM_LEVEL, nowMs);
  return {
    zoomLevel: DEFAULT_ZOOM_LEVEL,
    viewportStartAt: initialStart,
    nowIndicatorVisible: nowInWindow(initialStart, DEFAULT_ZOOM_LEVEL, nowMs),
    setZoom: (level) =>
      set((s) => {
        // Hold the viewport centre fixed across the zoom change.
        const centerMs = s.viewportStartAt.getTime() + halfSpanMs(s.zoomLevel);
        const start = new Date(centerMs - halfSpanMs(level));
        return {
          zoomLevel: level,
          viewportStartAt: start,
          nowIndicatorVisible: nowInWindow(start, level, Date.now()),
        };
      }),
    panTo: (start) =>
      set((s) => ({
        viewportStartAt: start,
        nowIndicatorVisible: nowInWindow(start, s.zoomLevel, Date.now()),
      })),
    centerOnNow: () =>
      set((s) => {
        const start = centeredStart(s.zoomLevel, Date.now());
        return { viewportStartAt: start, nowIndicatorVisible: true };
      }),
  };
});
