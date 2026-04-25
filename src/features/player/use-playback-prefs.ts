import { useSyncExternalStore } from "react";

import {
  DEFAULT_AUTO_PLAY_ON_OPEN,
  DEFAULT_PRE_ROLL_SECONDS,
  type PlaybackSpeed,
  clampSpeed,
} from "./player-constants";

export type VolumeMemory = "session" | "vod" | "global";

// Phase 6 housekeeping: `completionThreshold` moved to backend
// `app_settings.completion_threshold` (migration 0009) so the watch
// state machine in `cmd_update_watch_progress` reads the same value
// the user sets in Settings. It is no longer kept in localStorage —
// the Settings UI talks to `update_settings` directly.

export interface PlaybackPrefs {
  autoplay: boolean;
  preRollSeconds: number;
  defaultSpeed: PlaybackSpeed;
  volumeMemory: VolumeMemory;
  pipOnBlur: boolean;
}

const DEFAULTS: PlaybackPrefs = {
  autoplay: DEFAULT_AUTO_PLAY_ON_OPEN,
  preRollSeconds: DEFAULT_PRE_ROLL_SECONDS,
  defaultSpeed: 1,
  volumeMemory: "global",
  pipOnBlur: false,
};

const STORAGE_KEY = "sightline:playback-prefs";

function readPrefs(): PlaybackPrefs {
  if (typeof window === "undefined" || !window.localStorage) return DEFAULTS;
  try {
    const raw = window.localStorage.getItem(STORAGE_KEY);
    if (!raw) return DEFAULTS;
    const parsed = JSON.parse(raw) as Partial<PlaybackPrefs>;
    return {
      autoplay:
        typeof parsed.autoplay === "boolean"
          ? parsed.autoplay
          : DEFAULTS.autoplay,
      preRollSeconds: Math.min(
        30,
        Math.max(
          0,
          typeof parsed.preRollSeconds === "number"
            ? parsed.preRollSeconds
            : DEFAULTS.preRollSeconds
        )
      ),
      defaultSpeed: clampSpeed(
        typeof parsed.defaultSpeed === "number"
          ? parsed.defaultSpeed
          : DEFAULTS.defaultSpeed
      ),
      volumeMemory:
        parsed.volumeMemory === "session" ||
        parsed.volumeMemory === "vod" ||
        parsed.volumeMemory === "global"
          ? parsed.volumeMemory
          : DEFAULTS.volumeMemory,
      pipOnBlur:
        typeof parsed.pipOnBlur === "boolean"
          ? parsed.pipOnBlur
          : DEFAULTS.pipOnBlur,
    };
  } catch {
    return DEFAULTS;
  }
}

// External-store API so the Settings component re-renders when any
// tab updates prefs. The Settings tab isn't actually multi-window,
// but localStorage events cost almost nothing and keep the Settings
// UI in sync with a future per-player override surface.
type Listener = () => void;
const listeners = new Set<Listener>();

function emit() {
  for (const l of listeners) l();
}

if (typeof window !== "undefined") {
  window.addEventListener("storage", (e) => {
    if (e.key === STORAGE_KEY) emit();
  });
}

export function getPlaybackPrefs(): PlaybackPrefs {
  return readPrefs();
}

export function setPlaybackPrefs(update: Partial<PlaybackPrefs>): void {
  const next = { ...readPrefs(), ...update };
  try {
    window.localStorage.setItem(STORAGE_KEY, JSON.stringify(next));
    emit();
  } catch {
    /* ignore quota errors */
  }
}

export function usePlaybackPrefs(): PlaybackPrefs {
  return useSyncExternalStore(
    (listener) => {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    readPrefs,
    () => DEFAULTS
  );
}
