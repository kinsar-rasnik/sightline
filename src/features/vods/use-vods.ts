import { useQuery } from "@tanstack/react-query";

import { commands, type ListVodsInput, type VodWithChapters } from "@/ipc";

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
