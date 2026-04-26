import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import { commands, type PollStatusRow, type StreamerSummary } from "@/ipc";

export function useStreamers() {
  return useQuery<StreamerSummary[]>({
    queryKey: ["streamers"],
    queryFn: () => commands.listStreamers(),
  });
}

export function usePollStatus() {
  return useQuery<PollStatusRow[]>({
    queryKey: ["poll-status"],
    queryFn: () => commands.getPollStatus(),
    refetchInterval: 15_000,
  });
}

export function useAddStreamer() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (login: string) => commands.addStreamer({ login }),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["streamers"] });
      qc.invalidateQueries({ queryKey: ["poll-status"] });
      // R-RC-02 fix: bust the global storage forecast cache so
      // Settings → Storage Outlook reflects the new streamer
      // immediately rather than waiting for the 60s staleTime.
      qc.invalidateQueries({ queryKey: ["forecast"] });
    },
  });
}

export function useRemoveStreamer() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (twitchUserId: string) =>
      commands.removeStreamer({ twitchUserId }),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["streamers"] });
      qc.invalidateQueries({ queryKey: ["poll-status"] });
      qc.invalidateQueries({ queryKey: ["forecast"] });
    },
  });
}

export function useTriggerPoll() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (twitchUserId: string | null) =>
      commands.triggerPoll({ twitchUserId }),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["poll-status"] });
    },
  });
}
