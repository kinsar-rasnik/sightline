import { PLAYBACK_SPEEDS } from "@/features/player/player-constants";

import {
  overlapDurationSeconds,
  overlapIsNonEmpty,
  type OverlapWindow,
} from "./sync-math";

export interface MultiViewTransportBarProps {
  isPlaying: boolean;
  onTogglePlay: () => void;
  /** Wall-clock window the slider operates over. */
  overlap: OverlapWindow;
  /** Current wall-clock moment of the leader pane. */
  leaderMomentUnixSeconds: number | null;
  onSeek: (wallClockTs: number) => void;
  playbackRate: number;
  onSetSpeed: (rate: number) => void;
  onClose: () => void;
}

/**
 * Group-wide transport for `/multiview` (ADR-0023). Sits at the
 * bottom of the page; one play/pause, one seek slider, one speed
 * selector — every action fans to all panes.
 */
export function MultiViewTransportBar({
  isPlaying,
  onTogglePlay,
  overlap,
  leaderMomentUnixSeconds,
  onSeek,
  playbackRate,
  onSetSpeed,
  onClose,
}: MultiViewTransportBarProps) {
  const duration = overlapDurationSeconds(overlap);
  const offset =
    leaderMomentUnixSeconds != null
      ? Math.max(0, Math.min(duration, leaderMomentUnixSeconds - overlap.startAt))
      : 0;

  const handleSliderChange = (raw: number) => {
    const clamped = Math.max(0, Math.min(duration, raw));
    onSeek(overlap.startAt + clamped);
  };

  return (
    <div
      role="toolbar"
      aria-label="Multi-view transport"
      className="flex items-center gap-3 bg-black/95 text-white px-4 py-3 border-t border-white/10"
    >
      <button
        type="button"
        onClick={onTogglePlay}
        aria-label={isPlaying ? "Pause group" : "Play group"}
        className="w-9 h-9 flex items-center justify-center rounded-full bg-white/10 hover:bg-white/20 focus-visible:outline focus-visible:outline-2 focus-visible:outline-[--color-accent]"
      >
        {isPlaying ? "⏸" : "▶"}
      </button>

      <div className="flex-1 flex items-center gap-2">
        <span className="text-xs font-mono text-white/70 tabular-nums w-14 text-right">
          {formatTime(offset)}
        </span>
        <input
          type="range"
          min={0}
          max={Math.max(0, duration)}
          step={1}
          value={offset}
          aria-label="Wall-clock seek"
          aria-valuenow={Math.floor(offset)}
          aria-valuemin={0}
          aria-valuemax={Math.max(0, Math.floor(duration))}
          aria-valuetext={`${formatTime(offset)} of ${formatTime(duration)}`}
          onChange={(e) => handleSliderChange(Number(e.target.value))}
          disabled={!overlapIsNonEmpty(overlap)}
          className="flex-1 accent-[--color-accent]"
        />
        <span className="text-xs font-mono text-white/70 tabular-nums w-14">
          {formatTime(duration)}
        </span>
      </div>

      <label className="flex items-center gap-2 text-xs text-white/80">
        <span className="sr-only">Playback speed</span>
        <select
          value={playbackRate}
          onChange={(e) => onSetSpeed(Number(e.target.value))}
          aria-label="Playback speed"
          className="bg-black/60 border border-white/10 rounded px-2 py-1 text-xs"
        >
          {PLAYBACK_SPEEDS.map((s) => (
            <option key={s} value={s}>
              {s}×
            </option>
          ))}
        </select>
      </label>

      <button
        type="button"
        onClick={onClose}
        aria-label="Close multi-view"
        className="text-xs font-medium px-3 py-1.5 rounded bg-white/10 hover:bg-white/20 focus-visible:outline focus-visible:outline-2 focus-visible:outline-[--color-accent]"
      >
        Close
      </button>
    </div>
  );
}

function formatTime(seconds: number): string {
  if (!Number.isFinite(seconds) || seconds < 0) return "0:00";
  const total = Math.floor(seconds);
  const h = Math.floor(total / 3600);
  const m = Math.floor((total % 3600) / 60);
  const s = total % 60;
  const mm = m < 10 && h > 0 ? `0${m}` : `${m}`;
  const ss = s < 10 ? `0${s}` : `${s}`;
  return h > 0 ? `${h}:${mm}:${ss}` : `${mm}:${ss}`;
}
