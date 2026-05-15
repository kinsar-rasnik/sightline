/**
 * TimelinePage — the v2.1 Single-Time-Axis browse surface (ADR-0038).
 *
 * Replaces the Phase-4 one-lane-per-streamer proof-of-concept. Streams
 * are packed into overlap lanes around a shared time axis (D1); the
 * conditional hero billboard sits above the lanes (D9); a zoom chrome
 * drives the six D2 zoom levels; the Now-indicator marks the wall-clock
 * instant (D3); and a floating bar surfaces the Multiview multi-select
 * (D7).
 *
 * Keyboard cascade (CEO-A4): Tab-focus drives the same hover-preview
 * sequence as the mouse — that behaviour lives inside VodCard's
 * focus handling; this page only guarantees the lane-by-lane,
 * chronological-within Tab order (delegated to TimelineLanes).
 */

import { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import { ChevronRight, ZoomIn, ZoomOut } from "lucide-react";

import { ErrorBanner } from "@/components/primitives/ErrorBanner";
import { useStreamers } from "@/features/streamers/use-streamers";
import { useTimeline, useTimelineStats } from "@/features/timeline/use-timeline";
import type { VodCardStackPerspective } from "@/features/vods/VodCard";
import type { Interval, StreamerSummary, TimelineFilters } from "@/ipc";
import { useNavStore } from "@/stores/nav-store";
import { useTimelineMultiselectStore } from "@/stores/timeline-multiselect-store";
import {
  MAX_ZOOM_LEVEL,
  MIN_ZOOM_LEVEL,
  ZOOM_LEVEL_SPAN_SECONDS,
  useTimelineViewStore,
  type ZoomLevel,
} from "@/stores/timeline-view-store";

import {
  groupMultiPerspective,
  packIntoLanes,
  type MultiPerspectiveGroup,
} from "./lane-packing";
import { TimelineHeroSlot } from "./TimelineHeroSlot";
import { TimelineLanes, type TimelineRenderable } from "./TimelineLanes";
import { TimelineSelectionBar } from "./TimelineSelectionBar";

/** Lane-area width assumed before the container has been measured. */
const DEFAULT_AREA_WIDTH_PX = 1200;

const ZOOM_LEVEL_LABEL: Record<ZoomLevel, string> = {
  1: "15 min",
  2: "1 hour",
  3: "6 hours",
  4: "24 hours",
  5: "7 days",
  6: "30 days",
};

/** Re-render at the D3 live-tick cadence: 10 s at fine zoom, 30 s coarse. */
function useNowTick(zoomLevel: ZoomLevel): number {
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    const cadenceMs = zoomLevel <= 2 ? 10_000 : 30_000;
    const id = window.setInterval(() => setNow(Date.now()), cadenceMs);
    return () => window.clearInterval(id);
  }, [zoomLevel]);
  return now;
}

/** Resolve a multi-perspective group's members to streamer-avatar dots. */
function buildPerspectives(
  group: MultiPerspectiveGroup,
  streamers: StreamerSummary[] | undefined,
): VodCardStackPerspective[] {
  const seen = new Set<string>();
  const perspectives: VodCardStackPerspective[] = [];
  for (const member of group.members) {
    if (seen.has(member.streamerId)) continue;
    seen.add(member.streamerId);
    const summary = streamers?.find(
      (s) => s.streamer.twitchUserId === member.streamerId,
    );
    perspectives.push({
      twitchUserId: member.streamerId,
      displayName: summary?.streamer.displayName ?? member.streamerId,
      profileImageUrl: summary?.streamer.profileImageUrl ?? null,
    });
  }
  return perspectives;
}

