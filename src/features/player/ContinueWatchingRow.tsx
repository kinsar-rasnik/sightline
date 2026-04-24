import type { ContinueWatchingEntry } from "@/ipc";
import { formatDurationSeconds } from "@/lib/format";

export interface ContinueWatchingRowProps {
  entries: ContinueWatchingEntry[];
  onPlay: (entry: ContinueWatchingEntry) => void;
}

/**
 * Horizontal scrolling row surfaced above the library grid. Hidden
 * entirely when there's nothing to show — the mission spec is
 * explicit that an empty row doesn't take up space.
 */
export function ContinueWatchingRow({ entries, onPlay }: ContinueWatchingRowProps) {
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
            <button
              type="button"
              onClick={() => onPlay(e)}
              aria-label={`Resume ${e.title} — ${formatDurationSeconds(Math.floor(e.remainingSeconds))} left`}
              className="group block w-full text-left bg-[--color-surface] border border-[--color-border] rounded-[var(--radius-md)] overflow-hidden hover:border-[--color-muted] focus-visible:outline focus-visible:outline-2 focus-visible:outline-[--color-accent]"
            >
              <div className="aspect-video bg-[--color-bg] relative overflow-hidden">
                {e.thumbnailUrl ? (
                  <img
                    src={e.thumbnailUrl}
                    alt=""
                    aria-hidden="true"
                    className="absolute inset-0 w-full h-full object-cover"
                    loading="lazy"
                  />
                ) : (
                  <div
                    aria-hidden="true"
                    className="absolute inset-0 flex items-center justify-center text-xs text-[--color-muted]"
                  >
                    No thumbnail
                  </div>
                )}
                <div
                  role="progressbar"
                  aria-label="Watch progress"
                  aria-valuemin={0}
                  aria-valuemax={100}
                  aria-valuenow={Math.round(e.watchedFraction * 100)}
                  className="absolute left-0 right-0 bottom-0 h-1 bg-black/40"
                >
                  <div
                    className="h-full bg-[--color-accent]"
                    style={{
                      width: `${Math.min(100, e.watchedFraction * 100)}%`,
                    }}
                  />
                </div>
              </div>
              <div className="p-2 space-y-1">
                <p
                  className="text-xs font-medium truncate"
                  title={e.title}
                >
                  {e.title}
                </p>
                <p className="text-[10px] text-[--color-muted] flex justify-between">
                  <span className="truncate">{e.streamerDisplayName}</span>
                  <span className="font-mono">
                    {formatDurationSeconds(Math.floor(e.remainingSeconds))}{" "}
                    left
                  </span>
                </p>
              </div>
            </button>
          </li>
        ))}
      </ul>
    </section>
  );
}
