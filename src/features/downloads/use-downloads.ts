import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import {
  commands,
  type DownloadFilters,
  type DownloadRow,
  type LibraryInfo,
  type StagingInfo,
} from "@/ipc";

export function useDownloads(filters: DownloadFilters = {}) {
  return useQuery<DownloadRow[]>({
    queryKey: ["downloads", filters],
    queryFn: () => commands.listDownloads({ filters }),
    refetchInterval: 3_000, // short fallback in case events miss a beat
  });
}

export function useStagingInfo() {
  return useQuery<StagingInfo>({
    queryKey: ["staging-info"],
    queryFn: () => commands.getStagingInfo(),
  });
}

export function useLibraryInfo() {
  return useQuery<LibraryInfo>({
    queryKey: ["library-info"],
    queryFn: () => commands.getLibraryInfo(),
  });
}

function invalidateAll(qc: ReturnType<typeof useQueryClient>) {
  qc.invalidateQueries({ queryKey: ["downloads"] });
  qc.invalidateQueries({ queryKey: ["library-info"] });
}

export function useEnqueueDownload() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (vodId: string) => commands.enqueueDownload({ vodId }),
    onSuccess: () => invalidateAll(qc),
  });
}

export function usePauseDownload() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (vodId: string) => commands.pauseDownload({ vodId }),
    onSuccess: () => invalidateAll(qc),
  });
}

export function useResumeDownload() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (vodId: string) => commands.resumeDownload({ vodId }),
    onSuccess: () => invalidateAll(qc),
  });
}

export function useCancelDownload() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (vodId: string) => commands.cancelDownload({ vodId }),
    onSuccess: () => invalidateAll(qc),
  });
}

export function useRetryDownload() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (vodId: string) => commands.retryDownload({ vodId }),
    onSuccess: () => invalidateAll(qc),
  });
}
