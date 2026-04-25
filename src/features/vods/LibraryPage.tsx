import { useQuery } from "@tanstack/react-query";
import { useEffect, useMemo, useState } from "react";

import { Button } from "@/components/primitives/Button";
import { ErrorBanner } from "@/components/primitives/ErrorBanner";
import {
  useDownloads,
  useEnqueueDownload,
  useRetryDownload,
} from "@/features/downloads/use-downloads";
import { ContinueWatchingRow } from "@/features/player/ContinueWatchingRow";
import { useContinueWatching } from "@/features/player/use-watch-progress";
import { useStreamers } from "@/features/streamers/use-streamers";
import { useVods } from "@/features/vods/use-vods";
import { VodCard } from "@/features/vods/VodCard";
import {
  commands,
  type CoStream,
  type DownloadRow,
  type VodWithChapters,
} from "@/ipc";
import { formatDurationSeconds, formatUnixSeconds } from "@/lib/format";
import { useNavStore } from "@/stores/nav-store";

type StatusFilter =
  | "all"
  | "eligible"
  | "skipped_game"
  | "skipped_sub_only"
  | "skipped_live"
  | "error";

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

  const input = useMemo(
    () => ({
      filters: {
        ...(streamerId && { streamerIds: [streamerId] }),
        ...(statusFilter !== "all" && { statuses: [statusFilter] }),
      },
      sort: "stream_started_at_desc" as const,
      limit: 200,
      offset: 0,
    }),
    [statusFilter, streamerId]
  );

  const vods = useVods(input);
  const downloads = useDownloads({});
  const downloadsById = useMemo(() => {
    const m = new Map<string, DownloadRow>();
    for (const row of downloads.data ?? []) m.set(row.vodId, row);
    return m;
  }, [downloads.data]);
  const continueWatching = useContinueWatching(12);
  const openPlayer = useNavStore((s) => s.openPlayer);

  return (
    <div className="flex gap-6 h-full">
      <section
        className="flex-1 min-w-0 space-y-4"
        aria-label="Library grid"
      >
        <div
          className="flex flex-wrap gap-2"
          role="toolbar"
          aria-label="VOD filters"
        >
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
              <option
                key={s.streamer.twitchUserId}
                value={s.streamer.twitchUserId}
              >
                {s.streamer.displayName}
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

        <ContinueWatchingRow
          entries={continueWatching.data ?? []}
          onPlay={(entry) =>
            openPlayer({ vodId: entry.vodId, autoplay: true })
          }
        />

        {vods.data && vods.data.length > 0 && (
          <ul
            className="grid grid-cols-[repeat(auto-fill,minmax(240px,1fr))] gap-4 max-h-[calc(100vh-200px)] overflow-y-auto pr-1"
            aria-label={`Library — ${vods.data.length} VODs`}
          >
            {vods.data.map((v) => {
              const cw = continueWatching.data?.find(
                (e) => e.vodId === v.vod.twitchVideoId
              );
              return (
                <li key={v.vod.twitchVideoId}>
                  <VodCard
                    row={v}
                    download={downloadsById.get(v.vod.twitchVideoId) ?? null}
                    selected={selected === v.vod.twitchVideoId}
                    onSelect={() => setSelected(v.vod.twitchVideoId)}
                    onPlay={() =>
                      openPlayer({
                        vodId: v.vod.twitchVideoId,
                        autoplay: true,
                      })
                    }
                    {...(cw ? { watchedFraction: cw.watchedFraction } : {})}
                  />
                </li>
              );
            })}
          </ul>
        )}
      </section>

      <aside
        className="w-96 shrink-0 border-l border-[--color-border] pl-6"
        aria-label="VOD details"
      >
        <VodDetail
          vod={
            vods.data?.find((v) => v.vod.twitchVideoId === selected) ?? null
          }
          download={selected ? downloadsById.get(selected) ?? null : null}
        />
      </aside>
    </div>
  );
}

