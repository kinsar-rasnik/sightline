import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect, useMemo, useState } from "react";
import { listen } from "@tauri-apps/api/event";

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
  events,
  type CoStream,
  type DownloadRow,
  type VodStatus,
  type VodWithChapters,
} from "@/ipc";
import { formatDurationSeconds, formatUnixSeconds } from "@/lib/format";
import { useNavStore } from "@/stores/nav-store";

/**
 * Library lifecycle filter (ADR-0033 §Filter chips).  Decoupled
 * from the legacy ingest-status filter so the new view-by-status
 * chips can coexist if the user wants to drill in via both axes.
 */
type LifecycleFilter = "all" | "not_downloaded" | "downloaded" | "watched";

const LIFECYCLE_OPTIONS: Array<{ key: LifecycleFilter; label: string }> = [
  { key: "all", label: "All" },
  { key: "not_downloaded", label: "Not downloaded" },
  { key: "downloaded", label: "Downloaded" },
  { key: "watched", label: "Watched" },
];

/** True iff a VOD's lifecycle state matches the chosen filter. */
function matchesLifecycle(status: VodStatus, filter: LifecycleFilter): boolean {
  switch (filter) {
    case "all":
      return true;
    case "not_downloaded":
      return status === "available" || status === "deleted";
    case "downloaded":
      return (
        status === "queued" || status === "downloading" || status === "ready"
      );
    case "watched":
      return status === "archived";
  }
}

