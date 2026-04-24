import { useEffect, useRef, useState } from "react";

import type { Chapter, WatchState } from "@/ipc";

import { CONTROLS_HIDE_AFTER_MS, PLAYBACK_SPEEDS, formatTime } from "./player-constants";

export interface PlayerChromeProps {
  title: string;
  isPlaying: boolean;
  currentSeconds: number;
  durationSeconds: number;
  chapters: Chapter[];
  volume: number;
  muted: boolean;
  playbackRate: number;
  showRemaining: boolean;
  isFullscreen: boolean;
  pipActive: boolean;
  completionThreshold: number;
  resumeSeconds: number | null;
  watchedState: WatchState;
  onTogglePlay: () => void;
  onSeekTo: (seconds: number) => void;
  onSeekBy: (delta: number) => void;
  onToggleRemaining: () => void;
  onVolumeChange: (v: number) => void;
  onToggleMute: () => void;
  onSetSpeed: (rate: number) => void;
  onToggleFullscreen: () => void;
  onTogglePip: () => void;
  onJumpChapter: (dir: "next" | "prev") => void;
  onMarkWatched: () => void;
  onMarkUnwatched: () => void;
}

/**
 * Custom controls overlay. Renders above the `<video>` element.
 * Auto-hides after 3 s of mouse inactivity while playing; re-appears
 * on mousemove or keyboard input. Controls remain visible while
 * paused so users don't get trapped in a frozen frame.
 */
