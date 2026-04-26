import { useCallback, useEffect, useRef, useState } from "react";

import { assetUrl } from "@/lib/asset-url";
import { formatDurationSeconds, formatUnixSeconds } from "@/lib/format";
import type {
  DownloadRow,
  DownloadState,
  VodStatus,
  VodWithChapters,
} from "@/ipc";

import { useVodAssets } from "./use-vods";

/**
 * Tailwind class for the card's outer opacity, derived from the
 * lifecycle status (ADR-0033 §Visual states).  Pure mapping —
 * exposed so the LibraryPage tests can assert the differentiation
 * without rendering.
 */
export function statusOpacityClass(status: VodStatus): string {
  switch (status) {
    case "available":
      return "opacity-60";
    case "queued":
      return "opacity-70";
    case "downloading":
      return "opacity-80";
    case "ready":
      return "opacity-100";
    case "archived":
      return "opacity-100";
    case "deleted":
      return "opacity-50";
  }
}

/** Human label for the lifecycle status badge. */
export function statusLabel(status: VodStatus): string {
  switch (status) {
    case "available":
      return "Available";
    case "queued":
      return "Queued";
    case "downloading":
      return "Downloading";
    case "ready":
      return "Ready";
    case "archived":
      return "Watched";
    case "deleted":
      return "Removed";
  }
}

export interface VodCardProps {
  row: VodWithChapters;
  download: DownloadRow | null;
  selected: boolean;
  onSelect: () => void;
  /** Optional "Play" action — Phase 5 player. Disabled until completed. */
  onPlay?: () => void;
  /** Hover-revealed quick action: pull this VOD into the queue. */
  onDownload?: () => void;
  /** Hover-revealed quick action: cancel a queued / downloading row. */
  onCancel?: () => void;
  /** Hover-revealed quick action: re-pick a deleted row. */
  onRepick?: () => void;
  /** Hover-revealed quick action: remove a ready row from disk. */
  onRemove?: () => void;
  /** Watch-progress fraction in [0, 1]. Phase 5 feature; absent on fresh state. */
  watchedFraction?: number;
  /** True once crossing the completion threshold or manually marked. */
  watched?: boolean;
}

/**
 * Single card in the library grid. 16:9 thumbnail, hover-preview
 * shimmer across the six extracted frames, title + streamer + date
 * line, download-state badge, optional watch-progress bar (Phase 5),
 * and a primary Play / Download action.
 *
 * The hover-preview cycles through the six frames at 400ms per frame
 * while the pointer is over the card. `prefers-reduced-motion` users
 * see just the thumbnail — the shimmer is decorative.
 */
