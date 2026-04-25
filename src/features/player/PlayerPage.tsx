import { useQuery } from "@tanstack/react-query";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import { Button } from "@/components/primitives/Button";
import { useSettings } from "@/features/settings/use-settings";
import {
  commands,
  type Chapter,
  type VideoSourceState,
  type VodWithChapters,
} from "@/ipc";
import { assetUrl } from "@/lib/asset-url";
import { useNavStore } from "@/stores/nav-store";

import { PlayerChrome } from "./PlayerChrome";
import {
  DEFAULT_AUTO_PLAY_ON_OPEN,
  DEFAULT_COMPLETION_THRESHOLD,
  DEFAULT_PRE_ROLL_SECONDS,
  FRAME_STEP_SECONDS,
  PROGRESS_WRITE_INTERVAL_MS,
  RESTART_THRESHOLD_SECONDS,
  computeInitialSeekSeconds,
  speedStepDown,
  speedStepUp,
} from "./player-constants";
import { usePlaybackPrefs } from "./use-playback-prefs";
import {
  DEFAULT_PLAYER_SHORTCUTS,
  usePlayerShortcuts,
} from "./use-player-shortcuts";
import { readVolume, writeVolume } from "./volume-memory";
import {
  useMarkUnwatched,
  useMarkWatched,
  useUpdateWatchProgress,
  useWatchProgress,
} from "./use-watch-progress";

export interface PlayerPageProps {
  vodId: string;
  /** Initial seek (used by the cross-streamer deep link). */
  initialPositionSeconds?: number;
  /** Whether to start playback on mount. */
  autoplay?: boolean;
}

/**
 * `/watch/:vodId` route. Owns the `<video>` element, fetches the
 * signed asset URL, renders the custom controls overlay, and
 * persists watch-progress on a debounced timer.
 *
 * The video element is kept in a ref so the imperative API calls
 * (play, pause, seek, volume) don't require a re-render cycle each
 * time; the state updates flow through `onTimeUpdate` /
 * `onLoadedMetadata` listeners into React state that the overlay
 * reads via `PlayerChrome`.
 */
