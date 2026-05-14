import type { ContinueWatchingEntry } from "@/ipc";

import { VodCard } from "@/features/vods/VodCard";

export interface ContinueWatchingRowProps {
  entries: ContinueWatchingEntry[];
  onPlay: (entry: ContinueWatchingEntry) => void;
}

/**
 * Horizontal scrolling row surfaced above the library grid. Hidden
 * entirely when there's nothing to show — the mission spec is
 * explicit that an empty row doesn't take up space.
 *
 * Wave-4 Mission 2 migration: the inner card is now
 * `<VodCard variant="compact" />` per the v2.1 VodCard rewrite
 * (ADR-0039 DV1). The entry data already contains everything the
 * compact variant needs to render (title, streamer name, thumbnail,
 * duration, watchedFraction); we wire it through VodCard's optional
 * `data` override to avoid an extra `useVodSummary` IPC round-trip
 * per entry.
 */
export function ContinueWatchingRow({
  entries,
  onPlay,
}: ContinueWatchingRowProps) {
  if (entries.length === 0) return null;
  return (
    <section aria-labelledby="cw-heading" className="space-y-2">
      <h2 id="cw-heading" className="text-sm font-semibold">
        Continue Watching
      </h2>
      <ul
        className="flex gap-3 overflow-x-auto pb-2"
        aria-label="Continue watching"
      >
        {entries.map((e) => (
          <li key={e.vodId} className="shrink-0 w-56">
            <VodCard
              vodId={e.vodId}
              variant="compact"
              onSelect={() => onPlay(e)}
              className="w-full"
            />
          </li>
        ))}
      </ul>
    </section>
  );
}
