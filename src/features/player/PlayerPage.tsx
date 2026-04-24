import { useQuery } from "@tanstack/react-query";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import { Button } from "@/components/primitives/Button";
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
  PROGRESS_WRITE_INTERVAL_MS,
  RESTART_THRESHOLD_SECONDS,
} from "./player-constants";
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
  autoplay = DEFAULT_AUTO_PLAY_ON_OPEN,
}: PlayerPageProps) {
  const videoRef = useRef<HTMLVideoElement | null>(null);
  const containerRef = useRef<HTMLDivElement | null>(null);
  const hasSeekedRef = useRef(false);
  const lastWriteRef = useRef(0);
  const closePlayer = useNavStore((s) => s.closePlayer);

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
  const [volume, setVolume] = useState(1);
  const [muted, setMuted] = useState(false);
  const [playbackRate, setPlaybackRate] = useState(1);
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

  // On first load: seek to the right position (pre-roll applied
  // backend-side when we call update; the explicit deep-link value
  // wins, otherwise we apply the stored position minus pre-roll).
  useEffect(() => {
    if (hasSeekedRef.current) return;
    const video = videoRef.current;
    if (!video || sourceState !== "ready") return;
    if (!Number.isFinite(video.duration) || video.duration === 0) return;

    const stored = progress.data?.positionSeconds;
    let target: number;
    if (typeof initialPositionSeconds === "number") {
      target = Math.min(video.duration, Math.max(0, initialPositionSeconds));
    } else if (typeof stored === "number" && stored > 0) {
      if (stored >= video.duration - RESTART_THRESHOLD_SECONDS) {
        target = 0;
      } else {
        target = Math.max(0, stored - DEFAULT_PRE_ROLL_SECONDS);
      }
    } else {
      target = 0;
    }
    video.currentTime = target;
    hasSeekedRef.current = true;
  }, [initialPositionSeconds, progress.data?.positionSeconds, sourceState]);

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
    const onBlur = () => flushProgress(true);
    window.addEventListener("blur", onBlur);
    return () => {
      window.removeEventListener("blur", onBlur);
      flushProgress(true);
    };
  }, [flushProgress]);

  // ---- Video element event handlers ----

  const handleLoadedMetadata = useCallback(() => {
    const video = videoRef.current;
    if (!video) return;
    setDurationSeconds(Number.isFinite(video.duration) ? video.duration : 0);
    video.volume = volume;
    video.muted = muted;
    video.playbackRate = playbackRate;
    if (autoplay) {
      void video.play().catch(() => {
        // Some browsers block autoplay without user gesture — the
        // user can just click the button.
      });
    }
  }, [autoplay, muted, playbackRate, volume]);

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

  const setVolumeClamped = useCallback((v: number) => {
    const clamped = Math.min(1, Math.max(0, v));
    setVolume(clamped);
    if (videoRef.current) videoRef.current.volume = clamped;
  }, []);

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
  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;
    const onKey = (e: KeyboardEvent) => {
      // Ignore keypresses inside inputs (chapter menu filter, future
      // search). The container's tabindex=-1 ensures focus can land
      // here without interfering with child inputs.
      const target = e.target as HTMLElement | null;
      if (
        target &&
        (target.tagName === "INPUT" ||
          target.tagName === "TEXTAREA" ||
          target.isContentEditable)
      ) {
        return;
      }

      // Frame step (bracketed by shift) takes priority over the
      // regular seek.
      if (e.shiftKey && (e.key === "ArrowLeft" || e.key === "ArrowRight")) {
        e.preventDefault();
        if (!videoRef.current) return;
        videoRef.current.pause();
        seekBy(e.key === "ArrowLeft" ? -1 / 30 : 1 / 30);
        return;
      }
      switch (e.key) {
        case " ":
        case "k":
        case "K":
          e.preventDefault();
          togglePlay();
          break;
        case "ArrowLeft":
          e.preventDefault();
          seekBy(-5);
          break;
        case "ArrowRight":
          e.preventDefault();
          seekBy(5);
          break;
        case "j":
        case "J":
          e.preventDefault();
          seekBy(-10);
          break;
        case "l":
        case "L":
          e.preventDefault();
          seekBy(10);
          break;
        case "ArrowUp":
          e.preventDefault();
          adjustVolume(0.1);
          break;
        case "ArrowDown":
          e.preventDefault();
          adjustVolume(-0.1);
          break;
        case "m":
        case "M":
          e.preventDefault();
          toggleMute();
          break;
        case "f":
        case "F":
          e.preventDefault();
          toggleFullscreen();
          break;
        case "p":
        case "P":
          e.preventDefault();
          togglePip();
          break;
        case "c":
          e.preventDefault();
          jumpToChapter(e.shiftKey ? "prev" : "next");
          break;
        case "C":
          e.preventDefault();
          jumpToChapter("prev");
          break;
        case ",":
          e.preventDefault();
          if (videoRef.current) videoRef.current.pause();
          seekBy(-1 / 30);
          break;
        case ".":
          e.preventDefault();
          if (videoRef.current) videoRef.current.pause();
          seekBy(1 / 30);
          break;
        case "<":
          e.preventDefault();
          setSpeed(Math.max(0.5, playbackRate - 0.25));
          break;
        case ">":
          e.preventDefault();
          setSpeed(Math.min(2, playbackRate + 0.25));
          break;
        case "Escape":
          if (document.fullscreenElement) {
            void document.exitFullscreen();
          } else {
            closePlayer();
          }
          break;
        case "0":
        case "1":
        case "2":
        case "3":
        case "4":
        case "5":
        case "6":
        case "7":
        case "8":
        case "9": {
          const digit = Number(e.key);
          e.preventDefault();
          seekToFraction(digit / 10);
          break;
        }
      }
    };
    container.addEventListener("keydown", onKey);
    container.focus();
    return () => container.removeEventListener("keydown", onKey);
  }, [
    adjustVolume,
    closePlayer,
    jumpToChapter,
    playbackRate,
    seekBy,
    seekToFraction,
    setSpeed,
    toggleFullscreen,
    toggleMute,
    togglePip,
    togglePlay,
  ]);

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
            completionThreshold={DEFAULT_COMPLETION_THRESHOLD}
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