export function PlayerPage({
  vodId,
  initialPositionSeconds,
  autoplay,
}: PlayerPageProps) {
  const videoRef = useRef<HTMLVideoElement | null>(null);
  const containerRef = useRef<HTMLDivElement | null>(null);
  const hasSeekedRef = useRef(false);
  const lastWriteRef = useRef(0);
  const closePlayer = useNavStore((s) => s.closePlayer);
  const prefs = usePlaybackPrefs();
  // Source of truth for the completion threshold lives in
  // `app_settings.completion_threshold` (migration 0009) so the
  // backend state machine + this overlay agree. Falling back to the
  // compiled-in default keeps the chrome stable while the settings
  // query is still loading.
  const settings = useSettings();
  const completionThreshold =
    settings.data?.completionThreshold ?? DEFAULT_COMPLETION_THRESHOLD;
  // If the caller explicitly passed `autoplay`, honour that (deep link
  // scenario); otherwise fall back to the user preference.
  const effectiveAutoplay =
    typeof autoplay === "boolean" ? autoplay : prefs.autoplay ?? DEFAULT_AUTO_PLAY_ON_OPEN;

  const source = useQuery({
    queryKey: ["video-source", vodId],
    queryFn: () => commands.getVideoSource({ vodId }),
    staleTime: 3_000,
  });

  const vod = useQuery<VodWithChapters>({
    queryKey: ["vod", vodId],
    queryFn: () => commands.getVod({ twitchVideoId: vodId }),
    staleTime: 60_000,
  });

  const progress = useWatchProgress(vodId);
  const updateProgress = useUpdateWatchProgress();
  const markWatched = useMarkWatched();
  const markUnwatched = useMarkUnwatched();

  // ---- Local, imperative-ish player state ----
  const [isPlaying, setIsPlaying] = useState(false);
  const [currentSeconds, setCurrentSeconds] = useState(0);
  const [durationSeconds, setDurationSeconds] = useState(0);
  // Initial volume honours the Settings → Volume memory policy; per-VOD
  // memory falls back to the global key so a fresh VOD doesn't override
  // a softer setting the user has been on. See volume-memory.ts.
  const [volume, setVolume] = useState(() =>
    readVolume(prefs.volumeMemory, vodId)
  );
  const [muted, setMuted] = useState(false);
  const [playbackRate, setPlaybackRate] = useState<number>(prefs.defaultSpeed);
  const [showRemaining, setShowRemaining] = useState(false);
  const [isFullscreen, setIsFullscreen] = useState(false);
  const [pipActive, setPipActive] = useState(false);
  const [errorState, setErrorState] = useState<
    "none" | "decode" | "missing"
  >("none");

  const chapters = useMemo(() => vod.data?.chapters ?? [], [vod.data?.chapters]);
  const sourceState: VideoSourceState | "loading" | "error" =
    source.isLoading
      ? "loading"
      : source.isError
        ? "error"
        : (source.data?.state ?? "missing");
  const resolvedUrl = source.data?.path ? assetUrl(source.data.path) : "";

  // On first load: seek to the right position. Seek math lives in
  // `computeInitialSeekSeconds` so it's testable without mounting the
  // player; this effect just runs the calculation and applies the
  // result imperatively.
  useEffect(() => {
    if (hasSeekedRef.current) return;
    const video = videoRef.current;
    if (!video || sourceState !== "ready") return;
    if (!Number.isFinite(video.duration) || video.duration === 0) return;

    const target = computeInitialSeekSeconds({
      durationSeconds: video.duration,
      storedPositionSeconds: progress.data?.positionSeconds,
      initialPositionSeconds,
      preRollSeconds:
        typeof prefs.preRollSeconds === "number"
          ? prefs.preRollSeconds
          : DEFAULT_PRE_ROLL_SECONDS,
      restartThresholdSeconds: RESTART_THRESHOLD_SECONDS,
    });
    video.currentTime = target;
    hasSeekedRef.current = true;
  }, [
    initialPositionSeconds,
    prefs.preRollSeconds,
    progress.data?.positionSeconds,
    sourceState,
  ]);

  // Debounced DB write — fires every PROGRESS_WRITE_INTERVAL_MS wall
  // clock, on pause, on tab blur, and on unmount. Mutation is
  // best-effort; a network hiccup just delays the write.
  const flushProgress = useCallback(
    (force = false) => {
      const video = videoRef.current;
      if (!video || !Number.isFinite(video.duration)) return;
      const now = Date.now();
      if (!force && now - lastWriteRef.current < PROGRESS_WRITE_INTERVAL_MS)
        return;
      lastWriteRef.current = now;
      updateProgress.mutate({
        vodId,
        positionSeconds: video.currentTime,
        durationSeconds: video.duration,
      });
    },
    [updateProgress, vodId]
  );

  useEffect(() => {
    const onBlur = () => {
      flushProgress(true);
      // Optional: enter Picture-in-Picture when the window loses
      // focus (Settings → Playback → PiP-on-blur). Best-effort —
      // some browsers require a prior user gesture, in which case
      // the call silently rejects.
      if (
        prefs.pipOnBlur &&
        videoRef.current &&
        !videoRef.current.paused &&
        "requestPictureInPicture" in videoRef.current
      ) {
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        void (videoRef.current as any).requestPictureInPicture?.().catch(() => {
          /* ignore */
        });
      }
    };
    window.addEventListener("blur", onBlur);
    return () => {
      window.removeEventListener("blur", onBlur);
      flushProgress(true);
    };
  }, [flushProgress, prefs.pipOnBlur]);

  // ---- Video element event handlers ----

  const handleLoadedMetadata = useCallback(() => {
    const video = videoRef.current;
    if (!video) return;
    setDurationSeconds(Number.isFinite(video.duration) ? video.duration : 0);
    video.volume = volume;
    video.muted = muted;
    video.playbackRate = playbackRate;
    if (effectiveAutoplay) {
      void video.play().catch(() => {
        // Some browsers block autoplay without user gesture — the
        // user can just click the button.
      });
    }
  }, [effectiveAutoplay, muted, playbackRate, volume]);

  const handleTimeUpdate = useCallback(() => {
    const video = videoRef.current;
    if (!video) return;
    setCurrentSeconds(video.currentTime);
    flushProgress();
  }, [flushProgress]);

  const handlePause = useCallback(() => {
    setIsPlaying(false);
    flushProgress(true);
  }, [flushProgress]);

  const handlePlay = useCallback(() => setIsPlaying(true), []);

  const handleError = useCallback(() => {
    setErrorState("decode");
  }, []);

  // ---- Imperative action helpers (used by controls + shortcuts) ----

  const togglePlay = useCallback(() => {
    const v = videoRef.current;
    if (!v) return;
    if (v.paused) void v.play();
    else v.pause();
  }, []);

  const seekBy = useCallback((delta: number) => {
    const v = videoRef.current;
    if (!v || !Number.isFinite(v.duration)) return;
    v.currentTime = Math.min(v.duration, Math.max(0, v.currentTime + delta));
  }, []);

  const seekToFraction = useCallback((fraction: number) => {
    const v = videoRef.current;
    if (!v || !Number.isFinite(v.duration)) return;
    v.currentTime = Math.min(v.duration, Math.max(0, v.duration * fraction));
  }, []);

  const setVolumeClamped = useCallback(
    (v: number) => {
      const clamped = Math.min(1, Math.max(0, v));
      setVolume(clamped);
      if (videoRef.current) videoRef.current.volume = clamped;
      writeVolume(prefs.volumeMemory, vodId, clamped);
    },
    [prefs.volumeMemory, vodId]
  );

  const adjustVolume = useCallback(
    (delta: number) => setVolumeClamped(volume + delta),
    [setVolumeClamped, volume]
  );

  const toggleMute = useCallback(() => {
    setMuted((m) => {
      const next = !m;
      if (videoRef.current) videoRef.current.muted = next;
      return next;
    });
  }, []);

  const setSpeed = useCallback((rate: number) => {
    setPlaybackRate(rate);
    if (videoRef.current) videoRef.current.playbackRate = rate;
  }, []);

  const toggleFullscreen = useCallback(() => {
    const container = containerRef.current;
    if (!container) return;
    if (document.fullscreenElement) {
      void document.exitFullscreen();
    } else {
      void container.requestFullscreen?.();
    }
  }, []);

  useEffect(() => {
    const onFs = () => setIsFullscreen(document.fullscreenElement !== null);
    document.addEventListener("fullscreenchange", onFs);
    return () => document.removeEventListener("fullscreenchange", onFs);
  }, []);

  const togglePip = useCallback(() => {
    const v = videoRef.current;
    if (!v) return;
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const doc = document as any;
    if (doc.pictureInPictureElement) {
      void doc.exitPictureInPicture?.();
      setPipActive(false);
    } else if ("requestPictureInPicture" in v) {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      void (v as any).requestPictureInPicture?.().then(() => setPipActive(true));
    }
  }, []);

  const jumpToChapter = useCallback(
    (direction: "next" | "prev") => {
      const v = videoRef.current;
      if (!v || chapters.length === 0) return;
      const current = v.currentTime;
      const sorted = [...chapters].sort((a, b) => a.positionMs - b.positionMs);
      if (direction === "next") {
        const next = sorted.find((c) => c.positionMs / 1000 > current + 0.5);
        if (next) v.currentTime = next.positionMs / 1000;
      } else {
        const rev = [...sorted].reverse();
        const prev = rev.find((c) => c.positionMs / 1000 < current - 0.5);
        v.currentTime = prev ? prev.positionMs / 1000 : 0;
      }
    },
    [chapters]
  );

  // Mark-as-watched helper.
  const handleMarkWatched = useCallback(() => {
    markWatched.mutate(vodId);
  }, [markWatched, vodId]);

  const handleMarkUnwatched = useCallback(() => {
    markUnwatched.mutate(vodId);
  }, [markUnwatched, vodId]);

  // ---- Keyboard shortcuts ----
  // Phase 6: dispatched via the shortcuts service so the customisation
  // UI lands player keys on the same machinery as nav. The hook is
  // container-scoped (not window-scoped like `useShortcuts`) so player
  // actions only fire when the player has focus.
  const frameStep = useCallback(
    (direction: "back" | "forward") => {
      if (videoRef.current) videoRef.current.pause();
      seekBy(direction === "back" ? -FRAME_STEP_SECONDS : FRAME_STEP_SECONDS);
    },
    [seekBy]
  );
  const stepSpeed = useCallback(
    (direction: "up" | "down") => {
      setSpeed(
        direction === "up"
          ? speedStepUp(playbackRate)
          : speedStepDown(playbackRate)
      );
    },
    [playbackRate, setSpeed]
  );
  const closeAction = useCallback(() => {
    if (document.fullscreenElement) {
      void document.exitFullscreen();
    } else {
      closePlayer();
    }
  }, [closePlayer]);

  usePlayerShortcuts(containerRef, DEFAULT_PLAYER_SHORTCUTS, {
    playPause: togglePlay,
    seekBy,
    adjustVolume,
    toggleMute,
    toggleFullscreen,
    togglePip,
    jumpChapter: jumpToChapter,
    stepSpeed,
    close: closeAction,
    frameStep,
    seekToFraction,
  });

  // Focus the container on mount so the keyboard hook sees keystrokes
  // even before the user has clicked the video element.
  useEffect(() => {
    containerRef.current?.focus();
  }, []);

  const videoTitle = vod.data?.vod.title ?? "Player";
  const sortedChapters = useMemo<Chapter[]>(
    () => [...chapters].sort((a, b) => a.positionMs - b.positionMs),
    [chapters]
  );

  return (
    <div
      ref={containerRef}
      className="relative h-full flex flex-col bg-black outline-none"
      role="region"
      aria-label={`Player — ${videoTitle}`}
      tabIndex={-1}
    >
      <button
        type="button"
        onClick={() => closePlayer()}
        className="absolute top-3 left-3 z-20 text-xs text-white/80 bg-black/60 rounded-full px-3 py-1 hover:bg-black/80 focus-visible:outline focus-visible:outline-2 focus-visible:outline-[--color-accent]"
      >
        ← Back to library
      </button>

      {sourceState === "loading" && (
        <div className="flex-1 flex items-center justify-center text-white/70 text-sm">
          Loading video…
        </div>
      )}

      {sourceState === "error" && (
        <div className="flex-1 flex items-center justify-center text-white/70 text-sm">
          Could not resolve video source — is the library root configured?
        </div>
      )}

      {sourceState === "missing" && (
        <div
          className="flex-1 flex flex-col items-center justify-center gap-3 text-white/80"
          role="alert"
        >
          <p className="text-base">File not found</p>
          <p className="text-xs max-w-sm text-center">
            Sightline can&apos;t find this VOD on disk. It may have been
            moved or deleted outside the app.
          </p>
          <Button
            variant="primary"
            onClick={() => commands.enqueueDownload({ vodId })}
          >
            Re-download
          </Button>
        </div>
      )}

      {sourceState === "partial" && (
        <div className="flex-1 flex items-center justify-center text-white/70 text-sm">
          This VOD is still downloading. Come back when it&apos;s ready.
        </div>
      )}

      {sourceState === "ready" && errorState === "decode" && (
        <div
          className="flex-1 flex flex-col items-center justify-center gap-3 text-white/80"
          role="alert"
        >
          <p className="text-base">Can&apos;t play this file</p>
          <p className="text-xs max-w-sm text-center">
            The browser&apos;s video pipeline rejected this file. Try
            remuxing it — Sightline will run ffmpeg to produce a
            compatible MP4.
          </p>
          <Button
            variant="primary"
            onClick={() => {
              setErrorState("none");
              void commands.requestRemux({ vodId });
            }}
          >
            Remux file
          </Button>
        </div>
      )}

      {sourceState === "ready" && errorState === "none" && (
        <>
          {/* Captions / subtitles intentionally deferred to Phase 7.
              See docs/a11y-exceptions.md for the scope note on
              `jsx-a11y/media-has-caption`. */}
          {/* eslint-disable-next-line jsx-a11y/media-has-caption */}
          <video
            ref={videoRef}
            src={resolvedUrl}
            aria-label={videoTitle}
            preload="metadata"
            className="block w-full h-full bg-black object-contain"
            onLoadedMetadata={handleLoadedMetadata}
            onTimeUpdate={handleTimeUpdate}
            onPause={handlePause}
            onPlay={handlePlay}
            onError={handleError}
            onClick={togglePlay}
            onDoubleClick={toggleFullscreen}
            onWheel={(e) => {
              e.preventDefault();
              adjustVolume(e.deltaY < 0 ? 0.05 : -0.05);
            }}
            // We draw our own controls overlay; the native ones are
            // suppressed so there isn't a double keyboard path.
            controls={false}
          />

          <PlayerChrome
            title={videoTitle}
            isPlaying={isPlaying}
            currentSeconds={currentSeconds}
            durationSeconds={durationSeconds}
            chapters={sortedChapters}
            volume={volume}
            muted={muted}
            playbackRate={playbackRate}
            showRemaining={showRemaining}
            isFullscreen={isFullscreen}
            pipActive={pipActive}
            completionThreshold={completionThreshold}
            resumeSeconds={progress.data?.positionSeconds ?? null}
            onTogglePlay={togglePlay}
            onSeekTo={(seconds) => {
              const v = videoRef.current;
              if (v) v.currentTime = seconds;
            }}
            onSeekBy={seekBy}
            onToggleRemaining={() => setShowRemaining((v) => !v)}
            onVolumeChange={setVolumeClamped}
            onToggleMute={toggleMute}
            onSetSpeed={setSpeed}
            onToggleFullscreen={toggleFullscreen}
            onTogglePip={togglePip}
            onJumpChapter={jumpToChapter}
            onMarkWatched={handleMarkWatched}
            onMarkUnwatched={handleMarkUnwatched}
            watchedState={progress.data?.state ?? "unwatched"}
          />
        </>
      )}
    </div>
  );
}
