import { useQuery } from "@tanstack/react-query";

import {
  commands,
  type ListVodsInput,
  type VodAssets,
  type VodWithChapters,
} from "@/ipc";

export function useVods(input: ListVodsInput) {
  return useQuery<VodWithChapters[]>({
    queryKey: ["vods", input],
    queryFn: () => commands.listVods(input),
  });
}

export function useVod(twitchVideoId: string | null) {
  return useQuery<VodWithChapters>({
    queryKey: ["vod", twitchVideoId],
    queryFn: () => commands.getVod({ twitchVideoId: twitchVideoId ?? "" }),
    enabled: twitchVideoId !== null,
  });
}

/**
 * Fetch the media-asset bundle (video + thumbnail + preview frames)
 * for a VOD. Enabled only when a vod_id is supplied so a disabled
 * hook at render time doesn't over-fetch.
 */
export function useVodAssets(vodId: string | null) {
  return useQuery<VodAssets>({
    queryKey: ["vod-assets", vodId],
    queryFn: () => commands.getVodAssets({ vodId: vodId ?? "" }),
    enabled: vodId !== null,
    // Paths change only when a download completes, migrates, or the
    // backfill task runs — all of which fire events elsewhere. Ten
    // seconds is a comfortable refetch cadence for hover-grid state.
    staleTime: 10_000,
  });
}