function VodDetail({
  vod,
  download,
}: {
  vod: VodWithChapters | null;
  download: DownloadRow | null;
}) {
  const enqueue = useEnqueueDownload();
  const retry = useRetryDownload();
  const openPlayer = useNavStore((s) => s.openPlayer);
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
          {vod.streamerDisplayName} · {formatUnixSeconds(v.streamStartedAt)}
        </p>
      </div>

      <div className="flex flex-wrap gap-2">
        {!download && (
          <Button
            variant="primary"
            onClick={() => enqueue.mutate(v.twitchVideoId)}
            disabled={enqueue.isPending}
          >
            Download
          </Button>
        )}
        {download?.state === "completed" && (
          <Button
            variant="primary"
            onClick={() =>
              openPlayer({ vodId: v.twitchVideoId, autoplay: true })
            }
          >
            Watch
          </Button>
        )}
        {(download?.state === "failed_retryable" ||
          download?.state === "failed_permanent") && (
          <Button
            variant="primary"
            onClick={() => retry.mutate(v.twitchVideoId)}
            disabled={retry.isPending}
          >
            Retry
          </Button>
        )}
        {download &&
          !["completed", "failed_retryable", "failed_permanent"].includes(
            download.state
          ) && (
            <Button variant="secondary" disabled>
              In queue
            </Button>
          )}
      </div>

      <CoStreamsSection vod={vod} />


      <dl className="grid grid-cols-[max-content_1fr] gap-x-4 gap-y-1 text-xs">
        <dt className="text-[--color-muted]">Status</dt>
        <dd>{v.ingestStatus}</dd>
        {v.statusReason && (
          <>
            <dt className="text-[--color-muted]">Reason</dt>
            <dd>{v.statusReason}</dd>
          </>
        )}
        <dt className="text-[--color-muted]">Duration</dt>
        <dd>{formatDurationSeconds(v.durationSeconds)}</dd>
        <dt className="text-[--color-muted]">Views</dt>
        <dd>{v.viewCount.toLocaleString()}</dd>
        <dt className="text-[--color-muted]">Sub-only</dt>
        <dd>{v.isSubOnly ? "yes" : "no"}</dd>
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
              key={`${c.positionMs}-${i}`}
              className="flex justify-between gap-4 border-b border-[--color-border]/50 pb-1"
            >
              <span>
                {c.gameName || "Unknown game"}{" "}
                {c.chapterType === "SYNTHETIC" && (
                  <span className="text-[--color-muted]">(single-game)</span>
                )}
              </span>
              <span className="font-mono text-[--color-muted]">
                {formatDurationSeconds(Math.floor(c.positionMs / 1000))}
              </span>
            </li>
          ))}
          {vod.chapters.length === 0 && (
            <li className="text-[--color-muted]">No chapters available.</li>
          )}
        </ol>
      </section>

      {vod.vod.isSubOnly && (
        <p className="text-xs text-[--color-muted] border-t border-[--color-border] pt-3">
          This VOD is restricted to the streamer's subscribers. Sightline
          won't retry downloading it, but it will re-check on every poll in
          case the streamer unlocks it.
        </p>
      )}
    </div>
  );
}

/**
 * Phase 5 cross-streamer deep link surface. Pulls the co-streams
 * from the backend and, for each, renders a "Watch this perspective
 * at HH:MM:SS" action. Clicking opens the other VOD's player seeked
 * to the shared wall-clock position (math lives in
 * `domain::deep_link::resolve_deep_link_target` server-side — we
 * mirror it here so the label shown to the user matches the actual
 * seek). If the other VOD isn't downloaded yet, the action becomes
 * "Download and watch" at elevated priority and the user sees a
 * placeholder player while the download finishes.
 */
