import { useMemo, useState } from "react";

import { ErrorBanner } from "@/components/primitives/ErrorBanner";
import { useStreamers } from "@/features/streamers/use-streamers";
import { useTimeline, useTimelineStats } from "@/features/timeline/use-timeline";
import type { Interval } from "@/ipc";

/**
 * Timeline foundation view. Renders one lane per streamer with each
 * stream as a horizontal bar. Visual polish (zoom, drag, axis labels)
 * will be layered on in the Phase 4 follow-up; this first pass proves
 * the data layer + overlap detection and gives the user something to
 * click to preview Phase 6's multi-view sync.
 *
 * Performance guardrails:
 *   - intervals are range-filtered server-side (see ListTimelineInput).
 *   - the SVG uses viewport-relative widths; we render all lanes but
 *     only the intervals inside the visible `window` bounds per the
 *     stats call, keeping the DOM small for huge datasets.
 */
export function TimelinePage() {
  const stats = useTimelineStats();
  const streamers = useStreamers();
  const [filterStreamerId, setFilterStreamerId] = useState<string | null>(null);
  const [onlyOverlaps, setOnlyOverlaps] = useState(false);

  // Default range: last 30 days. Derived from stats so the user can
  // scroll back when there's older data.
  const { fromSeconds, toSeconds } = useMemo(() => {
    const now = Math.floor(Date.now() / 1000);
    const thirtyDays = 30 * 86_400;
    const to = stats.data?.latestEndAt ?? now;
    const from = Math.max(stats.data?.earliestStartAt ?? to - thirtyDays, to - thirtyDays);
    return { fromSeconds: from, toSeconds: to };
  }, [stats.data]);

  const timeline = useTimeline({
    since: fromSeconds,
    until: toSeconds,
    streamerIds: filterStreamerId ? [filterStreamerId] : null,
  });

  const laneIds = useMemo(() => {
    if (!timeline.data) return [];
    const set = new Set<string>();
    for (const iv of timeline.data) set.add(iv.streamerId);
    return [...set].sort();
  }, [timeline.data]);

  const byStreamer = useMemo(() => {
    const m = new Map<string, Interval[]>();
    if (timeline.data) {
      for (const iv of timeline.data) {
        const list = m.get(iv.streamerId) ?? [];
        list.push(iv);
        m.set(iv.streamerId, list);
      }
    }
    return m;
  }, [timeline.data]);

  const overlappingVodIds = useMemo(() => {
    if (!timeline.data) return new Set<string>();
    const list = [...timeline.data].sort((a, b) => a.startAt - b.startAt);
    const overlaps = new Set<string>();
    for (let i = 0; i < list.length; i++) {
      const a = list[i];
      if (!a) continue;
      for (let j = i + 1; j < list.length; j++) {
        const b = list[j];
        if (!b) continue;
        if (b.startAt >= a.endAt) break;
        if (b.streamerId !== a.streamerId) {
          overlaps.add(a.vodId);
          overlaps.add(b.vodId);
        }
      }
    }
    return overlaps;
  }, [timeline.data]);

  const displayName = (streamerId: string): string => {
    const s = streamers.data?.find((x) => x.streamer.twitchUserId === streamerId);
    return s?.streamer.displayName ?? streamerId;
  };

  return (
    <div className="space-y-4">
      <header className="flex items-baseline justify-between">
        <h2 className="text-lg font-semibold tracking-tight">Timeline</h2>
        <div className="text-xs text-[--color-muted]" aria-live="polite">
          {stats.data
            ? `${stats.data.totalIntervals} intervals · peak concurrent: ${stats.data.largestOverlapGroup}`
            : "…"}
        </div>
      </header>

      <div className="flex flex-wrap items-center gap-2" role="toolbar" aria-label="Timeline filters">
        <select
          aria-label="Filter by streamer"
          value={filterStreamerId ?? ""}
          onChange={(e) => setFilterStreamerId(e.target.value || null)}
          className="text-xs bg-[--color-surface] border border-[--color-border] rounded-full px-3 py-1.5"
        >
          <option value="">All streamers</option>
          {streamers.data?.map((s) => (
            <option key={s.streamer.twitchUserId} value={s.streamer.twitchUserId}>
              {s.streamer.displayName}
            </option>
          ))}
        </select>
        <label className="text-xs inline-flex items-center gap-2">
          <input
            type="checkbox"
            checked={onlyOverlaps}
            onChange={(e) => setOnlyOverlaps(e.target.checked)}
          />
          Only show streams with overlaps
        </label>
      </div>

      <ErrorBanner error={timeline.error} />
      {timeline.isLoading && (
        <p className="text-sm text-[--color-muted]" role="status">
          Loading timeline…
        </p>
      )}

      {timeline.data && timeline.data.length === 0 && (
        <p className="text-sm text-[--color-muted]">
          No intervals in the selected range. As VODs are ingested, the
          timeline index populates automatically.
        </p>
      )}

      {timeline.data && timeline.data.length > 0 && (
        <div
          role="region"
          aria-label="Stream timeline lanes"
          className="relative overflow-x-auto border border-[--color-border] rounded-[var(--radius-md)] bg-[--color-surface]"
        >
          <div className="sticky top-0 z-10 bg-[--color-surface] border-b border-[--color-border] px-3 py-2 text-[11px] text-[--color-muted] font-mono">
            {formatUnixDate(fromSeconds)} → {formatUnixDate(toSeconds)}
          </div>
          <ul className="divide-y divide-[--color-border]">
            {laneIds.map((streamerId) => {
              const lane = byStreamer.get(streamerId) ?? [];
              const visible = onlyOverlaps
                ? lane.filter((iv) => overlappingVodIds.has(iv.vodId))
                : lane;
              if (visible.length === 0 && onlyOverlaps) return null;
              return (
                <li key={streamerId} className="flex gap-3 px-3 py-2 items-center">
                  <div className="w-36 shrink-0 text-xs">
                    <div className="font-medium truncate">{displayName(streamerId)}</div>
                    <div className="text-[--color-muted]">
                      {visible.length} stream{visible.length === 1 ? "" : "s"}
                    </div>
                  </div>
                  <TimelineLane
                    intervals={visible}
                    fromSeconds={fromSeconds}
                    toSeconds={toSeconds}
                    overlappingIds={overlappingVodIds}
                  />
                </li>
              );
            })}
          </ul>
        </div>
      )}
    </div>
  );
}

