import type { CleanupLogEntry } from "@/ipc";
import { formatBytes, formatUnixSeconds } from "@/lib/format";

export function CleanupHistoryView({
  entries,
}: {
  entries: CleanupLogEntry[] | undefined;
}) {
  if (!entries || entries.length === 0) {
    return (
      <div className="text-xs text-[--color-muted]">
        No cleanup history yet.
      </div>
    );
  }
  return (
    <div className="text-xs space-y-1">
      <div className="text-[--color-muted] font-medium">
        Recent cleanup runs
      </div>
      <ul className="space-y-1">
        {entries.map((e) => (
          <li
            key={e.id}
            className="flex items-center justify-between border-b border-[--color-border] py-1"
          >
            <div>
              <span className="font-mono">{formatUnixSeconds(e.ranAt)}</span>
              <span className="text-[--color-muted]"> · {e.mode}</span>
            </div>
            <div className="flex items-center gap-3">
              <span>{formatBytes(e.freedBytes)}</span>
              <span className="text-[--color-muted]">
                {e.deletedVodCount} VODs
              </span>
              <span
                className={
                  e.status === "ok"
                    ? "text-emerald-600"
                    : e.status === "skipped"
                      ? "text-[--color-muted]"
                      : e.status === "partial"
                        ? "text-amber-600"
                        : "text-red-500"
                }
              >
                {e.status}
              </span>
            </div>
          </li>
        ))}
      </ul>
    </div>
  );
}
