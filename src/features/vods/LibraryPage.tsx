import { useMemo, useState } from "react";

import { Button } from "@/components/primitives/Button";
import { ErrorBanner } from "@/components/primitives/ErrorBanner";
import { useStreamers } from "@/features/streamers/use-streamers";
import { useVods } from "@/features/vods/use-vods";
import type { VodWithChapters } from "@/ipc";
import { formatDurationSeconds, formatUnixSeconds } from "@/lib/format";

type StatusFilter = "all" | "eligible" | "skipped_game" | "skipped_sub_only" | "skipped_live" | "error";

const STATUS_OPTIONS: Array<{ key: StatusFilter; label: string }> = [
  { key: "all", label: "All" },
  { key: "eligible", label: "Eligible" },
  { key: "skipped_game", label: "Skipped — game" },
  { key: "skipped_sub_only", label: "Sub-only" },
  { key: "skipped_live", label: "Live" },
  { key: "error", label: "Error" },
];

export function LibraryPage() {
  const [statusFilter, setStatusFilter] = useState<StatusFilter>("all");
  const [streamerId, setStreamerId] = useState<string | null>(null);
  const [selected, setSelected] = useState<string | null>(null);
  const streamers = useStreamers();

  const input = useMemo(() => {
    const statuses =
      statusFilter === "all" ? undefined : [statusFilter as string];
    const streamerIds = streamerId ? [streamerId] : undefined;
    return {
      filters: {
        streamer_ids: streamerIds,
        statuses,
        game_ids: undefined,
        since: undefined,
        until: undefined,
      },
      sort: "stream_started_at_desc" as const,
      limit: 200,
      offset: 0,
    };
  }, [statusFilter, streamerId]);

  const vods = useVods(input);

  return (
    <div className="flex gap-6 h-full">
      <div className="flex-1 min-w-0 space-y-4">
        <div className="flex flex-wrap gap-2" role="toolbar" aria-label="VOD filters">
          {STATUS_OPTIONS.map((opt) => (
            <button
              key={opt.key}
              type="button"
              aria-pressed={statusFilter === opt.key}
              onClick={() => setStatusFilter(opt.key)}
              className={`text-xs px-3 py-1.5 rounded-full border transition-colors ${
                statusFilter === opt.key
                  ? "bg-[--color-accent] text-white border-transparent"
                  : "bg-transparent text-[--color-fg] border-[--color-border] hover:bg-[--color-surface]"
              }`}
            >
              {opt.label}
            </button>
          ))}
          <select
            value={streamerId ?? ""}
            onChange={(e) => setStreamerId(e.target.value || null)}
            aria-label="Filter by streamer"
            className="text-xs bg-[--color-surface] border border-[--color-border] rounded-full px-3 py-1.5 focus:outline focus:outline-2 focus:outline-[--color-accent]"
          >
            <option value="">All streamers</option>
            {streamers.data?.map((s) => (
              <option key={s.streamer.twitch_user_id} value={s.streamer.twitch_user_id}>
                {s.streamer.display_name}
              </option>
            ))}
          </select>
        </div>

        {vods.isLoading && (
          <p className="text-sm text-[--color-muted]" role="status">
            Loading library…
          </p>
        )}
        <ErrorBanner error={vods.error} />
        {vods.data?.length === 0 && (
          <p className="text-sm text-[--color-muted]">
            No VODs match these filters yet. Add streamers and let the poller
            run, or trigger a poll from the Streamers page.
          </p>
        )}

        {vods.data && vods.data.length > 0 && (
          <ul className="divide-y divide-[--color-border] rounded border border-[--color-border] bg-[--color-surface] max-h-[calc(100vh-200px)] overflow-y-auto">
            {vods.data.map((v) => (
              <VodRow
                key={v.vod.twitch_video_id}
                row={v}
                selected={selected === v.vod.twitch_video_id}
                onSelect={() => setSelected(v.vod.twitch_video_id)}
              />
            ))}
          </ul>
        )}
      </div>

      <aside
        className="w-96 shrink-0 border-l border-[--color-border] pl-6"
        aria-label="VOD details"
      >
        <VodDetail
          vod={
            vods.data?.find((v) => v.vod.twitch_video_id === selected) ?? null
          }
        />
      </aside>
    </div>
  );
}

