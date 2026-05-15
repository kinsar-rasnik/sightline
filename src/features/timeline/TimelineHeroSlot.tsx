/**
 * TimelineHeroSlot — the conditional "Happening now" billboard above the
 * lane area (ADR-0038 D9 + ADR-0040 TV10).
 *
 * Layout-reservation trigger (D9): the slot is reserved if and only if
 * at least one *favourited* streamer is currently live — not merely any
 * tracked streamer. When multiple favourited streamers are live the
 * slot picks the most-recently-started one (TV-A6 tie-break, via
 * `lastLiveAt`). When no favourited streamer is live the component
 * renders nothing — total collapse, zero DOM reservation (TV10).
 *
 * R-ADR-01 note. The brief named the data fields `is_favourited` /
 * `is_live`; the IPC contract exposes `StreamerSummary.streamer.favorite`
 * and `StreamerSummary.liveNow`. D9's favourited-AND-live substance is
 * unchanged — only the field names are mapped to the real contract.
 */

import { VodCard } from "@/features/vods/VodCard";
import type { Interval, StreamerSummary } from "@/ipc";

/**
 * D9 + TV-A6: the favourited-AND-live streamer that owns the hero slot,
 * or `null` when none qualifies. Ties break to the most-recently-started
 * stream (`lastLiveAt` descending).
 */
export function selectHeroStreamer(
  streamers: readonly StreamerSummary[],
): StreamerSummary | null {
  const live = streamers.filter(
    (s) => s.streamer.favorite === true && s.liveNow === true,
  );
  if (live.length === 0) return null;
  return live.reduce((best, candidate) =>
    (candidate.streamer.lastLiveAt ?? 0) > (best.streamer.lastLiveAt ?? 0)
      ? candidate
      : best,
  );
}

/** Most-recently-started interval belonging to `streamerId`, or `null`. */
function latestIntervalFor(
  intervals: readonly Interval[],
  streamerId: string,
): Interval | null {
  let latest: Interval | null = null;
  for (const interval of intervals) {
    if (interval.streamerId !== streamerId) continue;
    if (latest === null || interval.startAt > latest.startAt) {
      latest = interval;
    }
  }
  return latest;
}

interface TimelineHeroSlotProps {
  streamers: StreamerSummary[];
  intervals: Interval[];
  /** Watch-live CTA — opens the hero VOD. */
  onWatchLive: (vodId: string) => void;
}

export function TimelineHeroSlot({
  streamers,
  intervals,
  onWatchLive,
}: TimelineHeroSlotProps) {
  const hero = selectHeroStreamer(streamers);
  const heroInterval = hero
    ? latestIntervalFor(intervals, hero.streamer.twitchUserId)
    : null;

  // Total collapse (TV10): no DOM reservation when no favourited
  // streamer is live, or the hero streamer has no interval to surface.
  if (heroInterval === null) return null;

  const heroVodId = heroInterval.vodId;
  return (
    <div className="mb-4" data-testid="timeline-hero-slot">
      <VodCard
        vodId={heroVodId}
        variant="hero"
        hero={{ onWatchLive: () => onWatchLive(heroVodId) }}
      />
    </div>
  );
}
