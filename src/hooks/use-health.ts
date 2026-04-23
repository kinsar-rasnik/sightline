import { useQuery } from "@tanstack/react-query";

import { commands, type HealthReport } from "@/ipc";

export function useHealth() {
  return useQuery<HealthReport>({
    queryKey: ["health"],
    queryFn: () => commands.health(),
    refetchInterval: 10_000,
  });
}
