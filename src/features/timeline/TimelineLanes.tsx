/**
 * TimelineLanes — the lane area of the Single-Time-Axis timeline
 * (ADR-0038 D6 + D8).
 *
 * Consumes the lane-packing result and lays cards out around the time
 * axis: `above` lanes stack upward, `below` lanes stack downward, lane 0
 * on each side nearest the axis spine. Cards are absolutely positioned
 * via `transform: translateX` so panning stays on the GPU compositor.
 *
 * Viewport virtualisation (D6): only renderables intersecting the
 * render-buffer — the visible window plus a half-width of padding each
 * side — are mounted; the rest are unmounted. Visible renderables are
 * ordered lane-by-lane, chronological within a lane, so keyboard
 * Tab-focus walks the timeline in the D7 / DV9 order.
 */

import { useMemo } from "react";

import type { VodCardStackPerspective } from "@/features/vods/VodCard";
import type { Interval } from "@/ipc";

import type { LaneAssignment, LaneSide } from "./lane-packing";
import { TimelineCard } from "./TimelineCard";
import { TimelineStackMarker } from "./TimelineStackMarker";

/** TV-A1: lane height. */
export const LANE_HEIGHT_PX = 72;
/** TV3 `--space-2`: vertical gap between adjacent lanes. */
export const INTER_LANE_PX = 8;
const LANE_PITCH_PX = LANE_HEIGHT_PX + INTER_LANE_PX;

/** D6: the render-buffer pads the visible window by half a width each side. */
export const RENDER_BUFFER_FRACTION = 0.5;

export interface MultiPerspectiveRenderGroup {
  /** Every member VOD id, longest-running first. */
  memberVodIds: string[];
  /** Streamer identities for the stack avatar-dots row (DV4). */
  perspectives: VodCardStackPerspective[];
}

export interface TimelineRenderable {
  interval: Interval;
  assignment: LaneAssignment;
  /** Non-null when this renderable anchors a multi-perspective stack. */
  group: MultiPerspectiveRenderGroup | null;
}

interface TimelineLanesProps {
  renderables: TimelineRenderable[];
  /** Left edge of the visible window, unix seconds. */
  viewportSince: number;
  /** Visible window width in seconds (the zoom-level span). */
  viewportSpanSeconds: number;
  /** Time→pixel scale for the lane area. */
  pxPerSecond: number;
  onActivate: (vodId: string) => void;
  onToggleSelect: (vodId: string) => void;
  onToggleSelectGroup: (memberVodIds: string[]) => void;
}

/** D6: whether an interval intersects the render-buffer window. */
export function isWithinRenderBuffer(
  interval: Interval,
  viewportSince: number,
  viewportSpanSeconds: number,
): boolean {
  const buffer = RENDER_BUFFER_FRACTION * viewportSpanSeconds;
  const renderedSince = viewportSince - buffer;
  const renderedUntil = viewportSince + viewportSpanSeconds + buffer;
  return interval.endAt >= renderedSince && interval.startAt <= renderedUntil;
}

function sideRank(side: LaneSide): number {
  return side === "above" ? 0 : 1;
}

export function TimelineLanes({
  renderables,
  viewportSince,
  viewportSpanSeconds,
  pxPerSecond,
  onActivate,
  onToggleSelect,
  onToggleSelectGroup,
}: TimelineLanesProps) {
  const { aboveCount, belowCount } = useMemo(() => {
    let above = 0;
    let below = 0;
    for (const r of renderables) {
      if (r.assignment.side === "above") {
        above = Math.max(above, r.assignment.laneIdx + 1);
      } else {
        below = Math.max(below, r.assignment.laneIdx + 1);
      }
    }
    return { aboveCount: above, belowCount: below };
  }, [renderables]);

  const axisY = aboveCount * LANE_PITCH_PX;
  const totalHeight = (aboveCount + belowCount) * LANE_PITCH_PX;

  const visible = useMemo(() => {
    return renderables
      .filter((r) =>
        isWithinRenderBuffer(r.interval, viewportSince, viewportSpanSeconds),
      )
      .sort((a, b) => {
        const bySide = sideRank(a.assignment.side) - sideRank(b.assignment.side);
        if (bySide !== 0) return bySide;
        if (a.assignment.laneIdx !== b.assignment.laneIdx) {
          return a.assignment.laneIdx - b.assignment.laneIdx;
        }
        return a.interval.startAt - b.interval.startAt;
      });
  }, [renderables, viewportSince, viewportSpanSeconds]);

  const topFor = (assignment: LaneAssignment): number =>
    assignment.side === "above"
      ? (aboveCount - 1 - assignment.laneIdx) * LANE_PITCH_PX
      : axisY + assignment.laneIdx * LANE_PITCH_PX;

  return (
    <div
      className="relative"
      style={{ height: Math.max(totalHeight, LANE_PITCH_PX) }}
    >
      {/* §3.6 time-axis spine — the centre line the lanes pack around. */}
      <div
        aria-hidden="true"
        className="absolute inset-x-0 bg-[var(--color-border)]"
        style={{ top: axisY, height: 1 }}
      />
      {visible.map((r) => {
        const offsetPx = (r.interval.startAt - viewportSince) * pxPerSecond;
        const widthPx =
          (r.interval.endAt - r.interval.startAt) * pxPerSecond;
        const topPx = topFor(r.assignment);
        if (r.group) {
          return (
            <TimelineStackMarker
              key={r.interval.vodId}
              leadVodId={r.interval.vodId}
              memberVodIds={r.group.memberVodIds}
              perspectives={r.group.perspectives}
              widthPx={widthPx}
              offsetPx={offsetPx}
              topPx={topPx}
              onActivate={onActivate}
              onToggleSelectGroup={onToggleSelectGroup}
            />
          );
        }
        return (
          <TimelineCard
            key={r.interval.vodId}
            vodId={r.interval.vodId}
            widthPx={widthPx}
            offsetPx={offsetPx}
            topPx={topPx}
            onActivate={onActivate}
            onToggleSelect={onToggleSelect}
          />
        );
      })}
    </div>
  );
}
