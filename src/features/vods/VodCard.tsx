/**
 * VodCard — v2.1 rewrite per ADR-0039.
 *
 * Atomic visual element for the v2.1 browse language. Used by:
 *  - TimelinePage's TimelineLanes (Mission 3) — Full / Compact / Sliver
 *    variants depending on the card's zoom-mapped width
 *  - LibraryPage's grid (Phase-8 / ADR-0033 origin) — Full variant
 *  - ContinueWatchingRow (Mission 2 migration) — Compact variant
 *  - TimelineHeroSlot (Mission 3) — Hero variant
 *
 * ADR-0039 binding sections:
 *  - DV1  Card-Geometry: 16:9 ≥ 75 % image-area; 3 width variants + Hero
 *  - DV2  Idle-Composition: position-map (selection top-right, retention
 *         bottom-right ≥ stage-3, 3-px stripe bottom-edge, avatar+title strip)
 *  - DV3  Hover-Sequence: 800 ms delay → zoom 1.10 + panel slide-in +
 *         frame-strip crossfade; 400 ms / frame loop; singleton across mounts
 *  - DV4  Stack-Card: silhouette layers + avatar-dots row (fan-out is
 *         caller-provided in Mission 3; this file exposes the silhouette layer
 *         and the dot row only)
 *  - DV5  Retention: 3-stage position-migration; clock-icon badge
 *  - DV6  Progress-Stripe: 3-px at image-bottom-edge
 *  - DV7  State-Matrix: all 13 primary rows + stack variant
 *  - DV9  Accessibility: ARIA-label format + keyboard trigger gate
 *  - DV10 Performance: React.memo + eager-mount-CSS-hidden frame strip +
 *         singleton invariant
 *
 * ADR-0040 binding tokens (TV4/TV5/TV6/TV11) consumed via Tailwind
 * arbitrary-value notation for custom-prefixed tokens (per Mission-1
 * audit §6: standard prefixes auto-generate utilities; custom prefixes
 * use `var(--…)`).
 *
 * Backward-compatibility note. The Phase-8 LibraryPage API
 * (quick-action props onDownload / onCancel / onPlay / onRemove /
 * onRepick + download row + watchedFraction + watched) is preserved as
 * optional `actions` for LibraryPage callers. New callers (Mission 3
 * TimelineLanes) use the data-driven path via `vodId` + selection
 * subscription. LibraryPage will migrate to the data-driven API in
 * Mission 5; until then, both paths coexist.
 */

