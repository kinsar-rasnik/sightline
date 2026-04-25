import { useEffect, useRef } from "react";
import { useQuery } from "@tanstack/react-query";

import { commands, type VideoSource } from "@/ipc";
import { assetUrl } from "@/lib/asset-url";

import type { PaneRuntime } from "./sync-store";

export interface MultiViewPaneProps {
  pane: PaneRuntime;
  isLeader: boolean;
  isPlaying: boolean;
  playbackRate: number;
  onPromoteLeader: () => void;
  onVolumeChange: (value: number) => void;
  onToggleMute: () => void;
  onAttachVideo: (paneIndex: number, video: HTMLVideoElement | null) => void;
}

/**
 * One pane of the multi-view UI. Owns its own `<video>` element,
 * fetches the asset URL via `getVideoSource` (same as the single-
 * player route — ADR-0019 path validation applies), and surfaces
 * the per-pane chrome: leader badge, promote-to-leader button,
 * volume + mute, out-of-range overlay.
 *
 * The actual sync correction lives in the `useSyncLoop` hook
 * mounted at the page level; the pane just exposes its `<video>`
 * imperatively via `onAttachVideo`.
 */
export function MultiViewPane({
  pane,
  isLeader,
  isPlaying,
  playbackRate,
  onPromoteLeader,
  onVolumeChange,
  onToggleMute,
  onAttachVideo,
}: MultiViewPaneProps) {
  const videoRef = useRef<HTMLVideoElement | null>(null);

  const source = useQuery<VideoSource>({
    queryKey: ["video-source", pane.vodId],
    queryFn: () => commands.getVideoSource({ vodId: pane.vodId }),
    staleTime: 3_000,
  });

  // Bind the imperative ref to the parent so the sync loop can read
  // currentTime + issue corrective seeks.
  useEffect(() => {
    onAttachVideo(pane.paneIndex, videoRef.current);
    return () => onAttachVideo(pane.paneIndex, null);
  }, [pane.paneIndex, onAttachVideo]);

  // Mirror group-wide transport state onto the underlying video
  // element. Best-effort: a `play()` call inside an effect can be
  // rejected if the user hasn't gestured first; the UI still
  // reflects the intended state.
  useEffect(() => {
    const v = videoRef.current;
    if (!v) return;
    if (isPlaying && v.paused) {
      void v.play().catch(() => {
        /* autoplay rejection — group transport renders the
           "click to start" hint at the page level. */
      });
    } else if (!isPlaying && !v.paused) {
      v.pause();
    }
  }, [isPlaying]);

  useEffect(() => {
    const v = videoRef.current;
    if (v) v.playbackRate = playbackRate;
  }, [playbackRate]);

  // Apply the pane-local volume + mute state to the element.
  useEffect(() => {
    const v = videoRef.current;
    if (!v) return;
    v.volume = pane.volume;
    v.muted = pane.muted;
  }, [pane.volume, pane.muted]);

  const sourceState = source.data?.state ?? (source.isLoading ? "loading" : "missing");
  const resolvedUrl = source.data?.path ? assetUrl(source.data.path) : "";

  return (
    <article
      className="relative h-full bg-black overflow-hidden flex flex-col"
      aria-label={`Multi-view pane ${pane.paneIndex + 1}${isLeader ? " (leader)" : ""}`}
    >
      {sourceState === "ready" && resolvedUrl && (
        // The captions/track issue is the same as the single-player
        // route — Phase 7 polish. See docs/a11y-exceptions.md.
        // eslint-disable-next-line jsx-a11y/media-has-caption
        <video
          ref={videoRef}
          src={resolvedUrl}
          aria-label={`Pane ${pane.paneIndex + 1}`}
          preload="metadata"
          className="block w-full h-full bg-black object-contain"
          controls={false}
        />
      )}
      {sourceState === "loading" && (
        <div className="flex-1 flex items-center justify-center text-white/70 text-sm">
          Loading pane…
        </div>
      )}
      {sourceState === "missing" && (
        <div className="flex-1 flex items-center justify-center text-white/70 text-xs px-4 text-center">
          Pane source missing — start the download from the Library detail
          drawer.
        </div>
      )}
      {sourceState === "partial" && (
        <div className="flex-1 flex items-center justify-center text-white/70 text-xs">
          This pane&apos;s VOD is still downloading.
        </div>
      )}

      {/* Out-of-range overlay — covers the video when the leader's
          wall-clock falls outside this pane's range. */}
      {pane.outOfRange && (
        <div
          role="alert"
          className="absolute inset-0 flex flex-col items-center justify-center gap-2 bg-black/85 text-white"
        >
          <p className="text-sm font-medium">Out of range</p>
          <p className="text-xs text-white/70 max-w-[16rem] text-center">
            Sightline is waiting for the leader&apos;s wall-clock to re-enter
            this pane&apos;s window.
          </p>
        </div>
      )}

      {/* Pane chrome — leader badge, promote button, audio mix. */}
      <div
        className="absolute top-2 left-2 right-2 flex items-center gap-2"
        aria-label={`Pane ${pane.paneIndex + 1} controls`}
      >
        {isLeader ? (
          <span
            aria-label="Leader pane"
            title="Leader pane drives the wall-clock"
            className="text-xs font-medium px-2 py-1 rounded-full bg-amber-300/90 text-black"
          >
            ★ Leader
          </span>
        ) : (
          <button
            type="button"
            onClick={onPromoteLeader}
            className="text-xs font-medium px-2 py-1 rounded-full bg-white/90 text-black hover:bg-white focus-visible:outline focus-visible:outline-2 focus-visible:outline-[--color-accent]"
          >
            Promote to leader
          </button>
        )}
        <div className="ml-auto flex items-center gap-1 bg-black/60 rounded-full px-2 py-1">
          <button
            type="button"
            onClick={onToggleMute}
            aria-label={pane.muted ? `Unmute pane ${pane.paneIndex + 1}` : `Mute pane ${pane.paneIndex + 1}`}
            aria-pressed={pane.muted}
            className="text-xs text-white/90 hover:text-white px-1 focus-visible:outline focus-visible:outline-2 focus-visible:outline-[--color-accent]"
          >
            {pane.muted || pane.volume === 0
              ? "🔇"
              : pane.volume < 0.5
                ? "🔉"
                : "🔊"}
          </button>
          <input
            type="range"
            min={0}
            max={1}
            step={0.01}
            value={pane.muted ? 0 : pane.volume}
            aria-label={`Pane ${pane.paneIndex + 1} volume`}
            aria-valuenow={Math.round((pane.muted ? 0 : pane.volume) * 100)}
            aria-valuemin={0}
            aria-valuemax={100}
            onChange={(e) => onVolumeChange(Number(e.target.value))}
            className="w-20 accent-[--color-accent]"
          />
        </div>
      </div>
    </article>
  );
}