export function TimelinePage() {
  const stats = useTimelineStats();
  const streamers = useStreamers();

  // Broad fetch window — the whole available range. D6 viewport
  // virtualisation narrows the rendered set client-side, so the IPC
  // query key stays stable across pans and zooms.
  const filters = useMemo<TimelineFilters>(() => {
    const nowSec = Math.floor(Date.now() / 1000);
    const until = stats.data?.latestEndAt ?? nowSec;
    const since = stats.data?.earliestStartAt ?? until - 30 * 86_400;
    return { since, until, streamerIds: null };
  }, [stats.data]);
  const timeline = useTimeline(filters);
  const intervals = useMemo<Interval[]>(
    () => timeline.data ?? [],
    [timeline.data],
  );

  const zoomLevel = useTimelineViewStore((s) => s.zoomLevel);
  const viewportStartAt = useTimelineViewStore((s) => s.viewportStartAt);
  const setZoom = useTimelineViewStore((s) => s.setZoom);
  const centerOnNow = useTimelineViewStore((s) => s.centerOnNow);

  const toggleSelect = useTimelineMultiselectStore((s) => s.toggle);
  const selectRange = useTimelineMultiselectStore((s) => s.selectRange);
  const clearSelection = useTimelineMultiselectStore((s) => s.clear);

  const now = useNowTick(zoomLevel);
  const nowSec = Math.floor(now / 1000);

  const viewportSpanSeconds = ZOOM_LEVEL_SPAN_SECONDS[zoomLevel];
  const viewportSince = Math.floor(viewportStartAt.getTime() / 1000);

  // Lane-packing + multi-perspective grouping. One lead card renders per
  // group; ungrouped intervals render individually (D1 + D5).
  const renderables = useMemo<TimelineRenderable[]>(() => {
    const groups = groupMultiPerspective(intervals);
    const leadByVod = new Map(groups.map((g) => [g.leadVodId, g]));
    const groupedVodIds = new Set(
      groups.flatMap((g) => g.members.map((m) => m.vodId)),
    );
    const renderableIntervals = intervals.filter(
      (iv) => !groupedVodIds.has(iv.vodId) || leadByVod.has(iv.vodId),
    );
    const assignments = packIntoLanes(renderableIntervals);
    const assignmentByVod = new Map(assignments.map((a) => [a.vodId, a]));

    const result: TimelineRenderable[] = [];
    for (const interval of renderableIntervals) {
      const assignment = assignmentByVod.get(interval.vodId);
      if (!assignment) continue;
      const group = leadByVod.get(interval.vodId);
      result.push({
        interval,
        assignment,
        group: group
          ? {
              memberVodIds: group.members.map((m) => m.vodId),
              perspectives: buildPerspectives(group, streamers.data),
            }
          : null,
      });
    }
    return result;
  }, [intervals, streamers.data]);

  // Lane-area width measurement — drives the time→pixel scale.
  const areaRef = useRef<HTMLDivElement>(null);
  const [areaWidth, setAreaWidth] = useState(0);
  useLayoutEffect(() => {
    const el = areaRef.current;
    if (!el) return;
    const measure = () => setAreaWidth(el.clientWidth);
    measure();
    window.addEventListener("resize", measure);
    return () => window.removeEventListener("resize", measure);
  }, []);
  const pxPerSecond =
    (areaWidth > 0 ? areaWidth : DEFAULT_AREA_WIDTH_PX) / viewportSpanSeconds;

  // D7: Escape clears the selection; leaving the page clears it too —
  // selections do not persist across page boundaries.
  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") clearSelection();
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [clearSelection]);
  useEffect(() => () => clearSelection(), [clearSelection]);

  const handleActivate = useCallback((vodId: string) => {
    useNavStore.getState().openPlayer({ vodId, autoplay: true });
  }, []);
  const handleToggleSelect = useCallback(
    (vodId: string) => toggleSelect(vodId),
    [toggleSelect],
  );
  const handleToggleSelectGroup = useCallback(
    (memberVodIds: string[]) => {
      const first = memberVodIds[0];
      const last = memberVodIds[memberVodIds.length - 1];
      if (first && last) selectRange(first, last, memberVodIds);
    },
    [selectRange],
  );

  const nowInViewport =
    nowSec >= viewportSince && nowSec <= viewportSince + viewportSpanSeconds;
  const nowOffsetPx = (nowSec - viewportSince) * pxPerSecond;

  const hasData = intervals.length > 0;

  return (
    <div className="relative flex flex-col gap-3">
      <TimelineHeroSlot
        streamers={streamers.data ?? []}
        intervals={intervals}
        onWatchLive={handleActivate}
      />

      <header className="flex items-baseline justify-between">
        <h2 className="text-lg font-title tracking-tight">Timeline</h2>
        <div
          role="toolbar"
          aria-label="Timeline zoom controls"
          className="flex items-center gap-2 text-sm"
        >
          <button
            type="button"
            onClick={() =>
              setZoom(Math.max(MIN_ZOOM_LEVEL, zoomLevel - 1) as ZoomLevel)
            }
            disabled={zoomLevel <= MIN_ZOOM_LEVEL}
            aria-label="Zoom in"
            className="inline-flex items-center rounded-sm bg-surface-2 p-1.5 text-fg disabled:opacity-40"
          >
            <ZoomIn width={16} height={16} strokeWidth={1.5} aria-hidden="true" />
          </button>
          <span className="font-tabular text-muted" aria-live="polite">
            {ZOOM_LEVEL_LABEL[zoomLevel]}
          </span>
          <button
            type="button"
            onClick={() =>
              setZoom(Math.min(MAX_ZOOM_LEVEL, zoomLevel + 1) as ZoomLevel)
            }
            disabled={zoomLevel >= MAX_ZOOM_LEVEL}
            aria-label="Zoom out"
            className="inline-flex items-center rounded-sm bg-surface-2 p-1.5 text-fg disabled:opacity-40"
          >
            <ZoomOut
              width={16}
              height={16}
              strokeWidth={1.5}
              aria-hidden="true"
            />
          </button>
          <button
            type="button"
            onClick={centerOnNow}
            aria-label="Jump to now"
            className="inline-flex items-center gap-1 rounded-sm bg-surface-2 px-2.5 py-1.5 text-fg"
          >
            <ChevronRight
              width={14}
              height={14}
              strokeWidth={1.5}
              aria-hidden="true"
            />
            Jump to now
          </button>
        </div>
      </header>

      <ErrorBanner error={timeline.error} />
      {timeline.isLoading && (
        <p className="text-sm text-muted" role="status">
          Loading timeline…
        </p>
      )}
      {timeline.data && !hasData && (
        <p className="text-sm text-muted">
          No intervals in range. As VODs are ingested, the timeline
          populates automatically.
        </p>
      )}

      {hasData && (
        <div
          ref={areaRef}
          role="region"
          aria-label="Stream timeline"
          className="relative overflow-hidden rounded-md bg-surface-1"
        >
          <TimelineLanes
            renderables={renderables}
            viewportSince={viewportSince}
            viewportSpanSeconds={viewportSpanSeconds}
            pxPerSecond={pxPerSecond}
            onActivate={handleActivate}
            onToggleSelect={handleToggleSelect}
            onToggleSelectGroup={handleToggleSelectGroup}
          />
          {nowInViewport && (
            <div
              aria-hidden="true"
              data-testid="now-indicator"
              className="pointer-events-none absolute bottom-0 top-0 z-[var(--z-now-indicator)]"
              style={{
                width: "var(--now-indicator-width)",
                background: "var(--now-indicator-color)",
                transform: `translateX(${nowOffsetPx}px)`,
              }}
            />
          )}
        </div>
      )}

      <TimelineSelectionBar />
    </div>
  );
}
