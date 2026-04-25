/**
 * Playback constants used across the player components. Kept in one
 * file so the Settings UI + help overlay + tests agree on what the
 * "step" for speed / volume is.
 */

export const PLAYBACK_SPEEDS = [
  0.5, 0.75, 1, 1.25, 1.5, 1.75, 2,
] as const;
export type PlaybackSpeed = (typeof PLAYBACK_SPEEDS)[number];

export const VOLUME_STEP = 0.1; // Arrow keys.
export const WHEEL_VOLUME_STEP = 0.05; // Scroll wheel.
export const SEEK_STEP_SMALL_SECONDS = 5;
export const SEEK_STEP_LARGE_SECONDS = 10;
export const FRAME_STEP_SECONDS = 1 / 30; // Assumes ~30 fps source.
export const CONTROLS_HIDE_AFTER_MS = 3000;
export const DEFAULT_PRE_ROLL_SECONDS = 5;
export const DEFAULT_COMPLETION_THRESHOLD = 0.9;
export const DEFAULT_AUTO_PLAY_ON_OPEN = true;
export const RESTART_THRESHOLD_SECONDS = 30;
export const PROGRESS_WRITE_INTERVAL_MS = 5000;

export const PLAYER_SHORTCUTS: ReadonlyArray<{
  keys: string;
  label: string;
}> = [
  { keys: "Space / K", label: "Play / pause" },
  { keys: "← / →", label: "Seek ±5 s" },
  { keys: "Shift + ← / →", label: "Frame step (pauses if playing)" },
  { keys: "J / L", label: "Seek ±10 s" },
  { keys: "↑ / ↓", label: "Volume ±10%" },
  { keys: "M", label: "Mute toggle" },
  { keys: "F", label: "Fullscreen" },
  { keys: "P", label: "Picture-in-picture" },
  { keys: "C / Shift+C", label: "Next / previous chapter" },
  { keys: "0 – 9", label: "Seek 0–90% of duration" },
  { keys: "< / >", label: "Speed down / up one step" },
  { keys: ", / .", label: "Frame step while paused" },
  { keys: "Esc", label: "Exit fullscreen / overlay" },
];

/**
 * Compute the initial `currentTime` for `<video>` when the player
 * mounts. Mirrors the Rust-side `domain::watch_progress::resume_position_for`
 * (which the backend uses to drive the watch_progress state machine);
 * factored out as a pure function so the JS pre-roll path is testable
 * without spinning up the player. Rules:
 *
 * 1. Explicit deep-link wins (`initialPositionSeconds`).
 * 2. Stored position within `RESTART_THRESHOLD_SECONDS` of the end →
 *    treat as completed; restart from zero.
 * 3. Otherwise `stored - preRoll`, clamped to `>= 0`.
 * 4. No stored position → start from zero.
 */
export function computeInitialSeekSeconds(input: {
  durationSeconds: number;
  storedPositionSeconds?: number;
  initialPositionSeconds?: number;
  preRollSeconds: number;
  restartThresholdSeconds: number;
}): number {
  const {
    durationSeconds,
    storedPositionSeconds,
    initialPositionSeconds,
    preRollSeconds,
    restartThresholdSeconds,
  } = input;
  if (!Number.isFinite(durationSeconds) || durationSeconds <= 0) return 0;
  if (typeof initialPositionSeconds === "number") {
    return Math.min(durationSeconds, Math.max(0, initialPositionSeconds));
  }
  if (typeof storedPositionSeconds === "number" && storedPositionSeconds > 0) {
    if (storedPositionSeconds >= durationSeconds - restartThresholdSeconds) {
      return 0;
    }
    return Math.max(0, storedPositionSeconds - preRollSeconds);
  }
  return 0;
}

/** Round a playback speed to the nearest supported step. */
export function clampSpeed(speed: number): PlaybackSpeed {
  let best: PlaybackSpeed = 1;
  let bestDistance = Number.POSITIVE_INFINITY;
  for (const s of PLAYBACK_SPEEDS) {
    const d = Math.abs(s - speed);
    if (d < bestDistance) {
      best = s;
      bestDistance = d;
    }
  }
  return best;
}

/** Next speed above the supplied value, or the max. */
export function speedStepUp(current: number): PlaybackSpeed {
  const idx = PLAYBACK_SPEEDS.findIndex((s) => s === clampSpeed(current));
  return PLAYBACK_SPEEDS[Math.min(PLAYBACK_SPEEDS.length - 1, idx + 1)]!;
}

export function speedStepDown(current: number): PlaybackSpeed {
  const idx = PLAYBACK_SPEEDS.findIndex((s) => s === clampSpeed(current));
  return PLAYBACK_SPEEDS[Math.max(0, idx - 1)]!;
}

/**
 * Format seconds as H:MM:SS / MM:SS.
 */
export function formatTime(seconds: number): string {
  if (!Number.isFinite(seconds) || seconds < 0) return "0:00";
  const total = Math.floor(seconds);
  const h = Math.floor(total / 3600);
  const m = Math.floor((total % 3600) / 60);
  const s = total % 60;
  const mm = m < 10 && h > 0 ? `0${m}` : `${m}`;
  const ss = s < 10 ? `0${s}` : `${s}`;
  return h > 0 ? `${h}:${mm}:${ss}` : `${mm}:${ss}`;
}