export function VodCard({
  row,
  download,
  selected,
  onSelect,
  onPlay,
  onDownload,
  onCancel,
  onRepick,
  onRemove,
  watchedFraction,
  watched,
}: VodCardProps) {
  const v = row.vod;
  const status = v.status;
  const assets = useVodAssets(v.twitchVideoId);
  const [hover, setHover] = useState(false);
  const [frameIndex, setFrameIndex] = useState(0);
  const reduceMotion = useRef(false);

  useEffect(() => {
    const m = window.matchMedia("(prefers-reduced-motion: reduce)");
    reduceMotion.current = m.matches;
    const onChange = (e: MediaQueryListEvent) => {
      reduceMotion.current = e.matches;
    };
    m.addEventListener("change", onChange);
    return () => m.removeEventListener("change", onChange);
  }, []);

  useEffect(() => {
    if (!hover || reduceMotion.current) {
      setFrameIndex(0);
      return;
    }
    const frames = assets.data?.previewFramePaths ?? [];
    if (frames.length === 0) return;
    const id = window.setInterval(() => {
      setFrameIndex((i) => (i + 1) % frames.length);
    }, 400);
    return () => window.clearInterval(id);
  }, [hover, assets.data?.previewFramePaths]);

  const previewFrames = assets.data?.previewFramePaths ?? [];
  const activeFrame = hover && previewFrames.length > 0 ? previewFrames[frameIndex] ?? null : null;
  const thumb = assets.data?.thumbnailPath ?? null;
  // Fallback to the remote Twitch thumbnail URL if we have no local
  // asset yet (pre-Phase-5 row that hasn't backfilled).
  const imageUrl = activeFrame
    ? assetUrl(activeFrame)
    : thumb
      ? assetUrl(thumb)
      : (v.thumbnailUrl ?? "");

  const handleMouseEnter = useCallback(() => setHover(true), []);
  const handleMouseLeave = useCallback(() => setHover(false), []);

  return (
    <article
      aria-label={v.title}
      aria-current={selected ? "true" : undefined}
      data-vod-status={status}
      className={`group relative rounded-[var(--radius-md)] overflow-hidden bg-[--color-surface] border border-[--color-border] transition-[border-color,transform] ${statusOpacityClass(
        status
      )} ${
        selected
          ? "border-[--color-accent]"
          : "hover:border-[--color-muted]"
      }`}
      onMouseEnter={handleMouseEnter}
      onMouseLeave={handleMouseLeave}
    >
      <button
        type="button"
        onClick={onSelect}
        aria-label={`Open details for ${v.title}`}
        className="block w-full text-left focus-visible:outline focus-visible:outline-2 focus-visible:outline-[--color-accent]"
      >
        <div className="aspect-video bg-[--color-bg] relative overflow-hidden">
          {imageUrl ? (
            <img
              src={imageUrl}
              alt=""
              aria-hidden="true"
              loading="lazy"
              className="absolute inset-0 h-full w-full object-cover"
            />
          ) : (
            <div
              aria-hidden="true"
              className="absolute inset-0 flex items-center justify-center text-[--color-muted] text-xs"
            >
              No thumbnail yet
            </div>
          )}

          {watched && (
            <span
              aria-label="Watched"
              title="Watched"
              className="absolute top-2 right-2 rounded-full bg-[--color-accent] text-[--color-accent-fg] text-[10px] font-bold h-5 w-5 flex items-center justify-center"
            >
              ✓
            </span>
          )}
          {typeof watchedFraction === "number" &&
            watchedFraction > 0.01 &&
            !watched && (
              <div
                role="progressbar"
                aria-label="Watch progress"
                aria-valuemin={0}
                aria-valuemax={100}
                aria-valuenow={Math.round(watchedFraction * 100)}
                className="absolute bottom-0 left-0 right-0 h-1 bg-black/30"
              >
                <div
                  className="h-full bg-[--color-accent]"
                  style={{ width: `${Math.min(100, watchedFraction * 100)}%` }}
                />
              </div>
            )}

          <span className="absolute bottom-1 right-2 rounded bg-black/70 text-white text-[10px] font-mono px-1.5 py-0.5">
            {formatDurationSeconds(v.durationSeconds)}
          </span>
          <span className="absolute top-1 left-2 flex flex-col gap-1">
            <StatusBadge status={status} />
            {download && status !== "ready" && status !== "archived" && (
              <DownloadBadge state={download.state} />
            )}
          </span>
        </div>

        <div className="p-3 space-y-1">
          <h3 className="text-sm font-medium truncate" title={v.title}>
            {v.title}
          </h3>
          <p className="text-xs text-[--color-muted] truncate">
            {row.streamerDisplayName} · {formatUnixSeconds(v.streamStartedAt)}
          </p>
        </div>
      </button>

      {/* Hover-reveal quick actions (ADR-0033 §Per-VOD quick actions).
          The overlay is keyboard-reachable: each button is focusable
          and labelled, so a Tab user can act on a card without
          relying on hover.  The opacity-0/group-hover:opacity-100
          transition is purely cosmetic. */}
      <div
        className="pointer-events-none absolute inset-0 flex items-end justify-end p-3 gap-2 opacity-0 group-hover:opacity-100 focus-within:opacity-100 transition-opacity duration-[var(--motion-fast)]"
      >
        {status === "available" && onDownload && (
          <button
            type="button"
            onClick={(e) => {
              e.stopPropagation();
              onDownload();
            }}
            aria-label={`Download ${v.title}`}
            className="pointer-events-auto bg-[--color-accent] text-[--color-accent-fg] text-xs px-3 py-1 rounded focus-visible:outline focus-visible:outline-2 focus-visible:outline-[--color-accent]"
          >
            ↓ Download
          </button>
        )}
        {status === "queued" && onCancel && (
          <button
            type="button"
            onClick={(e) => {
              e.stopPropagation();
              onCancel();
            }}
            aria-label={`Cancel queued ${v.title}`}
            className="pointer-events-auto bg-[--color-surface] border border-[--color-border] text-[--color-fg] text-xs px-3 py-1 rounded focus-visible:outline focus-visible:outline-2 focus-visible:outline-[--color-accent]"
          >
            ✕ Cancel
          </button>
        )}
        {(status === "ready" || status === "archived") && onPlay && (
          <button
            type="button"
            onClick={(e) => {
              e.stopPropagation();
              onPlay();
            }}
            aria-label={
              status === "archived"
                ? `Re-watch ${v.title}`
                : `Play ${v.title}`
            }
            className="pointer-events-auto bg-[--color-accent] text-[--color-accent-fg] text-xs px-3 py-1 rounded focus-visible:outline focus-visible:outline-2 focus-visible:outline-[--color-accent]"
          >
            {status === "archived" ? "▶ Re-watch" : "▶ Play"}
          </button>
        )}
        {status === "ready" && onRemove && (
          <button
            type="button"
            onClick={(e) => {
              e.stopPropagation();
              onRemove();
            }}
            aria-label={`Remove ${v.title} from disk`}
            className="pointer-events-auto bg-[--color-surface] border border-[--color-border] text-[--color-fg] text-xs px-3 py-1 rounded focus-visible:outline focus-visible:outline-2 focus-visible:outline-[--color-accent]"
          >
            × Remove
          </button>
        )}
        {status === "deleted" && onRepick && (
          <button
            type="button"
            onClick={(e) => {
              e.stopPropagation();
              onRepick();
            }}
            aria-label={`Pick again ${v.title}`}
            className="pointer-events-auto bg-[--color-accent] text-[--color-accent-fg] text-xs px-3 py-1 rounded focus-visible:outline focus-visible:outline-2 focus-visible:outline-[--color-accent]"
          >
            ↻ Pick again
          </button>
        )}
      </div>
    </article>
  );
}

