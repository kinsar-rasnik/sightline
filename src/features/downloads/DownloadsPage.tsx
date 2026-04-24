import { useState } from "react";

import { Button } from "@/components/primitives/Button";
import { ErrorBanner } from "@/components/primitives/ErrorBanner";
import type { DownloadRow, DownloadState } from "@/ipc";
import {
  formatBytes,
  formatEta,
  formatSpeed,
  formatUnixSeconds,
} from "@/lib/format";

import {
  useCancelDownload,
  useDownloads,
  usePauseDownload,
  useResumeDownload,
  useRetryDownload,
} from "./use-downloads";

type StateFilter = "all" | DownloadState;

const STATE_OPTIONS: Array<{ key: StateFilter; label: string }> = [
  { key: "all", label: "All" },
  { key: "queued", label: "Queued" },
  { key: "downloading", label: "Downloading" },
  { key: "paused", label: "Paused" },
  { key: "completed", label: "Completed" },
  { key: "failed_retryable", label: "Retrying" },
  { key: "failed_permanent", label: "Failed" },
];

export function DownloadsPage() {
  const [stateFilter, setStateFilter] = useState<StateFilter>("all");
  const filters = stateFilter === "all" ? {} : { state: stateFilter };
  const downloads = useDownloads(filters);

  return (
    <section aria-labelledby="downloads-heading" className="space-y-4">
      <div className="flex items-baseline gap-3">
        <h2 id="downloads-heading" className="text-base font-medium">
          Downloads
        </h2>
        <span className="text-xs text-[--color-muted]">
          {downloads.data
            ? `${downloads.data.length} row${downloads.data.length === 1 ? "" : "s"}`
            : ""}
        </span>
      </div>

      <div className="flex flex-wrap gap-2" role="toolbar" aria-label="State filters">
        {STATE_OPTIONS.map((opt) => (
          <button
            key={opt.key}
            type="button"
            aria-pressed={stateFilter === opt.key}
            onClick={() => setStateFilter(opt.key)}
            className={`text-xs px-3 py-1.5 rounded-full border transition-colors ${
              stateFilter === opt.key
                ? "bg-[--color-accent] text-white border-transparent"
                : "bg-transparent text-[--color-fg] border-[--color-border] hover:bg-[--color-surface]"
            }`}
          >
            {opt.label}
          </button>
        ))}
      </div>

      {downloads.isLoading && (
        <p className="text-sm text-[--color-muted]" role="status">
          Loading queue…
        </p>
      )}
      <ErrorBanner error={downloads.error} />
      {downloads.data?.length === 0 && (
        <p className="text-sm text-[--color-muted]">
          Nothing in the queue yet. Open the Library, pick a VOD, and hit
          Download.
        </p>
      )}

      {downloads.data && downloads.data.length > 0 && (
        <div className="overflow-x-auto rounded border border-[--color-border] bg-[--color-surface]">
          <table className="w-full text-sm">
            <thead className="bg-[--color-bg] text-xs text-[--color-muted]">
              <tr>
                <th scope="col" className="text-left px-3 py-2 font-medium">
                  Title
                </th>
                <th scope="col" className="text-left px-3 py-2 font-medium">
                  Streamer
                </th>
                <th scope="col" className="text-left px-3 py-2 font-medium">
                  State
                </th>
                <th scope="col" className="text-left px-3 py-2 font-medium">
                  Progress
                </th>
                <th scope="col" className="text-left px-3 py-2 font-medium">
                  Speed
                </th>
                <th scope="col" className="text-left px-3 py-2 font-medium">
                  ETA
                </th>
                <th scope="col" className="text-left px-3 py-2 font-medium">
                  Actions
                </th>
              </tr>
            </thead>
            <tbody className="divide-y divide-[--color-border]">
              {downloads.data.map((row) => (
                <DownloadRow key={row.vodId} row={row} />
              ))}
            </tbody>
          </table>
        </div>
      )}
    </section>
  );
}