function TimelineLane({
  intervals,
  fromSeconds,
  toSeconds,
  overlappingIds,
}: {
  intervals: Interval[];
  fromSeconds: number;
  toSeconds: number;
  overlappingIds: Set<string>;
}) {
  const span = Math.max(toSeconds - fromSeconds, 1);
  return (
    <div
      className="relative flex-1 h-10 bg-[--color-bg] rounded-sm"
      role="img"
      aria-label={`${intervals.length} stream intervals`}
    >
      {intervals.map((iv) => {
        const leftPct = ((iv.startAt - fromSeconds) / span) * 100;
        const widthPct = Math.max(((iv.endAt - iv.startAt) / span) * 100, 0.3);
        const overlapping = overlappingIds.has(iv.vodId);
        return (
          <button
            key={iv.vodId}
            type="button"
            className={`absolute top-1 bottom-1 rounded-sm text-[10px] text-white/80 truncate px-1 hover:ring-1 hover:ring-white/50 focus-visible:ring-2 focus-visible:ring-[--color-focus-ring] ${
              overlapping ? "bg-[--color-accent]" : "bg-[--color-info]"
            }`}
            style={{ left: `${leftPct}%`, width: `${widthPct}%` }}
            title={`${formatUnixDateTime(iv.startAt)} · ${Math.round((iv.endAt - iv.startAt) / 60)} min`}
          >
            <span className="sr-only">
              Stream {iv.vodId} from {formatUnixDateTime(iv.startAt)} to{" "}
              {formatUnixDateTime(iv.endAt)}
            </span>
          </button>
        );
      })}
    </div>
  );
}

function formatUnixDate(sec: number) {
  return new Date(sec * 1000).toLocaleDateString();
}

function formatUnixDateTime(sec: number) {
  return new Date(sec * 1000).toLocaleString();
}