function VodRow({
  row,
  selected,
  onSelect,
}: {
  row: VodWithChapters;
  selected: boolean;
  onSelect: () => void;
}) {
  const v = row.vod;
  return (
    <li>
      <button
        type="button"
        onClick={onSelect}
        aria-current={selected ? "true" : undefined}
        className={`w-full text-left px-4 py-3 hover:bg-[--color-bg] focus:outline focus:outline-2 focus:outline-[--color-accent] ${
          selected ? "bg-[--color-bg]" : ""
        }`}
      >
        <div className="flex items-baseline justify-between gap-4">
          <span className="font-medium truncate">{v.title}</span>
          <span className="text-[10px] uppercase tracking-wider text-[--color-muted] whitespace-nowrap">
            {v.ingest_status}
          </span>
        </div>
        <div className="text-xs text-[--color-muted] mt-1 flex gap-4">
          <span>{row.streamer_display_name}</span>
          <span>{formatUnixSeconds(v.stream_started_at)}</span>
          <span>{formatDurationSeconds(v.duration_seconds)}</span>
          <span>{row.chapters.length} chapter{row.chapters.length === 1 ? "" : "s"}</span>
        </div>
      </button>
    </li>
  );
}

function VodDetail({ vod }: { vod: VodWithChapters | null }) {
  if (!vod) {
    return (
      <p className="text-sm text-[--color-muted]">
        Select a VOD to see its chapters and metadata.
      </p>
    );
  }
  const v = vod.vod;
  return (
    <div className="space-y-4">
      <div className="space-y-1">
        <h3 className="text-base font-medium">{v.title}</h3>
        <p className="text-xs text-[--color-muted]">
          {vod.streamer_display_name} · {formatUnixSeconds(v.stream_started_at)}
        </p>
      </div>
      <dl className="grid grid-cols-[max-content_1fr] gap-x-4 gap-y-1 text-xs">
        <dt className="text-[--color-muted]">Status</dt>
        <dd>{v.ingest_status}</dd>
        {v.status_reason && (
          <>
            <dt className="text-[--color-muted]">Reason</dt>
            <dd>{v.status_reason}</dd>
          </>
        )}
        <dt className="text-[--color-muted]">Duration</dt>
        <dd>{formatDurationSeconds(v.duration_seconds)}</dd>
        <dt className="text-[--color-muted]">Views</dt>
        <dd>{v.view_count.toLocaleString()}</dd>
        <dt className="text-[--color-muted]">Sub-only</dt>
        <dd>{v.is_sub_only ? "yes" : "no"}</dd>
      </dl>

      <a
        href={v.url}
        target="_blank"
        rel="noreferrer noopener"
        className="inline-block text-xs underline text-[--color-accent] focus:outline focus:outline-2 focus:outline-[--color-accent]"
      >
        Open on Twitch
      </a>

      <section aria-labelledby="chapters-heading" className="space-y-2">
        <h4 id="chapters-heading" className="text-sm font-medium">
          Chapters
        </h4>
        <ol className="space-y-1 text-xs">
          {vod.chapters.map((c, i) => (
            <li
              key={`${c.position_ms}-${i}`}
              className="flex justify-between gap-4 border-b border-[--color-border]/50 pb-1"
            >
              <span>
                {c.game_name || "Unknown game"}{" "}
                {c.chapter_type === "SYNTHETIC" && (
                  <span className="text-[--color-muted]">(single-game)</span>
                )}
              </span>
              <span className="font-mono text-[--color-muted]">
                {formatDurationSeconds(Math.floor(c.position_ms / 1000))}
              </span>
            </li>
          ))}
          {vod.chapters.length === 0 && (
            <li className="text-[--color-muted]">No chapters available.</li>
          )}
        </ol>
      </section>

      {vod.vod.is_sub_only && (
        <p className="text-xs text-[--color-muted] border-t border-[--color-border] pt-3">
          This VOD is restricted to the streamer's subscribers. Sightline
          won't retry downloading it, but it will re-check on every poll in
          case the streamer unlocks it.
        </p>
      )}
    </div>
  );
}
