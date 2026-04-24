import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import { commands, type AutostartStatus } from "@/ipc";

/**
 * Read the autostart status (both the DB-desired and the OS-observed
 * value). The two diverge when the user toggles the OS registration
 * directly through System Settings / Task Scheduler — in that case
 * the OS value is authoritative.
 */
export function useAutostartStatus() {
  return useQuery<AutostartStatus>({
    queryKey: ["autostart-status"],
    queryFn: () => commands.getAutostartStatus(),
    staleTime: 15_000,
  });
}

/**
 * Flip the autostart setting. Writes the OS registration, then
 * mirrors the OS readback into the DB so a failed write doesn't
 * leave the two diverged.
 */
export function useSetAutostart() {
  const qc = useQueryClient();
  return useMutation<AutostartStatus, Error, boolean>({
    mutationFn: (enabled) => commands.setAutostart({ enabled }),
    onSuccess: (status) => {
      qc.setQueryData(["autostart-status"], status);
      // Settings page also shows `startAtLogin`; invalidate so that
      // card refetches with the OS-anchored value.
      qc.invalidateQueries({ queryKey: ["settings"] });
    },
  });
}
