import { useQuery } from "@tanstack/react-query";

import { commands, type ForecastResult, type GlobalForecast } from "@/ipc";

/**
 * Per-streamer storage forecast (ADR-0032).  Pulled lazily — the
 * Streamer-Add dialog calls this once per (login, settings) tuple
 * to populate the forecast box.  The 30s staleTime keeps the
 * estimate stable while the user fiddles with the form, and the
 * `enabled` flag lets the dialog defer the fetch until the user
 * has actually entered a login.
 */
export function useStreamerForecast(twitchUserId: string | null) {
  return useQuery<ForecastResult>({
    queryKey: ["forecast", "streamer", twitchUserId],
    queryFn: () =>
      commands.estimateStreamerFootprint({ twitchUserId: twitchUserId ?? "" }),
    enabled: !!twitchUserId,
    staleTime: 30_000,
  });
}

/**
 * Global storage forecast: combined totals + per-streamer
 * breakdown.  Drives the Settings → Storage Outlook section.
 */
export function useGlobalForecast() {
  return useQuery<GlobalForecast>({
    queryKey: ["forecast", "global"],
    queryFn: () => commands.estimateGlobalFootprint(),
    // Forecast inputs (vods, settings, free disk) shift slowly;
    // 60s is a reasonable cache before the user benefits from a
    // refresh.  The Settings section can re-fetch on-mount via
    // refetchOnMount to bust the cache when the user navigates
    // back from a streamer add.
    staleTime: 60_000,
  });
}
