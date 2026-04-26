import { useEffect, useRef } from "react";

import { commands } from "@/ipc";

/**
 * Watch-progress fraction at which the pre-fetch hook fires
 * (ADR-0031). Calibrated so a 6h GTA-RP VOD pre-fetches its
 * successor with ~1h of watching to spare — long enough for the
 * download to land before the user reaches the end.
 */
export const PREFETCH_PROGRESS_THRESHOLD = 0.7;

/**
 * Alternative trigger: VODs short enough that 70% leaves <2 min of
 * runway also pre-fetch when remaining time falls below this floor.
 * Avoids the late-trigger edge case for sub-7-minute clips.
 */
export const PREFETCH_REMAINING_SECONDS_THRESHOLD = 120;

/**
 * Pure decision step exposed for unit tests. Returns `true` when
 * the player should fire `commands.prefetchCheck`. The "already
 * triggered" guard is the in-session throttle — once a VOD has
 * fired, it won't fire again for the same session even if the user
 * scrubs back and forth across the threshold.
 */
export function shouldTriggerPrefetch({
  currentSeconds,
  durationSeconds,
  alreadyTriggered,
}: {
  currentSeconds: number;
  durationSeconds: number;
  alreadyTriggered: boolean;
}): boolean {
  if (alreadyTriggered) return false;
  if (!Number.isFinite(durationSeconds) || durationSeconds <= 0) return false;
  if (!Number.isFinite(currentSeconds) || currentSeconds < 0) return false;

  const progress = currentSeconds / durationSeconds;
  const remaining = durationSeconds - currentSeconds;
  return (
    progress >= PREFETCH_PROGRESS_THRESHOLD ||
    remaining <= PREFETCH_REMAINING_SECONDS_THRESHOLD
  );
}

interface UsePrefetchHookArgs {
  vodId: string | null;
  currentSeconds: number;
  durationSeconds: number;
}

/**
 * ADR-0031 player hook — fires `commands.prefetchCheck` when the
 * watch-progress crosses the threshold. Throttled per VOD per
 * session via a `useRef`-held `Set<vodId>` so the 5-second progress
 * cadence can't trigger a thousand prefetch_check calls.
 *
 * Pre-fetch failures are silent: the user's playback shouldn't be
 * disturbed by a non-blocking optimisation that didn't fire.
 */
export function usePrefetchHook({
  vodId,
  currentSeconds,
  durationSeconds,
}: UsePrefetchHookArgs): void {
  const triggeredRef = useRef<Set<string>>(new Set());

  useEffect(() => {
    if (!vodId) return;
    const alreadyTriggered = triggeredRef.current.has(vodId);
    if (
      !shouldTriggerPrefetch({
        currentSeconds,
        durationSeconds,
        alreadyTriggered,
      })
    ) {
      return;
    }
    triggeredRef.current.add(vodId);
    void commands.prefetchCheck({ vodId }).catch(() => {
      // Pre-fetch failures are non-blocking by design.  The
      // distribution service already logs at warn level on the
      // backend; the renderer doesn't need to surface anything.
    });
  }, [vodId, currentSeconds, durationSeconds]);
}
