import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import { commands, type AppSettings, type SettingsPatch } from "@/ipc";

export function useSettings() {
  return useQuery<AppSettings>({
    queryKey: ["settings"],
    queryFn: () => commands.getSettings(),
  });
}

export function useUpdateSettings() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (patch: SettingsPatch) => commands.updateSettings(patch),
    onSuccess: (next) => {
      qc.setQueryData(["settings"], next);
    },
  });
}

export function useSetCredentials() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (input: { clientId: string; clientSecret: string }) =>
      commands.setTwitchCredentials(input),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["settings"] });
    },
  });
}

export function useClearCredentials() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: () => commands.clearTwitchCredentials(),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["settings"] });
    },
  });
}