export function LibraryPage() {
  const [lifecycle, setLifecycle] = useState<LifecycleFilter>("all");
  const [streamerId, setStreamerId] = useState<string | null>(null);
  const [selected, setSelected] = useState<string | null>(null);
  const [actionError, setActionError] = useState<string | null>(null);
  const streamers = useStreamers();
  const qc = useQueryClient();
  const enqueue = useEnqueueDownload();

  const input = useMemo(
    () => ({
      filters: {
        ...(streamerId && { streamerIds: [streamerId] }),
      },
      sort: "stream_started_at_desc" as const,
      limit: 200,
      offset: 0,
    }),
    [streamerId]
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

  const filteredVods = useMemo(() => {
    if (!vods.data) return [];
    if (lifecycle === "all") return vods.data;
    return vods.data.filter((v) => matchesLifecycle(v.vod.status, lifecycle));
  }, [vods.data, lifecycle]);

  // ADR-0033 §Event-driven UI updates: subscribe to distribution
  // events and bust the vods cache so status badges + filter chips
  // reflect reality immediately rather than after the next refetch.
  // R-RC-02 fix: track resolved unlisteners in a ref so cleanup is
  // synchronous and the unmounted component cannot receive a late
  // event tick into invalidateQueries.
  useEffect(() => {
    let cancelled = false;
    const unlisteners: Array<() => void> = [];
    const subscribe = async (
      topic: string,
      handler: () => void
    ): Promise<void> => {
      const unsub = await listen(topic, handler);
      if (cancelled) {
        unsub();
        return;
      }
      unlisteners.push(unsub);
    };
    const invalidate = () =>
      qc.invalidateQueries({ queryKey: ["vods"] });
    void Promise.all([
      subscribe(events.distributionVodPicked, invalidate),
      subscribe(events.distributionVodArchived, invalidate),
      subscribe(events.distributionWindowEnforced, invalidate),
      subscribe(events.downloadCompleted, invalidate),
    ]);
    return () => {
      cancelled = true;
      unlisteners.forEach((u) => u());
    };
  }, [qc]);

  const handleDownload = async (vodId: string) => {
    setActionError(null);
    try {
      await commands.pickVod({ vodId });
    } catch {
      // Pull-mode pick failed — fall back to the legacy enqueue
      // path so a user in auto-mode (or with a stale vods.status)
      // still gets the download started. Await mutateAsync so the
      // subsequent invalidateQueries observes the new row.
      try {
        await enqueue.mutateAsync(vodId);
      } catch (e) {
        const detail = e instanceof Error ? e.message : "unknown error";
        setActionError(`Could not start download: ${detail}`);
        return;
      }
    }
    void qc.invalidateQueries({ queryKey: ["vods"] });
  };

  const handleCancel = async (vodId: string) => {
    setActionError(null);
    try {
      await commands.unpickVod({ vodId });
    } catch {
      // unpick is only valid from queued; if the row is already
      // downloading the state machine rejects the transition.
      // Surface a hint so the user knows where to go next instead
      // of seeing a Cancel button that appears to do nothing.
      setActionError(
        "This download has already started. Cancel it from the Downloads page."
      );
      return;
    }
    void qc.invalidateQueries({ queryKey: ["vods"] });
  };

  const handleRemove = async (vodId: string) => {
    setActionError(null);
    try {
      await commands.removeVod({ vodId });
    } catch (e) {
      const detail = e instanceof Error ? e.message : "unknown error";
      setActionError(`Could not remove file: ${detail}`);
      return;
    }
    void qc.invalidateQueries({ queryKey: ["vods"] });
  };

  const emptyStateCopy = ((): string => {
    if (vods.isLoading) return "";
    if (!vods.data) return "";
    // The "all" branch is handled by this top-level check —
    // filteredVods === vods.data when lifecycle === "all", so the
    // per-filter switch below never runs for the "all" case.
    if (vods.data.length === 0) {
      return "No VODs match these filters yet. Add streamers and let the poller run, or trigger a poll from the Streamers page.";
    }
    if (filteredVods.length === 0) {
      switch (lifecycle) {
        case "not_downloaded":
          return "All caught up — every VOD is already downloaded or watched.";
        case "downloaded":
          return "Nothing downloaded yet. Pick a VOD from the Not downloaded filter.";
        case "watched":
          return "No watched VODs yet. Watching one to completion will move it here.";
        case "all":
          // Unreachable: vods.data.length === 0 already returned above.
          return "";
      }
    }
    return "";
  })();

  return (
    <div className="flex gap-6 h-full">
      <section
        className="flex-1 min-w-0 space-y-4"
        aria-label="Library grid"
      >
        <div
          className="flex flex-wrap gap-2"
          role="toolbar"
          aria-label="VOD lifecycle filter"
        >
          {LIFECYCLE_OPTIONS.map((opt) => (
            <button
              key={opt.key}
              type="button"
              aria-pressed={lifecycle === opt.key}
              onClick={() => setLifecycle(opt.key)}
              className={`text-xs px-3 py-1.5 rounded-full border transition-colors ${
                lifecycle === opt.key
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
        {actionError && (
          <div
            role="alert"
            className="rounded border border-amber-500/30 bg-amber-500/10 px-3 py-2 text-xs text-amber-200 flex items-center justify-between gap-3"
          >
            <span>{actionError}</span>
            <button
              type="button"
              aria-label="Dismiss"
              onClick={() => setActionError(null)}
              className="text-amber-300 hover:text-amber-100"
            >
              ×
            </button>
          </div>
        )}
        {emptyStateCopy && (
          <p className="text-sm text-[--color-muted]">{emptyStateCopy}</p>
        )}

        <ContinueWatchingRow
          entries={continueWatching.data ?? []}
          onPlay={(entry) =>
            openPlayer({ vodId: entry.vodId, autoplay: true })
          }
        />

        {filteredVods.length > 0 && (
          <ul
            className="grid grid-cols-[repeat(auto-fill,minmax(240px,1fr))] gap-4 max-h-[calc(100vh-200px)] overflow-y-auto pr-1"
            aria-label={`Library — ${filteredVods.length} VODs`}
          >
            {filteredVods.map((v) => {
              const cw = continueWatching.data?.find(
                (e) => e.vodId === v.vod.twitchVideoId
              );
              const vodId = v.vod.twitchVideoId;
              return (
                <li key={vodId}>
                  <VodCard
                    row={v}
                    download={downloadsById.get(vodId) ?? null}
                    selected={selected === vodId}
                    onSelect={() => setSelected(vodId)}
                    onPlay={() =>
                      openPlayer({
                        vodId,
                        autoplay: true,
                      })
                    }
                    onDownload={() => void handleDownload(vodId)}
                    onCancel={() => void handleCancel(vodId)}
                    onRepick={() => void handleDownload(vodId)}
                    onRemove={() => void handleRemove(vodId)}
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
