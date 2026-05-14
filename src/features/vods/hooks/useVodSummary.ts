/**
 * Composite hook returning the data surface a v2.1 VodCard needs.
 *
 * Per ADR-0039 DV8 + ADR-0038 D8: NO NEW IPC ENDPOINTS. We aggregate
 * existing queries via TanStack-Query's cross-component cache
 * deduplication:
 *
 *   - `useVod(vodId)`          — vod + chapters
 *   - `useVodAssets(vodId)`    — thumbnail + 6 preview frames
 *   - `useWatchProgress(vodId)`— watched_fraction + state
 *   - `useStreamers()`         — listStreamers, shared cache key
 *     (single fetch across N cards, then per-vod lookup by twitchUserId)
 *
 * The composite is **referentially stable** as long as its constituent
 * TanStack-Query cache entries are unchanged — `React.memo` boundaries
 * downstream avoid unnecessary VodCard re-renders per DV10.
 *
 * Retention stage is computed client-side per VOd-A2 (binding): pure
 * derivation from `streamer.broadcasterType` + `vod.streamStartedAt`,
 * no backend touch. The `retentionAt` UNIX-seconds value is exposed for
 * tabular display; the stage discriminant drives the VodCard
 * composition per DV5.
 *
 * `coStreams` is intentionally NOT fetched here — stack-card logic is
 * TimelinePage-surface (Mission 3) which subscribes to `useCoStreams`
 * separately and passes the result to `VodCard` via a prop. Keeping
 * coStreams out of the composite avoids ~N IPC calls when this hook
 * mounts on surfaces that don't need stack-rendering (LibraryPage,
 * ContinueWatchingRow).
 */

import { useMemo } from "react";
import { useQuery } from "@tanstack/react-query";

import { useStreamers } from "@/features/streamers/use-streamers";
import { useWatchProgress } from "@/features/player/use-watch-progress";
import {
  commands,
  type StreamerSummary,
  type VodAssets,
  type VodWithChapters,
  type WatchProgressRow,
} from "@/ipc";

// `Streamer` is the inner shape of `StreamerSummary`. The IPC wrapper
// exposes `StreamerSummary` (which embeds the row); we derive the inner
// type to avoid reaching into `@/ipc/bindings` directly.
type Streamer = StreamerSummary["streamer"];

/**
 * Retention stage discriminant per ADR-0039 DV5 + Q-7 anchor binding.
 *
 *   stage 1 → > 7 d   : no indicator
 *   stage 2 → 7 d ≥ X > 48 h  : strip Line 2 text label
 *   stage 3 → 48 h ≥ X > 24 h : neutral badge in image-area overlay
 *   stage 4 → ≤ 24 h          : accent badge in image-area overlay
 *   expired→ ≤ 0
 */
export type RetentionStage = 1 | 2 | 3 | 4 | "expired";

export interface RetentionInfo {
  /** Stage 1..4 or `"expired"`; drives VodCard composition. */
  stage: RetentionStage;
  /** UNIX seconds when the VOD will be removed from Twitch CDN. */
  retentionAt: number;
  /** Seconds remaining until expiry; may be negative for expired. */
  remainingSeconds: number;
}

/**
 * Twitch retention tiers per `broadcasterType`. Public-Twitch-CDN
 * defaults; the same constants ADR-0039 DV5 documents.
 */
function retentionDays(broadcasterType: string | null | undefined): number {
  switch (broadcasterType) {
    case "partner":
      return 60;
    case "affiliate":
      return 14;
    default:
      return 7;
  }
}

export function computeRetention(
  broadcasterType: string | null | undefined,
  streamStartedAt: number,
  nowSec: number = Math.floor(Date.now() / 1000)
): RetentionInfo {
  const retentionAt = streamStartedAt + retentionDays(broadcasterType) * 86_400;
  const remainingSeconds = retentionAt - nowSec;
  let stage: RetentionStage;
  if (remainingSeconds <= 0) stage = "expired";
  else if (remainingSeconds > 7 * 86_400) stage = 1;
  else if (remainingSeconds > 48 * 3600) stage = 2;
  else if (remainingSeconds > 24 * 3600) stage = 3;
  else stage = 4;
  return { stage, retentionAt, remainingSeconds };
}

/**
 * The DTO a v2.1 VodCard renders from. Shape matches ADR-0039 DV8
 * `VodSummary` with `coStreams` lifted to the stack-card-aware caller.
 */
export interface VodSummary {
  vod: VodWithChapters;
  /** Single streamer lookup; `null` if the streamer row is missing
   *  (e.g., streamer was removed but the VOD row persists). */
  streamer: Streamer | null;
  assets: VodAssets | null;
  watchProgress: WatchProgressRow | null;
  retention: RetentionInfo;
}

export interface UseVodSummaryResult {
  data: VodSummary | null;
  /** True while any of the underlying queries are loading. */
  isLoading: boolean;
  /** First error from any underlying query, or null. */
  error: Error | null;
}

/**
 * Internal asset hook lifted here to avoid importing `useVodAssets`
 * from `use-vods.ts` which mutates the existing `vod-assets` query key
 * (we want the same cache, same staleTime; this re-exports cleanly).
 */
function useVodAssetsInternal(vodId: string | null) {
  return useQuery<VodAssets>({
    queryKey: ["vod-assets", vodId],
    queryFn: () => commands.getVodAssets({ vodId: vodId ?? "" }),
    enabled: vodId !== null,
    staleTime: 10_000,
  });
}

function useVodInternal(vodId: string | null) {
  return useQuery<VodWithChapters>({
    queryKey: ["vod", vodId],
    queryFn: () => commands.getVod({ twitchVideoId: vodId ?? "" }),
    enabled: vodId !== null,
  });
}

/**
 * Composite VodSummary hook. Returns a stable `VodSummary` reference
 * across renders so long as the underlying TanStack-Query cache entries
 * are unchanged — `React.memo` on the VodCard reads it as one prop.
 */
export function useVodSummary(vodId: string | null): UseVodSummaryResult {
  const vodQuery = useVodInternal(vodId);
  const assetsQuery = useVodAssetsInternal(vodId);
  const watchQuery = useWatchProgress(vodId);
  const streamersQuery = useStreamers();

  const data = useMemo<VodSummary | null>(() => {
    if (!vodQuery.data) return null;
    const vod = vodQuery.data;
    const streamerSummary = streamersQuery.data?.find(
      (s) => s.streamer.twitchUserId === vod.vod.twitchUserId
    );
    const streamer = streamerSummary?.streamer ?? null;
    const retention = computeRetention(
      streamer?.broadcasterType ?? null,
      vod.vod.streamStartedAt
    );
    return {
      vod,
      streamer,
      assets: assetsQuery.data ?? null,
      watchProgress: watchQuery.data ?? null,
      retention,
    };
  }, [
    vodQuery.data,
    assetsQuery.data,
    watchQuery.data,
    streamersQuery.data,
  ]);

  const isLoading =
    vodQuery.isLoading ||
    assetsQuery.isLoading ||
    watchQuery.isLoading ||
    streamersQuery.isLoading;

  const error =
    (vodQuery.error as Error | null) ||
    (assetsQuery.error as Error | null) ||
    (watchQuery.error as Error | null) ||
    (streamersQuery.error as Error | null) ||
    null;

  return { data, isLoading, error };
}
