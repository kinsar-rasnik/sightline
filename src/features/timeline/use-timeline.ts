import { useQuery } from "@tanstack/react-query";

import { commands, type ListTimelineInput, type TimelineFilters } from "@/ipc";

export function useTimeline(filters: TimelineFilters | undefined) {
  const input: ListTimelineInput = { filters };
  return useQuery({
    queryKey: ["timeline", "list", filters],
    queryFn: () => commands.listTimeline(input),
    staleTime: 30_000,
  });
}

export function useTimelineStats() {
  return useQuery({
    queryKey: ["timeline", "stats"],
    queryFn: () => commands.getTimelineStats(),
    staleTime: 30_000,
  });
}

export function useCoStreams(vodId: string | null) {
  return useQuery({
    queryKey: ["timeline", "co-streams", vodId],
    queryFn: () => commands.getCoStreams({ vodId: vodId ?? "" }),
    enabled: !!vodId,
    staleTime: 30_000,
  });
}