function DownloadRow({ row }: { row: DownloadRow }) {
  const pause = usePauseDownload();
  const resume = useResumeDownload();
  const cancel = useCancelDownload();
  const retry = useRetryDownload();

  const progressPct = (() => {
    if (!row.bytesTotal || row.bytesTotal <= 0) return null;
    return Math.round((row.bytesDone / row.bytesTotal) * 100);
  })();

  return (
    <tr>
      <td className="px-3 py-2 align-top max-w-0 truncate" title={row.title}>
        <div className="font-medium truncate">{row.title}</div>
        <div className="text-[10px] text-[--color-muted] font-mono">
          twitch-{row.vodId} · {row.qualityResolved ?? row.qualityPreset}
          {row.lastError ? ` · ${row.lastError}` : ""}
        </div>
      </td>
      <td className="px-3 py-2 align-top text-[--color-muted] text-xs">
        {row.streamerDisplayName}
      </td>
      <td className="px-3 py-2 align-top">
        <StateBadge state={row.state} />
      </td>
      <td className="px-3 py-2 align-top text-xs w-48">
        {progressPct != null ? (
          <div>
            <div className="h-2 bg-[--color-bg] rounded overflow-hidden">
              <div
                className="h-full bg-[--color-accent]"
                style={{ width: `${progressPct}%` }}
              />
            </div>
            <div className="text-[10px] text-[--color-muted] mt-1">
              {progressPct}% · {formatBytes(row.bytesDone)} /{" "}
              {formatBytes(row.bytesTotal)}
            </div>
          </div>
        ) : (
          <span className="text-[--color-muted]">–</span>
        )}
      </td>
      <td className="px-3 py-2 align-top text-xs text-[--color-muted]">
        {formatSpeed(row.speedBps)}
      </td>
      <td className="px-3 py-2 align-top text-xs text-[--color-muted]">
        {formatEta(row.etaSeconds)}
      </td>
      <td className="px-3 py-2 align-top">
        <div className="flex flex-wrap gap-1.5">
          {row.state === "downloading" && (
            <Button
              variant="secondary"
              onClick={() => pause.mutate(row.vodId)}
              disabled={pause.isPending}
            >
              Pause
            </Button>
          )}
          {row.state === "paused" && (
            <Button
              variant="primary"
              onClick={() => resume.mutate(row.vodId)}
              disabled={resume.isPending}
            >
              Resume
            </Button>
          )}
          {(row.state === "failed_retryable" ||
            row.state === "failed_permanent") && (
            <Button
              variant="primary"
              onClick={() => retry.mutate(row.vodId)}
              disabled={retry.isPending}
            >
              Retry
            </Button>
          )}
          {row.state !== "completed" && (
            <Button
              variant="danger"
              onClick={() => cancel.mutate(row.vodId)}
              disabled={cancel.isPending}
            >
              {row.state === "failed_permanent" ? "Remove" : "Cancel"}
            </Button>
          )}
          {row.state === "completed" && (
            <span
              className="text-xs text-[--color-muted]"
              title={row.finalPath ?? undefined}
            >
              {formatUnixSeconds(row.finishedAt)}
            </span>
          )}
        </div>
      </td>
    </tr>
  );
}

function StateBadge({ state }: { state: DownloadState }) {
  const palette: Record<DownloadState, string> = {
    queued:
      "bg-[--color-bg] text-[--color-muted] border border-[--color-border]",
    downloading: "bg-blue-500/20 text-blue-300 border border-blue-500/30",
    paused: "bg-amber-500/20 text-amber-300 border border-amber-500/30",
    completed: "bg-emerald-500/20 text-emerald-300 border border-emerald-500/30",
    failed_retryable:
      "bg-orange-500/20 text-orange-300 border border-orange-500/30",
    failed_permanent: "bg-red-500/20 text-red-400 border border-red-500/30",
  };
  const label: Record<DownloadState, string> = {
    queued: "Queued",
    downloading: "Downloading",
    paused: "Paused",
    completed: "Completed",
    failed_retryable: "Retrying",
    failed_permanent: "Failed",
  };
  return (
    <span
      className={`text-[10px] uppercase tracking-wider px-1.5 py-0.5 rounded ${palette[state]}`}
    >
      {label[state]}
    </span>
  );
}