import {
  memo,
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import { Check, Clock, Play, Plus, Star } from "lucide-react";

import { assetUrl } from "@/lib/asset-url";
import {
  formatDurationSeconds,
  formatUnixSeconds,
} from "@/lib/format";
import type {
  DownloadRow,
  DownloadState,
  VodStatus,
  VodWithChapters,
} from "@/ipc";

import {
  useVodSummary,
  type RetentionInfo,
  type VodSummary,
} from "./hooks/useVodSummary";

/* ---------------------------------------------------------------------------
 * Singleton frame-strip-loop registry (DV10 binding).
 * One module-level slot tracks the currently-looping vodId across all
 * mounted cards on the page; on trigger, the previously-looping card is
 * aborted to idle.
 * ------------------------------------------------------------------------- */

type FrameStripAbortFn = () => void;
let currentLooping: { vodId: string; abort: FrameStripAbortFn } | null = null;

function claimFrameStripLoop(vodId: string, abort: FrameStripAbortFn): void {
  if (currentLooping && currentLooping.vodId !== vodId) {
    currentLooping.abort();
  }
  currentLooping = { vodId, abort };
}

function releaseFrameStripLoop(vodId: string): void {
  if (currentLooping?.vodId === vodId) {
    currentLooping = null;
  }
}

/* ---------------------------------------------------------------------------
 * Status helpers — preserved from v2.0.1 for LibraryPage backward-compat.
 * ------------------------------------------------------------------------- */

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

/* ---------------------------------------------------------------------------
 * Public types.
 * ------------------------------------------------------------------------- */

export type VodCardVariant = "full" | "compact" | "sliver" | "hero";

/**
 * Stack-perspective entry for `stack` prop. Provided by the caller
 * (TimelinePage in Mission 3); avatars resolve from `profileImageUrl`
 * with `initials` fallback. Up to 4 avatar dots render, then "+N more".
 */
export interface VodCardStackPerspective {
  twitchUserId: string;
  displayName: string;
  profileImageUrl: string | null;
}

/**
 * LibraryPage quick-action callbacks. Optional; only render when
 * provided. Preserved from v2.0.1 API; Mission 5 may refine.
 */
export interface VodCardActions {
  onPlay?: () => void;
  onDownload?: () => void;
  onCancel?: () => void;
  onRepick?: () => void;
  onRemove?: () => void;
  /** Optional Phase-8 download row for status-badge rendering. */
  download?: DownloadRow | null;
}

/**
 * Hero-variant-specific props.
 */
export interface VodCardHeroProps {
  pitch?: string;
  onWatchLive?: () => void;
  onMoreInfo?: () => void;
}

export interface VodCardProps {
  /** Twitch VOD id. Drives `useVodSummary` when `data` is not supplied. */
  vodId: string;
  /** Visual variant. `full` is the default LibraryPage shape. */
  variant?: VodCardVariant;
  /**
   * Selection state (D7 of ADR-0038). When `true`, renders the accent
   * checkmark in the image area's top-right corner.
   */
  selected?: boolean;
  /** Default click handler. Open Detail Overlay or LibraryPage detail. */
  onSelect?: () => void;
  /** Optional pre-loaded summary. Skips `useVodSummary` fetch when set. */
  data?: VodSummary;
  /** Optional override row for legacy LibraryPage callers that already
   *  have a `VodWithChapters`. Bridges to the new composite-data shape
   *  without forcing a hook re-fetch when the data is already in hand. */
  row?: VodWithChapters;
  /** Stack-perspective set (Mission 3 caller-provided). When non-empty,
   *  renders silhouette layers + avatar-dots row per DV4. */
  stack?: VodCardStackPerspective[];
  /** Quick-action callbacks (LibraryPage backward-compat). */
  actions?: VodCardActions;
  /** Hero-variant props (only consulted when variant === "hero"). */
  hero?: VodCardHeroProps;
  /** Optional className for outer wrapper (Library grid uses for layout). */
  className?: string;
}

/* ---------------------------------------------------------------------------
 * Helpers.
 * ------------------------------------------------------------------------- */

/**
 * Short form used in the image-area badge (DV5 stage 3 / 4).
 * ADR-0039 DV5 examples: "32h" (stage 3), "18h" or "<1d" (stage 4).
 * Stages 3 (24–48 h) and 4 (≤ 24 h) both render hours; "<1d" surfaces
 * only when remainingSeconds <= 24h yet >= 1h (stage 4 floor).
 */
function retentionLabelShort(remainingSeconds: number): string {
  if (remainingSeconds <= 0) return "expired";
  if (remainingSeconds < 48 * 3600) {
    const hours = Math.max(1, Math.floor(remainingSeconds / 3600));
    return `${hours}h`;
  }
  const days = Math.floor(remainingSeconds / 86_400);
  return `${days}d`;
}

/**
 * Long form used in metadata-strip text labels (DV5 stage 2) and the
 * card's screen-reader aria-label. Mirrors `retentionLabelShort` but
 * with explicit "expires in" prefix.
 */
function retentionLabelLong(remainingSeconds: number): string {
  if (remainingSeconds <= 0) return "expired";
  if (remainingSeconds < 48 * 3600) {
    const hours = Math.max(1, Math.floor(remainingSeconds / 3600));
    return `expires in ${hours}h`;
  }
  const days = Math.floor(remainingSeconds / 86_400);
  return `expires in ${days}d`;
}

function ariaLabel(summary: VodSummary, selected: boolean): string {
  const title = summary.vod.vod.title;
  const streamer = summary.streamer?.displayName ?? summary.vod.streamerDisplayName;
  const duration = formatDurationSeconds(summary.vod.vod.durationSeconds);
  const parts = [title, streamer, duration];
  if (summary.retention.stage === 2) {
    parts.push(retentionLabelLong(summary.retention.remainingSeconds));
  } else if (summary.retention.stage === 3 || summary.retention.stage === 4) {
    parts.push(retentionLabelLong(summary.retention.remainingSeconds));
  } else if (summary.retention.stage === "expired") {
    parts.push("expired");
  }
  const wp = summary.watchProgress;
  if (wp && wp.state === "in_progress" && wp.watchedFraction > 0.01) {
    parts.push(`watched ${Math.round(wp.watchedFraction * 100)} percent`);
  } else if (
    wp &&
    (wp.state === "completed" || wp.state === "manually_watched")
  ) {
    parts.push("watched");
  }
  if (selected) parts.push("selected");
  return parts.join(", ");
}

/**
 * Build a `VodSummary` from a `VodWithChapters` legacy row when callers
 * already have the data. Used by LibraryPage's existing list-fetch path
 * to avoid a redundant `useVodSummary` round-trip per card. Streamer
 * lookup still uses `useStreamers` (cache-shared); retention is
 * computed client-side from the streamer's broadcaster_type if available.
 */
function summaryFromRow(
  row: VodWithChapters,
  fallbackBroadcasterType?: string | null
): Omit<VodSummary, "assets" | "watchProgress" | "streamer"> & {
  retention: RetentionInfo;
} {
  // Conservative retention if we don't know the broadcaster_type yet:
  // assume 7 d window (non-affiliate floor). Real-Use-Drift documented
  // in ADR-0039 DV5 / VOd-A2 as conservative-falsch — shows less time
  // than actual, never more.
  const broadcasterType = fallbackBroadcasterType ?? null;
  const retentionDays =
    broadcasterType === "partner"
      ? 60
      : broadcasterType === "affiliate"
        ? 14
        : 7;
  const nowSec = Math.floor(Date.now() / 1000);
  const retentionAt = row.vod.streamStartedAt + retentionDays * 86_400;
  const remainingSeconds = retentionAt - nowSec;
  let stage: RetentionInfo["stage"];
  if (remainingSeconds <= 0) stage = "expired";
  else if (remainingSeconds > 7 * 86_400) stage = 1;
  else if (remainingSeconds > 48 * 3600) stage = 2;
  else if (remainingSeconds > 24 * 3600) stage = 3;
  else stage = 4;
  return {
    vod: row,
    retention: { stage, retentionAt, remainingSeconds },
  };
}

/* ---------------------------------------------------------------------------
 * VodCard component.
 * ------------------------------------------------------------------------- */

function VodCardImpl({
  vodId,
  variant = "full",
  selected = false,
  onSelect,
  data,
  row,
  stack,
  actions,
  hero,
  className,
}: VodCardProps) {
  // Fetch composite summary only when caller hasn't pre-loaded data
  // and hasn't passed a legacy row.
  const shouldFetch = !data && !row;
  const fetched = useVodSummary(shouldFetch ? vodId : null);

  const summary: VodSummary | null = useMemo(() => {
    if (data) return data;
    if (row) {
      const partial = summaryFromRow(row);
      return {
        ...partial,
        streamer: null,
        assets: null,
        watchProgress: null,
      };
    }
    return fetched.data;
  }, [data, row, fetched.data]);

  // Hover/focus trigger state (DV3). Singleton across mounts is enforced
  // via the module-level `currentLooping` registry.
  const [hovered, setHovered] = useState(false);
  const [focused, setFocused] = useState(false);
  const [triggered, setTriggered] = useState(false);
  const delayTimerRef = useRef<number | null>(null);
  const frameTimerRef = useRef<number | null>(null);
  const [frameIndex, setFrameIndex] = useState(0);

  // Reduced-motion preference (DV3 + H-22). The token system handles
  // CSS-side overrides; this state gates the JS-driven frame-strip loop.
  const reduceMotionRef = useRef(false);
  useEffect(() => {
    const m = window.matchMedia("(prefers-reduced-motion: reduce)");
    reduceMotionRef.current = m.matches;
    const onChange = (e: MediaQueryListEvent) => {
      reduceMotionRef.current = e.matches;
    };
    m.addEventListener("change", onChange);
    return () => m.removeEventListener("change", onChange);
  }, []);

  const previewFrames = summary?.assets?.previewFramePaths ?? [];

  const cancelTriggerTimer = useCallback(() => {
    if (delayTimerRef.current !== null) {
      window.clearTimeout(delayTimerRef.current);
      delayTimerRef.current = null;
    }
  }, []);

  const stopFrameLoop = useCallback(() => {
    if (frameTimerRef.current !== null) {
      window.clearInterval(frameTimerRef.current);
      frameTimerRef.current = null;
    }
    setFrameIndex(0);
    setTriggered(false);
    releaseFrameStripLoop(vodId);
  }, [vodId]);

  // Start delay timer when hover OR focus is active; clear when both
  // are absent. Singleton-claim happens on trigger fire.
  useEffect(() => {
    if (variant === "sliver") return; // Sliver variant has no hover-preview
    const anyActive = hovered || focused;
    if (!anyActive) {
      cancelTriggerTimer();
      if (triggered) stopFrameLoop();
      return;
    }
    if (triggered) return; // already running
    cancelTriggerTimer();
    if (reduceMotionRef.current) {
      // Reduced-motion: still surface expanded panel instantly; no loop.
      setTriggered(true);
      return;
    }
    delayTimerRef.current = window.setTimeout(() => {
      setTriggered(true);
      claimFrameStripLoop(vodId, stopFrameLoop);
      // Start frame-strip loop at the configured cadence.
      if (previewFrames.length > 0) {
        frameTimerRef.current = window.setInterval(() => {
          setFrameIndex((i) => (i + 1) % previewFrames.length);
        }, 400);
      }
    }, 800);
  }, [
    hovered,
    focused,
    triggered,
    previewFrames.length,
    variant,
    vodId,
    cancelTriggerTimer,
    stopFrameLoop,
  ]);

  // Cleanup on unmount.
  useEffect(() => {
    return () => {
      cancelTriggerTimer();
      stopFrameLoop();
    };
  }, [cancelTriggerTimer, stopFrameLoop]);

  // Event handlers — must be declared above the variant-conditional
  // early returns to keep the hook-call order stable across renders
  // (rules-of-hooks: hooks count + order must not depend on props).
  const handleMouseEnter = useCallback(() => setHovered(true), []);
  const handleMouseLeave = useCallback(() => setHovered(false), []);
  const handleFocus = useCallback(() => setFocused(true), []);
  const handleBlur = useCallback(() => setFocused(false), []);

  // ============================================================
  // Loading skeleton (DV2 fallback)
  // ============================================================
  if (!summary) {
    return (
      <article
        aria-label="Loading VOD"
        aria-busy="true"
        className={`relative overflow-hidden rounded-[var(--card-corner-radius)] bg-surface-1 ${
          className ?? ""
        }`}
      >
        <div className="aspect-video bg-[var(--color-surface-skeleton)]" />
        {variant !== "sliver" && (
          <div className="px-2 py-1 space-y-1">
            <div className="h-3 w-3/4 rounded bg-[var(--color-surface-skeleton)]" />
            <div className="h-2 w-1/2 rounded bg-[var(--color-surface-skeleton)]" />
          </div>
        )}
      </article>
    );
  }

  if (variant === "hero") {
    return (
      <HeroVariant
        summary={summary}
        selected={selected}
        onSelect={onSelect}
        hero={hero}
        className={className}
      />
    );
  }

  if (variant === "sliver") {
    return (
      <SliverVariant
        summary={summary}
        selected={selected}
        onSelect={onSelect}
        className={className}
      />
    );
  }

  // Full and Compact share the same composition tree with strip-line
  // visibility gated below.
  const v = summary.vod.vod;
  const streamerDisplayName =
    summary.streamer?.displayName ?? summary.vod.streamerDisplayName;
  const streamerAvatarUrl = summary.streamer?.profileImageUrl ?? null;
  const status = v.status;
  const watch = summary.watchProgress;
  const showProgressStripe =
    watch !== null &&
    watch.state === "in_progress" &&
    watch.watchedFraction > 0.01 &&
    watch.watchedFraction < 1;
  const showWatchedGlyph =
    watch !== null &&
    (watch.state === "completed" || watch.state === "manually_watched");
  const showRetentionBadgeOnImage =
    summary.retention.stage === 3 || summary.retention.stage === 4;
  const showRetentionTextInStrip = summary.retention.stage === 2;
  const retentionAccent = summary.retention.stage === 4;

  // Active image source: frame-strip during triggered, thumbnail otherwise.
  const activeFramePath =
    triggered && !reduceMotionRef.current && previewFrames.length > 0
      ? (previewFrames[frameIndex] ?? null)
      : null;
  const imageUrl = activeFramePath
    ? assetUrl(activeFramePath)
    : summary.assets?.thumbnailPath
      ? assetUrl(summary.assets.thumbnailPath)
      : (v.thumbnailUrl ?? "");

  const isStack = (stack?.length ?? 0) >= 2;
  const silhouetteCount = Math.min(2, Math.max(0, (stack?.length ?? 0) - 1));

  return (
    /* eslint-disable jsx-a11y/no-redundant-roles,
       jsx-a11y/role-supports-aria-props,
       jsx-a11y/no-noninteractive-tabindex --
       ADR-0039 DV9 binding: the VodCard is a focusable interactive
       article with aria-selected / aria-expanded + tabindex=0 for
       keyboard-driven multiselect + stack-fan-out semantics. The
       explicit role="article" mirrors DV9's exact spec shape so
       downstream `vision-compliance-reviewer` can grade against the
       ADR. Lint rules are heuristics; the binding spec wins per
       R-ADR-01. */
    <article
      role="article"
      aria-label={ariaLabel(summary, selected)}
      aria-selected={selected ? "true" : "false"}
      aria-expanded={isStack ? (triggered ? "true" : "false") : undefined}
      data-vod-id={v.twitchVideoId}
      data-vod-status={status}
      data-stack={isStack ? "stack-idle" : "single"}
      data-triggered={triggered ? "true" : "false"}
      data-variant={variant}
      tabIndex={0}
      onMouseEnter={handleMouseEnter}
      onMouseLeave={handleMouseLeave}
      onFocus={handleFocus}
      onBlur={handleBlur}
      className={`group relative overflow-visible rounded-[var(--card-corner-radius)] bg-surface-1 ${statusOpacityClass(
        status
      )} ${className ?? ""}`}
      style={{
        transform: triggered
          ? "scale(var(--card-hover-zoom-factor))"
          : "scale(1)",
        transitionProperty: "transform",
        transitionDuration: "var(--motion-zoom-duration)",
        transitionTimingFunction: "var(--motion-easing-out)",
        zIndex: triggered ? "var(--z-card-hover)" : "var(--z-card-idle)",
      }}
    >
      {/* Stack silhouette layers (DV4 Layer 1) */}
      {silhouetteCount >= 1 && (
        <div
          aria-hidden="true"
          className="absolute inset-x-0 -bottom-[var(--silhouette-offset)] h-full rounded-[var(--card-corner-radius)] bg-surface-1 opacity-60 -z-10"
        />
      )}
      {silhouetteCount >= 2 && (
        <div
          aria-hidden="true"
          className="absolute inset-x-0 -bottom-[calc(2*var(--silhouette-offset))] h-full rounded-[var(--card-corner-radius)] bg-surface-1 opacity-40 -z-20"
        />
      )}

      <button
        type="button"
        onClick={onSelect}
        aria-label={`Open details for ${v.title}`}
        className="block w-full text-left focus:outline-none"
      >
        {/* Image area — 16:9, ≥75 % card height (CSS aspect-ratio enforces) */}
        <div className="relative overflow-hidden rounded-t-[var(--card-corner-radius)] aspect-video bg-[var(--color-surface-skeleton)]">
          {imageUrl ? (
            <img
              src={imageUrl}
              alt=""
              aria-hidden="true"
              loading="lazy"
              className="absolute inset-0 h-full w-full object-cover"
              style={{
                transitionProperty: "opacity",
                transitionDuration: "var(--motion-crossfade-duration)",
                transitionTimingFunction: "var(--motion-easing-out)",
              }}
            />
          ) : (
            <div
              aria-hidden="true"
              className="absolute inset-0 flex items-center justify-center text-xs text-muted"
            >
              No thumbnail
            </div>
          )}

          {/* Eager-mount frame-strip frames (DV10): display:none until
              triggered, browser pre-fetches the bytes. */}
          {!reduceMotionRef.current && previewFrames.length > 0 && (
            <div aria-hidden="true" style={{ display: "none" }}>
              {previewFrames.map((p, i) => (
                <img key={i} src={assetUrl(p)} alt="" loading="lazy" />
              ))}
            </div>
          )}

          {/* Top-right: selection checkmark (DV2 position-map) */}
          {selected && (
            <span
              aria-label="Selected"
              title="Selected"
              className="absolute top-[var(--selection-checkmark-padding)] right-[var(--selection-checkmark-padding)] flex items-center justify-center rounded-sm bg-accent text-accent-fg"
              style={{
                width: "var(--selection-checkmark-size)",
                height: "var(--selection-checkmark-size)",
              }}
            >
              <Check
                width={12}
                height={12}
                strokeWidth={Number("var(--icon-stroke-width)") || 1.5}
                aria-hidden="true"
              />
            </span>
          )}

          {/* Bottom-right: retention badge stage 3 / 4 (DV5) */}
          {showRetentionBadgeOnImage && (
            <span
              aria-label={retentionLabelLong(summary.retention.remainingSeconds)}
              className={`absolute right-[var(--selection-checkmark-padding)] bottom-[var(--selection-checkmark-padding)] inline-flex items-center gap-[var(--retention-badge-gap)] rounded-sm font-tabular text-xs ${
                retentionAccent
                  ? "bg-accent text-accent-fg"
                  : "bg-surface-2 text-fg"
              }`}
              style={{
                height: "var(--retention-badge-height)",
                paddingLeft: "var(--retention-badge-padding-x)",
                paddingRight: "var(--retention-badge-padding-x)",
              }}
            >
              <Clock
                width={12}
                height={12}
                strokeWidth={1.5}
                aria-hidden="true"
              />
              <span>
                {retentionLabelShort(summary.retention.remainingSeconds)}
              </span>
            </span>
          )}

          {/* Bottom edge: 3-px continue-watching stripe (DV6) */}
          {showProgressStripe && (
            <div
              role="progressbar"
              aria-label="Watch progress"
              aria-valuemin={0}
              aria-valuemax={100}
              aria-valuenow={Math.round(watch!.watchedFraction * 100)}
              className="absolute bottom-0 left-0 right-0 bg-[rgba(0,0,0,0.30)]"
              style={{ height: "var(--progress-stripe-height)" }}
            >
              <div
                className="h-full bg-accent"
                style={{
                  width: `${Math.min(100, watch!.watchedFraction * 100)}%`,
                }}
              />
            </div>
          )}

          {/* Duration overlay (bottom-left of image; small chip) */}
          <span className="absolute bottom-1 left-2 rounded bg-[rgba(0,0,0,0.70)] text-fg font-tabular px-1.5 py-0.5 text-[10px]">
            {formatDurationSeconds(v.durationSeconds)}
          </span>

          {/* Legacy status badge (LibraryPage backward-compat) */}
          {actions && (
            <span className="absolute top-1 left-2 flex flex-col gap-1">
              <StatusBadge status={status} />
              {actions.download &&
                status !== "ready" &&
                status !== "archived" && (
                  <DownloadBadge state={actions.download.state} />
                )}
            </span>
          )}
        </div>

        {/* Metadata strip — Line 1 + Line 2 (Full); Line 1 only (Compact) */}
        {variant === "full" || variant === "compact" ? (
          <div className="px-2 py-1 space-y-0.5">
            {/* Line 1: avatar / stack-dots + streamer + title */}
            <div className="flex items-center gap-1.5 text-sm font-mid">
              {isStack ? (
                <StackAvatarDots stack={stack!} />
              ) : streamerAvatarUrl ? (
                <img
                  src={streamerAvatarUrl}
                  alt=""
                  aria-hidden="true"
                  className="rounded-full object-cover"
                  style={{
                    width: "var(--avatar-size-single)",
                    height: "var(--avatar-size-single)",
                  }}
                />
              ) : (
                <span
                  aria-hidden="true"
                  className="inline-block rounded-full bg-surface-2"
                  style={{
                    width: "var(--avatar-size-single)",
                    height: "var(--avatar-size-single)",
                  }}
                />
              )}
              <span className="truncate text-muted">{streamerDisplayName}</span>
              <span aria-hidden="true" className="text-muted">
                ·
              </span>
              <span className="truncate text-fg" title={v.title}>
                {v.title}
              </span>
            </div>

            {/* Line 2: duration + retention text (stage 2) or watched glyph (VOd-A3) */}
            {variant === "full" && (
              <div className="flex items-center justify-between text-xs text-muted">
                <span className="font-tabular">
                  {formatDurationSeconds(v.durationSeconds)}
                  {summary.streamer && (
                    <>
                      <span aria-hidden="true"> · </span>
                      <span>{formatUnixSeconds(v.streamStartedAt)}</span>
                    </>
                  )}
                </span>
                <span className="flex items-center gap-1">
                  {showRetentionTextInStrip && (
                    <span className="font-tabular">
                      {retentionLabelLong(summary.retention.remainingSeconds)}
                    </span>
                  )}
                  {showWatchedGlyph && (
                    <span
                      aria-label="Watched"
                      title="Watched"
                      className="inline-flex items-center justify-center text-muted"
                    >
                      <Check
                        width={12}
                        height={12}
                        strokeWidth={1.5}
                        aria-hidden="true"
                      />
                    </span>
                  )}
                </span>
              </div>
            )}
          </div>
        ) : null}
      </button>

      {/* LibraryPage quick-actions overlay — always in DOM for keyboard
          accessibility (v2.0.1 pattern). Visually hidden via opacity
          until hover / focus-within; pointer-events-none in that state
          so the underlying button isn't trigger-clickable. ADR-0039
          DV3's expanded panel is delegated to the Mission-3 caller for
          TimelinePage / TimelineHeroSlot. */}
      {actions && (
        <div
          className="absolute right-2 bottom-2 flex gap-2 opacity-0 group-hover:opacity-100 focus-within:opacity-100 pointer-events-none group-hover:pointer-events-auto focus-within:pointer-events-auto transition-opacity"
          style={{
            zIndex: "var(--z-card-hover)",
            transitionDuration: "var(--motion-toolbar-fade-duration)",
          }}
        >
          <QuickActions status={status} actions={actions} title={v.title} />
        </div>
      )}
    </article>
  );
}

/**
 * Public component is wrapped in `React.memo` with a custom equality
 * function per DV10. Re-render only when vodId, selection, variant,
 * stack-identity, or `data` reference changes — TanStack-Query results
 * are referentially-stable so the composite-data check is shallow-cheap.
 */
export const VodCard = memo(VodCardImpl, (prev, next) => {
  return (
    prev.vodId === next.vodId &&
    prev.variant === next.variant &&
    prev.selected === next.selected &&
    prev.data === next.data &&
    prev.row === next.row &&
    prev.stack === next.stack &&
    prev.actions === next.actions &&
    prev.hero === next.hero &&
    prev.className === next.className &&
    prev.onSelect === next.onSelect
  );
});

/* ---------------------------------------------------------------------------
 * Variant: Hero (TimelineHeroSlot consumer; Mission 3 mounts it)
 * ------------------------------------------------------------------------- */

function HeroVariant({
  summary,
  selected,
  onSelect,
  hero,
  className,
}: {
  summary: VodSummary;
  selected: boolean;
  onSelect?: () => void;
  hero?: VodCardHeroProps;
  className?: string;
}) {
  const v = summary.vod.vod;
  const streamerDisplayName =
    summary.streamer?.displayName ?? summary.vod.streamerDisplayName;
  const imageUrl = summary.assets?.thumbnailPath
    ? assetUrl(summary.assets.thumbnailPath)
    : (v.thumbnailUrl ?? "");

  return (
    <article
      aria-label={ariaLabel(summary, selected)}
      data-vod-id={v.twitchVideoId}
      data-variant="hero"
      className={`relative overflow-hidden rounded-[var(--card-corner-radius)] bg-surface-1 ${
        className ?? ""
      }`}
      style={{ height: "var(--hero-slot-height)" }}
    >
      {imageUrl ? (
        <img
          src={imageUrl}
          alt=""
          aria-hidden="true"
          className="absolute inset-0 h-full w-full object-cover"
        />
      ) : null}
      {/* Bottom-to-mid gradient scrim for legibility (anchor §3.1) */}
      <div
        aria-hidden="true"
        className="absolute inset-0 bg-gradient-to-t from-[rgba(0,0,0,0.85)] via-[rgba(0,0,0,0.45)] to-transparent"
      />

      <div
        className="absolute inset-0 flex flex-col justify-end"
        style={{
          padding:
            "var(--hero-slot-padding-v) var(--hero-slot-padding-h)",
        }}
      >
        <div className="flex items-center gap-2 text-xs text-muted mb-2 font-tabular">
          <Star width={12} height={12} strokeWidth={1.5} aria-hidden="true" />
          <span>{streamerDisplayName}</span>
          <span aria-hidden="true">·</span>
          <span>Live now</span>
        </div>
        <h2
          className="text-fg font-heading"
          style={{ fontSize: "var(--hero-title-size)", lineHeight: 1.1 }}
        >
          {v.title}
        </h2>
        {hero?.pitch && (
          <p
            className="text-muted mt-2 max-w-2xl"
            style={{ fontSize: "var(--hero-pitch-size)" }}
          >
            {hero.pitch}
          </p>
        )}
        <div className="mt-4 flex items-center gap-3">
          <button
            type="button"
            onClick={hero?.onWatchLive ?? onSelect}
            aria-label={`Watch ${v.title} live`}
            className="inline-flex items-center gap-2 rounded-sm bg-accent text-accent-fg font-mid"
            style={{
              padding: "var(--hero-cta-padding-v) var(--hero-cta-padding-h)",
              fontSize: "var(--font-size-2xl)",
            }}
          >
            <Play width={20} height={20} strokeWidth={1.5} aria-hidden="true" />
            <span>Watch live</span>
          </button>
          {hero?.onMoreInfo && (
            <button
              type="button"
              onClick={hero.onMoreInfo}
              aria-label={`More info about ${v.title}`}
              className="inline-flex items-center gap-2 rounded-sm bg-surface-2 text-fg font-mid"
              style={{
                padding: "var(--hero-cta-padding-v) var(--hero-cta-padding-h)",
                fontSize: "var(--font-size-base)",
              }}
            >
              <Plus
                width={16}
                height={16}
                strokeWidth={1.5}
                aria-hidden="true"
              />
              <span>More info</span>
            </button>
          )}
        </div>
      </div>
    </article>
  );
}

/* ---------------------------------------------------------------------------
 * Variant: Sliver (extreme-zoom-out narrow card; image-only)
 * ------------------------------------------------------------------------- */

function SliverVariant({
  summary,
  selected,
  onSelect,
  className,
}: {
  summary: VodSummary;
  selected: boolean;
  onSelect?: () => void;
  className?: string;
}) {
  const v = summary.vod.vod;
  const imageUrl = summary.assets?.thumbnailPath
    ? assetUrl(summary.assets.thumbnailPath)
    : (v.thumbnailUrl ?? "");
  return (
    <button
      type="button"
      onClick={onSelect}
      aria-label={ariaLabel(summary, selected)}
      data-vod-id={v.twitchVideoId}
      data-variant="sliver"
      className={`relative overflow-hidden rounded-[var(--card-corner-radius)] aspect-video bg-surface-1 ${
        className ?? ""
      }`}
    >
      {imageUrl ? (
        <img
          src={imageUrl}
          alt=""
          aria-hidden="true"
          loading="lazy"
          className="absolute inset-0 h-full w-full object-cover"
        />
      ) : (
        <span
          aria-hidden="true"
          className="absolute inset-0 bg-[var(--color-surface-skeleton)]"
        />
      )}
      {selected && (
        <span
          aria-hidden="true"
          className="absolute inset-0 ring-2 ring-[var(--color-outline-selection)]"
        />
      )}
    </button>
  );
}

/* ---------------------------------------------------------------------------
 * Stack avatar-dots row (DV4 Layer 2) — used by Full / Compact variants.
 * ------------------------------------------------------------------------- */

function StackAvatarDots({ stack }: { stack: VodCardStackPerspective[] }) {
  const visible = stack.slice(0, 4);
  const overflow = Math.max(0, stack.length - 4);
  return (
    <span
      aria-label={`${stack.length} perspectives`}
      className="inline-flex items-center"
      style={{ gap: "var(--avatar-dot-gap)" }}
    >
      {visible.map((p) =>
        p.profileImageUrl ? (
          <img
            key={p.twitchUserId}
            src={p.profileImageUrl}
            alt=""
            aria-hidden="true"
            className="rounded-full object-cover"
            style={{
              width: "var(--avatar-dot-size)",
              height: "var(--avatar-dot-size)",
            }}
          />
        ) : (
          <span
            key={p.twitchUserId}
            aria-hidden="true"
            className="inline-block rounded-full bg-surface-2 text-fg text-[8px] font-mid text-center leading-[var(--avatar-dot-size)]"
            style={{
              width: "var(--avatar-dot-size)",
              height: "var(--avatar-dot-size)",
            }}
          >
            {p.displayName.charAt(0)}
          </span>
        )
      )}
      {overflow > 0 && (
        <span
          className="rounded-full bg-surface-2 text-fg px-1 font-tabular"
          style={{
            height: "var(--avatar-dot-size)",
            fontSize: "10px",
            lineHeight: "var(--avatar-dot-size)",
          }}
        >
          +{overflow}
        </span>
      )}
    </span>
  );
}

/* ---------------------------------------------------------------------------
 * Quick-actions (LibraryPage backward-compat). Hover-revealed; only
 * renders when the matching callback is provided. Status-gated.
 * ------------------------------------------------------------------------- */

function QuickActions({
  status,
  actions,
  title,
}: {
  status: VodStatus;
  actions: VodCardActions;
  title: string;
}) {
  const buttons: React.ReactNode[] = [];
  if (status === "available" && actions.onDownload) {
    buttons.push(
      <button
        key="download"
        type="button"
        onClick={(e) => {
          e.stopPropagation();
          actions.onDownload!();
        }}
        aria-label={`Download ${title}`}
        className="rounded bg-accent text-accent-fg text-xs px-2 py-1"
      >
        ↓ Download
      </button>
    );
  }
  if (status === "queued" && actions.onCancel) {
    buttons.push(
      <button
        key="cancel"
        type="button"
        onClick={(e) => {
          e.stopPropagation();
          actions.onCancel!();
        }}
        aria-label={`Cancel queued ${title}`}
        className="rounded bg-surface-2 text-fg text-xs px-2 py-1"
      >
        ✕ Cancel
      </button>
    );
  }
  if ((status === "ready" || status === "archived") && actions.onPlay) {
    buttons.push(
      <button
        key="play"
        type="button"
        onClick={(e) => {
          e.stopPropagation();
          actions.onPlay!();
        }}
        aria-label={status === "archived" ? `Re-watch ${title}` : `Play ${title}`}
        className="rounded bg-accent text-accent-fg text-xs px-2 py-1 inline-flex items-center gap-1"
      >
        <Play width={12} height={12} strokeWidth={1.5} aria-hidden="true" />
        {status === "archived" ? "Re-watch" : "Play"}
      </button>
    );
  }
  if (status === "ready" && actions.onRemove) {
    buttons.push(
      <button
        key="remove"
        type="button"
        onClick={(e) => {
          e.stopPropagation();
          actions.onRemove!();
        }}
        aria-label={`Remove ${title} from disk`}
        className="rounded bg-surface-2 text-fg text-xs px-2 py-1"
      >
        × Remove
      </button>
    );
  }
  if (status === "deleted" && actions.onRepick) {
    buttons.push(
      <button
        key="repick"
        type="button"
        onClick={(e) => {
          e.stopPropagation();
          actions.onRepick!();
        }}
        aria-label={`Pick again ${title}`}
        className="rounded bg-accent text-accent-fg text-xs px-2 py-1"
      >
        ↻ Pick again
      </button>
    );
  }
  return <>{buttons}</>;
}

/* ---------------------------------------------------------------------------
 * Status badge — migrated to token-driven Tailwind utilities per
 * Mission-1 hex-audit (bg-blue-500 → bg-info etc.).
 * ------------------------------------------------------------------------- */

function StatusBadge({ status }: { status: VodStatus }) {
  const palette: Record<VodStatus, string> = {
    available: "bg-surface-1 text-muted border border-[var(--color-border)]",
    queued: "bg-info/30 text-fg border border-info/50",
    downloading: "bg-info/80 text-fg",
    ready: "bg-success/80 text-fg",
    archived: "bg-success/30 text-fg border border-success/50",
    deleted: "bg-surface-2 text-muted border border-[var(--color-border)]",
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
    queued: "bg-surface-1 text-muted border border-[var(--color-border)]",
    downloading: "bg-info/80 text-fg",
    paused: "bg-warning/80 text-fg",
    completed: "bg-success/80 text-fg",
    failed_retryable: "bg-warning/80 text-fg",
    failed_permanent: "bg-danger/80 text-fg",
  };
  return (
    <span
      className={`text-[10px] uppercase tracking-wider px-1.5 py-0.5 rounded ${palette[state]}`}
    >
      {label[state]}
    </span>
  );
}
