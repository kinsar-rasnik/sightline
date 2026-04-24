import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import {
  commands,
  type ContinueWatchingEntry,
  type WatchProgressRow,
  type WatchStats,
} from "@/ipc";

/** Fetch the stored progress row for a single VOD. */
export function useWatchProgress(vodId: string | null) {
  return useQuery<WatchProgressRow | null>({
    queryKey: ["watch-progress", vodId],
    queryFn: () => commands.getWatchProgress({ vodId: vodId ?? "" }),
    enabled: vodId !== null,
    staleTime: 2_000,
  });
}

/** Debounced player `timeupdate` hook — the caller controls throttling. */
export function useUpdateWatchProgress() {
  const qc = useQueryClient();
  return useMutation<
    WatchProgressRow,
    Error,
    { vodId: string; positionSeconds: number; durationSeconds: number }
  >({
    mutationFn: (input) =>
      commands.updateWatchProgress({
        vodId: input.vodId,
        positionSeconds: input.positionSeconds,
        durationSeconds: input.durationSeconds,
      }),
    onSuccess: (row) => {
      qc.setQueryData(["watch-progress", row.vodId], row);
      // Continue-watching and the grid state bar depend on this too.
      qc.invalidateQueries({ queryKey: ["continue-watching"] });
    },
  });
}

export function useMarkWatched() {
  const qc = useQueryClient();
  return useMutation<WatchProgressRow, Error, string>({
    mutationFn: (vodId) => commands.markWatched({ vodId }),
    onSuccess: (row) => {
      qc.setQueryData(["watch-progress", row.vodId], row);
      qc.invalidateQueries({ queryKey: ["continue-watching"] });
    },
  });
}

export function useMarkUnwatched() {
  const qc = useQueryClient();
  return useMutation<WatchProgressRow, Error, string>({
    mutationFn: (vodId) => commands.markUnwatched({ vodId }),
    onSuccess: (row) => {
      qc.setQueryData(["watch-progress", row.vodId], row);
      qc.invalidateQueries({ queryKey: ["continue-watching"] });
    },
  });
}

export function useContinueWatching(limit = 12) {
  return useQuery<ContinueWatchingEntry[]>({
    queryKey: ["continue-watching", limit],
    queryFn: () => commands.listContinueWatching({ limit }),
    staleTime: 5_000,
  });
}

export function useWatchStats(streamerId: string | null = null) {
  return useQuery<WatchStats>({
    queryKey: ["watch-stats", streamerId],
    queryFn: () =>
      commands.getWatchStats({
        ...(streamerId ? { streamerId } : {}),
      }),
    staleTime: 15_000,
  });
}
