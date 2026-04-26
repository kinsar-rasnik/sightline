import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import {
  commands,
  type EncoderCapability,
  type VideoQualityProfile,
} from "@/ipc";

export const ENCODER_CAPABILITY_KEY = ["quality", "encoder-capability"] as const;

/**
 * Persisted encoder-capability snapshot (or null if detection has
 * never run).  The startup flow runs detection in a background
 * tokio task so a freshly-installed app may show null briefly
 * before the first probe completes.
 */
export function useEncoderCapability() {
  return useQuery<EncoderCapability | null>({
    queryKey: ENCODER_CAPABILITY_KEY,
    queryFn: () => commands.getEncoderCapability(),
    // Snapshot is cheap to fetch; refresh once per minute so the
    // "tested NN minutes ago" label stays roughly accurate.
    refetchInterval: 60_000,
  });
}

/**
 * Force a re-detection probe.  Triggers the 1-second test encode +
 * `ffmpeg -encoders` parse on the backend, then writes the result
 * to settings and returns the new capability.
 */
export function useRedetectEncoders() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: () => commands.redetectEncoders(),
    onSuccess: (next) => {
      qc.setQueryData(ENCODER_CAPABILITY_KEY, next);
      qc.invalidateQueries({ queryKey: ["settings"] });
    },
  });
}

/**
 * Persist the chosen quality profile.  Uses the dedicated
 * `setVideoQualityProfile` command rather than `updateSettings` so
 * the Settings UI can render an inline pending state without
 * affecting the rest of the settings query cache.
 */
export function useSetVideoQualityProfile() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (profile: VideoQualityProfile) =>
      commands.setVideoQualityProfile({ profile }),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["settings"] });
    },
  });
}
