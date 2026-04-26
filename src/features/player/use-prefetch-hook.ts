import { useEffect } from "react";

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
 * Minimum elapsed watching time before either trigger arm can
 * fire. Guards the short-clip edge case — without this, a 90s clip
 * opened from `currentSeconds = 0` would immediately satisfy the
 * remaining-floor branch (`90 < 120`) and pre-fetch its successor
 * before the user has watched a single frame. Five seconds is
 * short enough to feel instantaneous but long enough to confirm
 * the user actually started watching rather than briefly scrubbing
 * past.
 */
export const MIN_WATCHED_SECONDS_BEFORE_PREFETCH = 5;

/**
 * Module-level "this VOD has already triggered" set — promoted
 * out of `useRef` so the throttle scope matches the app session,
 * not the component mount. A user who closes and re-opens VOD K
 * mid-watch shouldn't re-fire the pre-fetch check; the previous
 * mount already settled K+1's state. Survives HMR in dev (Vite
 * keeps module instances) but resets on a full app reload, which
 * matches the documented "per session" intent.
 *
 * Exposed (read-only) for tests via `_clearForTests`.
 */
const triggeredVods = new Set<string>();

/** @internal Test-only: clear the module-level throttle set. */
export function _clearTriggeredVodsForTests(): void {
  triggeredVods.clear();
}

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
  // Without this guard a short clip mounted at currentSeconds = 0
  // would satisfy the remaining-floor branch immediately. Apply to
  // both branches uniformly for symmetry — the progress branch can
  // never realistically fire below 5s anyway (< 5/duration).
  if (currentSeconds < MIN_WATCHED_SECONDS_BEFORE_PREFETCH) return false;

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
 * app session via a module-level `Set<vodId>` so the 5-second
 * progress cadence can't trigger a thousand prefetch_check calls,
 * and a re-mount of the player on the same VOD doesn't fire a
 * duplicate.
 *
 * Pre-fetch failures are silent: the user's playback shouldn't be
 * disturbed by a non-blocking optimisation that didn't fire.
 */
export function usePrefetchHook({
  vodId,
  currentSeconds,
  durationSeconds,
}: UsePrefetchHookArgs): void {
  useEffect(() => {
    if (!vodId) return;
    const alreadyTriggered = triggeredVods.has(vodId);
    if (
      !shouldTriggerPrefetch({
        currentSeconds,
        durationSeconds,
        alreadyTriggered,
      })
    ) {
      return;
    }
    triggeredVods.add(vodId);
    void commands.prefetchCheck({ vodId }).catch(() => {
      // Pre-fetch failures are non-blocking by design.  The
      // distribution service already logs at warn level on the
      // backend; the renderer doesn't need to surface anything.
    });
  }, [vodId, currentSeconds, durationSeconds]);
}