function CoStreamsSection({ vod }: { vod: VodWithChapters }) {
  const openPlayer = useNavStore((s) => s.openPlayer);
  const openMultiView = useNavStore((s) => s.openMultiView);
  const [multiSelected, setMultiSelected] = useState<string | null>(null);
  const coStreams = useQuery<CoStream[]>({
    queryKey: ["co-streams", vod.vod.twitchVideoId],
    queryFn: () =>
      commands.getCoStreams({
        vodId: vod.vod.twitchVideoId,
      }),
    staleTime: 60_000,
  });
  const streamers = useStreamers();
  const downloads = useDownloads({});
  const downloadMap = useMemo(() => {
    const m = new Map<string, DownloadRow>();
    for (const row of downloads.data ?? []) m.set(row.vodId, row);
    return m;
  }, [downloads.data]);
  const streamerNameById = useMemo(() => {
    const m = new Map<string, string>();
    for (const row of streamers.data ?? []) {
      m.set(row.streamer.twitchUserId, row.streamer.displayName);
    }
    return m;
  }, [streamers.data]);

  // Reset the multi-view companion selection when the user picks a
  // different primary VOD — otherwise it would carry across drawer
  // navigations.
  useEffect(() => {
    setMultiSelected(null);
  }, [vod.vod.twitchVideoId]);

  if (!coStreams.data || coStreams.data.length === 0) return null;

  // Use the current VOD's midpoint as the "moment the user wants to
  // jump to" — a reasonable default for the detail drawer entry
  // point. The player's own cross-streamer action uses the live
  // currentTime; this one is a static shortcut.
  const selfMomentOffset = Math.floor(vod.vod.durationSeconds / 2);
  const selfMoment = vod.vod.streamStartedAt + selfMomentOffset;

  const handleOpenMultiView = () => {
    if (!multiSelected) return;
    openMultiView({ vodIds: [vod.vod.twitchVideoId, multiSelected] });
  };

  return (
    <section aria-labelledby="co-streams-heading" className="space-y-2">
      <div className="flex items-center justify-between gap-3">
        <h4 id="co-streams-heading" className="text-sm font-medium">
          Co-streams
        </h4>
        {multiSelected && (
          <Button
            variant="primary"
            onClick={handleOpenMultiView}
          >
            Open Multi-View
          </Button>
        )}
      </div>
      <ul className="space-y-2">
        {coStreams.data.map((cs) => {
          const intervalDuration = Math.max(0, cs.interval.endAt - cs.interval.startAt);
          const offset = Math.max(0, selfMoment - cs.interval.startAt);
          const clamped = Math.min(intervalDuration, offset);
          const label = formatDurationSeconds(Math.floor(clamped));
          const other = downloadMap.get(cs.interval.vodId);
          const downloaded = other?.state === "completed";
          const streamerName =
            streamerNameById.get(cs.interval.streamerId) ?? cs.interval.streamerId;
          const isMultiSelected = multiSelected === cs.interval.vodId;
          const handleClick = () => {
            if (!downloaded) {
              void commands.enqueueDownload({
                vodId: cs.interval.vodId,
                priority: 500, // higher than the default 100
              });
            }
            openPlayer({
              vodId: cs.interval.vodId,
              initialPositionSeconds: clamped,
              autoplay: downloaded,
            });
          };
          return (
            <li
              key={cs.interval.vodId}
              className={`text-xs flex items-center gap-3 border rounded p-2 ${
                isMultiSelected
                  ? "border-[--color-accent] bg-[--color-accent]/5"
                  : "border-[--color-border]/50"
              }`}
            >
              <input
                type="checkbox"
                aria-label={`Select ${streamerName} for multi-view`}
                checked={isMultiSelected}
                disabled={!downloaded}
                onChange={(e) =>
                  setMultiSelected(e.target.checked ? cs.interval.vodId : null)
                }
                className="accent-[--color-accent]"
              />
              <div className="flex-1 min-w-0">
                <p className="font-medium truncate">{streamerName}</p>
                <p className="text-[--color-muted]">
                  {formatUnixSeconds(cs.interval.startAt)} · overlap{" "}
                  {formatDurationSeconds(Math.floor(cs.overlapSeconds))}
                </p>
              </div>
              <Button variant="secondary" onClick={handleClick}>
                {downloaded ? `Jump to ${label}` : `Download & watch @ ${label}`}
              </Button>
            </li>
          );
        })}
      </ul>
      {!coStreams.data.some((cs) => downloadMap.get(cs.interval.vodId)?.state === "completed") && (
        <p className="text-[10px] text-[--color-muted] italic">
          Multi-view requires a downloaded co-stream. Use the action button to
          download one first.
        </p>
      )}
    </section>
  );
}
