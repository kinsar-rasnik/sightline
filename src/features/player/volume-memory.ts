import type { VolumeMemory } from "./use-playback-prefs";

/**
 * Per-VOD + global volume persistence. The Settings → Playback →
 * Volume memory toggle picks one of three branches:
 *
 * - `global` → one shared key, every VOD inherits the last value the
 *   user set anywhere.
 * - `vod`    → per-VOD key, with a global fallback for first-time
 *   playback so the user doesn't get blasted by 100 % on a new VOD
 *   when they've been listening at 30 % everywhere else.
 * - `session` → no persistence; in-memory only, resets to default on
 *   reload.
 *
 * Closes Phase-5 deferral #6: `usePlaybackPrefs` exposes the setting
 * but the `vod` branch was previously a no-op.
 */

const GLOBAL_KEY = "sightline:player:volume:global";
const VOD_KEY_PREFIX = "sightline:player:volume:vod:";

const DEFAULT_VOLUME = 1;

function vodKey(vodId: string): string {
  return `${VOD_KEY_PREFIX}${vodId}`;
}

function clampVolume(n: number): number {
  if (!Number.isFinite(n)) return DEFAULT_VOLUME;
  return Math.max(0, Math.min(1, n));
}

function safeRead(key: string): number | null {
  if (typeof window === "undefined" || !window.localStorage) return null;
  try {
    const raw = window.localStorage.getItem(key);
    if (raw == null) return null;
    const n = Number(raw);
    return Number.isFinite(n) ? clampVolume(n) : null;
  } catch {
    return null;
  }
}

/**
 * Read the persisted volume for the supplied policy + VOD. The `vod`
 * branch falls back to the global key so a freshly-opened VOD doesn't
 * default to 100 % when the user has been playing everything else at
 * a quieter setting.
 */
export function readVolume(memory: VolumeMemory, vodId: string): number {
  if (memory === "session") return DEFAULT_VOLUME;
  if (memory === "vod") {
    const perVod = safeRead(vodKey(vodId));
    if (perVod != null) return perVod;
    return safeRead(GLOBAL_KEY) ?? DEFAULT_VOLUME;
  }
  return safeRead(GLOBAL_KEY) ?? DEFAULT_VOLUME;
}

/**
 * Persist a fresh volume value under whichever key the policy
 * dictates. `session` is a no-op. `vod` writes both the per-VOD key
 * AND the global key — the global write keeps the fallback chain
 * coherent so toggling the policy back to `global` doesn't leave the
 * user on a stale value from before they switched to per-VOD.
 */
export function writeVolume(
  memory: VolumeMemory,
  vodId: string,
  volume: number
): void {
  if (memory === "session") return;
  if (typeof window === "undefined" || !window.localStorage) return;
  const clamped = clampVolume(volume);
  try {
    if (memory === "vod") {
      window.localStorage.setItem(vodKey(vodId), String(clamped));
    }
    window.localStorage.setItem(GLOBAL_KEY, String(clamped));
  } catch {
    /* quota — drop the write and accept session-only volume */
  }
}
