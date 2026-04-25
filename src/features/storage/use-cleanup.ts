import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import {
  commands,
  type CleanupLogEntry,
  type CleanupPlan,
  type CleanupResult,
  type DiskUsage,
  type ExecuteCleanupInput,
} from "@/ipc";

export function useDiskUsage() {
  return useQuery<DiskUsage>({
    queryKey: ["disk-usage"],
    queryFn: () => commands.getDiskUsage(),
    // The Storage section polls at a relaxed cadence — disk usage
    // changes slowly compared to the rest of the UI.
    refetchInterval: 60_000,
  });
}

export function useCleanupPlan(enabled: boolean) {
  return useQuery<CleanupPlan>({
    queryKey: ["cleanup-plan"],
    queryFn: () => commands.getCleanupPlan(),
    enabled,
  });
}

export function useCleanupHistory(limit = 25) {
  return useQuery<CleanupLogEntry[]>({
    queryKey: ["cleanup-history", limit],
    queryFn: () => commands.getCleanupHistory({ limit }),
  });
}

export function useExecuteCleanup() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (input: ExecuteCleanupInput): Promise<CleanupResult> =>
      commands.executeCleanup(input),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["disk-usage"] });
      qc.invalidateQueries({ queryKey: ["cleanup-history"] });
      qc.invalidateQueries({ queryKey: ["cleanup-plan"] });
      qc.invalidateQueries({ queryKey: ["downloads"] });
      qc.invalidateQueries({ queryKey: ["library-info"] });
    },
  });
}