export function PlayerChrome(props: PlayerChromeProps) {
  const {
    title,
    isPlaying,
    currentSeconds,
    durationSeconds,
    chapters,
    volume,
    muted,
    playbackRate,
    showRemaining,
    isFullscreen,
    pipActive,
    resumeSeconds,
    watchedState,
    onTogglePlay,
    onSeekTo,
    onToggleRemaining,
    onVolumeChange,
    onToggleMute,
    onSetSpeed,
    onToggleFullscreen,
    onTogglePip,
    onJumpChapter,
    onMarkWatched,
    onMarkUnwatched,
  } = props;

  const [visible, setVisible] = useState(true);
  const [chapterMenuOpen, setChapterMenuOpen] = useState(false);
  const [settingsMenuOpen, setSettingsMenuOpen] = useState(false);
  const hideTimerRef = useRef<number | null>(null);

  const scheduleHide = () => {
    if (hideTimerRef.current) window.clearTimeout(hideTimerRef.current);
    hideTimerRef.current = window.setTimeout(() => {
      // Don't hide while paused or a menu is open.
      if (!isPlaying || chapterMenuOpen || settingsMenuOpen) return;
      setVisible(false);
    }, CONTROLS_HIDE_AFTER_MS);
  };

  useEffect(() => {
    if (!isPlaying) {
      setVisible(true);
      if (hideTimerRef.current) window.clearTimeout(hideTimerRef.current);
      return;
    }
    scheduleHide();
    return () => {
      if (hideTimerRef.current) window.clearTimeout(hideTimerRef.current);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [isPlaying, chapterMenuOpen, settingsMenuOpen]);

  // Surface events on the parent video element's layer reach us via
  // the overlay's pointer listeners (it's `pointer-events:none` on
  // the underlay, `pointer-events:auto` on controls).
  const handleMouseMove = () => {
    setVisible(true);
    scheduleHide();
  };

  const fraction =
    durationSeconds > 0
      ? Math.min(1, Math.max(0, currentSeconds / durationSeconds))
      : 0;

  const resumeFraction =
    resumeSeconds != null && durationSeconds > 0
      ? Math.min(1, Math.max(0, resumeSeconds / durationSeconds))
      : null;

  const rightText = showRemaining
    ? `-${formatTime(Math.max(0, durationSeconds - currentSeconds))}`
    : formatTime(durationSeconds);

  return (
    <div
      aria-label="Player controls"
      className={`pointer-events-none absolute inset-0 flex flex-col justify-between transition-opacity duration-[var(--motion-fast)] ${
        visible ? "opacity-100" : "opacity-0"
      }`}
      onMouseMove={handleMouseMove}
    >
      <div className="pointer-events-auto px-6 pt-4">
        <p className="text-sm text-white/80 font-medium truncate drop-shadow">
          {title}
        </p>
      </div>

      <div
        className="pointer-events-auto px-6 pb-4 flex flex-col gap-2 bg-gradient-to-t from-black/90 to-transparent"
        role="toolbar"
        aria-label="Playback controls"
      >
        <SeekBar
          currentSeconds={currentSeconds}
          durationSeconds={durationSeconds}
          chapters={chapters}
          resumeFraction={resumeFraction}
          fraction={fraction}
          onSeekTo={onSeekTo}
        />

        <div className="flex items-center gap-3 text-white">
          <IconButton
            label={isPlaying ? "Pause" : "Play"}
            onClick={onTogglePlay}
            icon={isPlaying ? "⏸" : "▶"}
          />
          <IconButton
            label="Previous chapter"
            onClick={() => onJumpChapter("prev")}
            icon="⏮"
          />
          <IconButton
            label="Next chapter"
            onClick={() => onJumpChapter("next")}
            icon="⏭"
          />

          <div className="flex items-center gap-2 min-w-[140px]">
            <IconButton
              label={muted ? "Unmute" : "Mute"}
              onClick={onToggleMute}
              icon={muted || volume === 0 ? "🔇" : volume < 0.5 ? "🔉" : "🔊"}
            />
            <input
              type="range"
              min={0}
              max={1}
              step={0.01}
              value={muted ? 0 : volume}
              aria-label="Volume"
              aria-valuenow={Math.round((muted ? 0 : volume) * 100)}
              aria-valuemin={0}
              aria-valuemax={100}
              onChange={(e) => onVolumeChange(Number(e.target.value))}
              className="w-20 accent-[--color-accent]"
            />
          </div>

          <button
            type="button"
            onClick={onToggleRemaining}
            aria-label="Toggle elapsed / remaining time"
            className="text-xs font-mono tabular-nums text-white/80 hover:text-white focus-visible:outline focus-visible:outline-2 focus-visible:outline-[--color-accent]"
          >
            {formatTime(currentSeconds)} / {rightText}
          </button>

          <div className="ml-auto flex items-center gap-1">
            <SpeedMenu
              playbackRate={playbackRate}
              onSetSpeed={onSetSpeed}
              onOpenChange={setSettingsMenuOpen}
            />
            <ChapterMenu
              chapters={chapters}
              currentSeconds={currentSeconds}
              onOpenChange={setChapterMenuOpen}
              onJumpTo={(ms) => onSeekTo(ms / 1000)}
            />
            <IconButton
              label={pipActive ? "Exit picture-in-picture" : "Picture-in-picture"}
              onClick={onTogglePip}
              icon="⧉"
              aria-pressed={pipActive}
            />
            <IconButton
              label={isFullscreen ? "Exit fullscreen" : "Fullscreen"}
              onClick={onToggleFullscreen}
              icon={isFullscreen ? "⛶" : "⤢"}
              aria-pressed={isFullscreen}
            />
            <WatchStateMenu
              state={watchedState}
              onMarkWatched={onMarkWatched}
              onMarkUnwatched={onMarkUnwatched}
              onOpenChange={setSettingsMenuOpen}
            />
          </div>
        </div>
      </div>
    </div>
  );
}

function SeekBar({
  currentSeconds,
  durationSeconds,
  chapters,
  resumeFraction,
  fraction,
  onSeekTo,
}: {
  currentSeconds: number;
  durationSeconds: number;
  chapters: Chapter[];
  resumeFraction: number | null;
  fraction: number;
  onSeekTo: (seconds: number) => void;
}) {
  return (
    <div
      role="slider"
      aria-label="Seek"
      aria-valuemin={0}
      aria-valuemax={Math.floor(durationSeconds)}
      aria-valuenow={Math.floor(currentSeconds)}
      aria-valuetext={`${formatTime(currentSeconds)} of ${formatTime(durationSeconds)}`}
      tabIndex={0}
      onClick={(e) => {
        const bounds = (e.target as HTMLElement).getBoundingClientRect();
        const ratio = Math.min(1, Math.max(0, (e.clientX - bounds.left) / bounds.width));
        onSeekTo(ratio * durationSeconds);
      }}
      onKeyDown={(e) => {
        if (e.key === "Home") onSeekTo(0);
        if (e.key === "End") onSeekTo(durationSeconds);
      }}
      className="relative h-2 bg-white/20 rounded cursor-pointer focus-visible:outline focus-visible:outline-2 focus-visible:outline-[--color-accent]"
    >
      <div
        aria-hidden="true"
        className="absolute inset-y-0 left-0 bg-[--color-accent] rounded"
        style={{ width: `${fraction * 100}%` }}
      />
      {resumeFraction != null && (
        <div
          aria-hidden="true"
          title="Resume point"
          className="absolute top-0 bottom-0 w-0.5 bg-white/60"
          style={{ left: `calc(${resumeFraction * 100}% - 1px)` }}
        />
      )}
      {chapters.map((c, i) => {
        const cFraction =
          durationSeconds > 0 ? Math.min(1, c.positionMs / 1000 / durationSeconds) : 0;
        const isGta5 = c.gameId === "32982";
        return (
          <span
            key={`${c.positionMs}-${i}`}
            aria-hidden="true"
            title={`${c.gameName || "Chapter"} @ ${formatTime(c.positionMs / 1000)}`}
            className={`absolute top-0 bottom-0 w-0.5 ${
              isGta5 ? "bg-amber-300" : "bg-white/70"
            }`}
            style={{ left: `calc(${cFraction * 100}% - 1px)` }}
          />
        );
      })}
      <div
        aria-hidden="true"
        className="absolute top-1/2 -translate-y-1/2 h-3 w-3 rounded-full bg-[--color-accent] shadow"
        style={{ left: `calc(${fraction * 100}% - 6px)` }}
      />
    </div>
  );
}

function IconButton({
  label,
  icon,
  onClick,
  "aria-pressed": ariaPressed,
}: {
  label: string;
  icon: string;
  onClick: () => void;
  "aria-pressed"?: boolean;
}) {
  return (
    <button
      type="button"
      aria-label={label}
      aria-pressed={ariaPressed}
      onClick={onClick}
      className="w-8 h-8 flex items-center justify-center rounded text-white/90 hover:text-white hover:bg-white/10 focus-visible:outline focus-visible:outline-2 focus-visible:outline-[--color-accent]"
    >
      {icon}
    </button>
  );
}

function SpeedMenu({
  playbackRate,
  onSetSpeed,
  onOpenChange,
}: {
  playbackRate: number;
  onSetSpeed: (r: number) => void;
  onOpenChange: (open: boolean) => void;
}) {
  const [open, setOpen] = useState(false);
  useEffect(() => onOpenChange(open), [onOpenChange, open]);
  return (
    <div className="relative">
      <button
        type="button"
        aria-haspopup="menu"
        aria-expanded={open}
        aria-label={`Playback speed — currently ${playbackRate}×`}
        onClick={() => setOpen((o) => !o)}
        className="h-8 px-2 text-xs text-white/90 hover:text-white hover:bg-white/10 rounded focus-visible:outline focus-visible:outline-2 focus-visible:outline-[--color-accent]"
      >
        {playbackRate}×
      </button>
      {open && (
        <ul
          role="menu"
          className="absolute bottom-full mb-2 right-0 z-10 min-w-[80px] bg-black/90 border border-white/10 rounded text-xs"
        >
          {PLAYBACK_SPEEDS.map((s) => (
            <li key={s} role="none">
              <button
                role="menuitemradio"
                aria-checked={Math.abs(s - playbackRate) < 0.01}
                onClick={() => {
                  onSetSpeed(s);
                  setOpen(false);
                }}
                className={`w-full text-left px-3 py-1 hover:bg-white/10 focus-visible:outline focus-visible:outline-2 focus-visible:outline-[--color-accent] ${
                  Math.abs(s - playbackRate) < 0.01
                    ? "text-[--color-accent]"
                    : "text-white/80"
                }`}
              >
                {s}×
              </button>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}

function ChapterMenu({
  chapters,
  currentSeconds,
  onJumpTo,
  onOpenChange,
}: {
  chapters: Chapter[];
  currentSeconds: number;
  onJumpTo: (positionMs: number) => void;
  onOpenChange: (open: boolean) => void;
}) {
  const [open, setOpen] = useState(false);
  useEffect(() => onOpenChange(open), [onOpenChange, open]);
  return (
    <div className="relative">
      <button
        type="button"
        aria-haspopup="menu"
        aria-expanded={open}
        aria-label="Chapters"
        onClick={() => setOpen((o) => !o)}
        className="w-8 h-8 flex items-center justify-center rounded text-white/90 hover:text-white hover:bg-white/10 focus-visible:outline focus-visible:outline-2 focus-visible:outline-[--color-accent]"
      >
        ☰
      </button>
      {open && (
        <ul
          role="menu"
          className="absolute bottom-full mb-2 right-0 z-10 min-w-[220px] max-h-64 overflow-y-auto bg-black/90 border border-white/10 rounded text-xs"
        >
          {chapters.length === 0 && (
            <li role="none" className="px-3 py-2 text-white/60">
              No chapters
            </li>
          )}
          {chapters.map((c, i) => {
            const positionSeconds = c.positionMs / 1000;
            const isCurrent = Math.abs(positionSeconds - currentSeconds) < 10;
            const isGta5 = c.gameId === "32982";
            return (
              <li key={`${c.positionMs}-${i}`} role="none">
                <button
                  role="menuitem"
                  onClick={() => {
                    onJumpTo(c.positionMs);
                    setOpen(false);
                  }}
                  className={`w-full text-left px-3 py-1 flex justify-between gap-3 hover:bg-white/10 focus-visible:outline focus-visible:outline-2 focus-visible:outline-[--color-accent] ${
                    isCurrent ? "text-[--color-accent]" : "text-white/80"
                  }`}
                >
                  <span className={isGta5 ? "font-semibold" : ""}>
                    {c.gameName || "Unknown game"}
                  </span>
                  <span className="font-mono text-white/60">
                    {formatTime(positionSeconds)}
                  </span>
                </button>
              </li>
            );
          })}
        </ul>
      )}
    </div>
  );
}

function WatchStateMenu({
  state,
  onMarkWatched,
  onMarkUnwatched,
  onOpenChange,
}: {
  state: WatchState;
  onMarkWatched: () => void;
  onMarkUnwatched: () => void;
  onOpenChange: (open: boolean) => void;
}) {
  const [open, setOpen] = useState(false);
  useEffect(() => onOpenChange(open), [onOpenChange, open]);
  const watched = state === "completed" || state === "manually_watched";
  return (
    <div className="relative">
      <button
        type="button"
        aria-haspopup="menu"
        aria-expanded={open}
        aria-label="More actions"
        onClick={() => setOpen((o) => !o)}
        className="w-8 h-8 flex items-center justify-center rounded text-white/90 hover:text-white hover:bg-white/10 focus-visible:outline focus-visible:outline-2 focus-visible:outline-[--color-accent]"
      >
        ⋯
      </button>
      {open && (
        <ul
          role="menu"
          className="absolute bottom-full mb-2 right-0 z-10 min-w-[180px] bg-black/90 border border-white/10 rounded text-xs"
        >
          <li role="none">
            <button
              role="menuitem"
              onClick={() => {
                if (watched) {
                  onMarkUnwatched();
                } else {
                  onMarkWatched();
                }
                setOpen(false);
              }}
              className="w-full text-left px-3 py-1.5 hover:bg-white/10 focus-visible:outline focus-visible:outline-2 focus-visible:outline-[--color-accent] text-white/80"
            >
              {watched ? "Mark as unwatched" : "Mark as watched"}
            </button>
          </li>
        </ul>
      )}
    </div>
  );
}