/** Status-tier badge for the lifecycle column (ADR-0033). */
function StatusBadge({ status }: { status: VodStatus }) {
  const palette: Record<VodStatus, string> = {
    available:
      "bg-[--color-surface] text-[--color-muted] border border-[--color-border]",
    queued: "bg-blue-500/30 text-blue-200 border border-blue-500/50",
    downloading: "bg-blue-500/80 text-white",
    ready: "bg-emerald-500/80 text-white",
    archived: "bg-emerald-500/30 text-emerald-200 border border-emerald-500/50",
    deleted: "bg-zinc-500/30 text-zinc-300 border border-zinc-500/50",
  };
  return (
    <span
      className={`text-[10px] uppercase tracking-wider px-1.5 py-0.5 rounded ${palette[status]}`}
    >
      {statusLabel(status)}
    </span>
  );
}

function DownloadBadge({ state }: { state: DownloadState }) {
  const label: Record<DownloadState, string> = {
    queued: "Queued",
    downloading: "Downloading",
    paused: "Paused",
    completed: "Downloaded",
    failed_retryable: "Retrying",
    failed_permanent: "Failed",
  };
  const palette: Record<DownloadState, string> = {
    queued: "bg-[--color-surface] text-[--color-muted] border border-[--color-border]",
    downloading: "bg-blue-500/80 text-white",
    paused: "bg-amber-500/80 text-white",
    completed: "bg-emerald-500/80 text-white",
    failed_retryable: "bg-orange-500/80 text-white",
    failed_permanent: "bg-red-500/80 text-white",
  };
  return (
    <span
      className={`text-[10px] uppercase tracking-wider px-1.5 py-0.5 rounded ${palette[state]}`}
    >
      {label[state]}
    </span>
  );
}
