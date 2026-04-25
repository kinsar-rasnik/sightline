import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import {
  commands,
  type CheckForUpdateInput,
  type UpdateInfo,
  type UpdateStatus,
} from "@/ipc";

export function useUpdateStatus() {
  return useQuery<UpdateStatus>({
    queryKey: ["update-status"],
    queryFn: () => commands.getUpdateStatus(),
    refetchInterval: 60_000,
  });
}

export function useCheckForUpdate() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (input: CheckForUpdateInput): Promise<UpdateInfo | null> =>
      commands.checkForUpdate(input),
    onSettled: () => {
      qc.invalidateQueries({ queryKey: ["update-status"] });
    },
  });
}

export function useSkipUpdateVersion() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (version: string) =>
      commands.skipUpdateVersion({ version }),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["update-status"] });
    },
  });
}
