import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import { useSettings } from "@/features/settings/use-settings";
import { commands } from "@/ipc";
import { useNavStore } from "@/stores/nav-store";

import { MultiViewPane } from "./MultiViewPane";
import { MultiViewTransportBar } from "./MultiViewTransportBar";
import { DEFAULT_SYNC_DRIFT_THRESHOLD_MS } from "./sync-math";
import { useMultiViewStore } from "./sync-store";
import { useSyncLoop } from "./use-sync-loop";

export interface MultiViewPageProps {
  /** Entry contract: list of VOD ids the user picked. */
  vodIds: string[];
}

/**
 * `/multiview` route — Phase 6 / ADR-0021..0023.
 * Two-pane horizontal split, group-wide transport, leader-led sync.
 */
export function MultiViewPage({ vodIds }: MultiViewPageProps) {
  const closeMultiView = useNavStore((s) => s.closeMultiView);
  const settings = useSettings();
  const thresholdMs =
    settings.data?.syncDriftThresholdMs ?? DEFAULT_SYNC_DRIFT_THRESHOLD_MS;

  const phase = useMultiViewStore((s) => s.phase);
  const sessionId = useMultiViewStore((s) => s.sessionId);
  const panes = useMultiViewStore((s) => s.panes);
  const leaderPaneIndex = useMultiViewStore((s) => s.leaderPaneIndex);
  const overlap = useMultiViewStore((s) => s.overlap);
  const errorDetail = useMultiViewStore((s) => s.errorDetail);
  const isPlaying = useMultiViewStore((s) => s.isPlaying);
  const playbackRate = useMultiViewStore((s) => s.playbackRate);
  const openSession = useMultiViewStore((s) => s.openSession);
  const closeSessionAction = useMultiViewStore((s) => s.closeSession);
  const setLeader = useMultiViewStore((s) => s.setLeader);
  const setVolume = useMultiViewStore((s) => s.setVolume);
  const toggleMute = useMultiViewStore((s) => s.toggleMute);
  const setOutOfRange = useMultiViewStore((s) => s.setOutOfRange);
  const recordDrift = useMultiViewStore((s) => s.recordDrift);
  const setPlaying = useMultiViewStore((s) => s.setPlaying);
  const setPlaybackRate = useMultiViewStore((s) => s.setPlaybackRate);

  // Refs to each pane's <video> element; the sync loop reads them.
  const videoRefs = useRef<Map<number, HTMLVideoElement | null>>(new Map());

  // Open the session on mount; close on unmount.
  useEffect(() => {
    void openSession(vodIds);
    return () => {
      void closeSessionAction();
    };
    // We intentionally re-open if the input vodIds change (route param
    // change) — the cleanup runs first via the previous effect.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [vodIds.join("|")]);

  const onAttachVideo = useCallback(
    (paneIndex: number, video: HTMLVideoElement | null) => {
      if (video) videoRefs.current.set(paneIndex, video);
      else videoRefs.current.delete(paneIndex);
    },
    []
  );

  // The leader's wall-clock moment, derived for the seek slider.
  // Re-renders the slider every 250 ms via a local timer so the
  // marker tracks playback without putting the value into the store
  // (which would re-render every pane on every tick).
  const [leaderMoment, setLeaderMoment] = useState<number | null>(null);
  useEffect(() => {
    if (sessionId == null || leaderPaneIndex == null) return;
    const id = window.setInterval(() => {
      const leader = panes.find((p) => p.paneIndex === leaderPaneIndex);
      const v = videoRefs.current.get(leaderPaneIndex);
      if (!leader || !v) return;
      setLeaderMoment(leader.streamStartedAt + Math.max(0, v.currentTime));
    }, 250);
    return () => window.clearInterval(id);
  }, [sessionId, leaderPaneIndex, panes]);

  useSyncLoop({
    sessionId,
    leaderPaneIndex,
    panes,
    thresholdMs,
    videoRefs,
    onOutOfRange: setOutOfRange,
    onDriftCorrected: recordDrift,
  });

  const handleClose = useCallback(() => {
    void closeSessionAction().then(() => closeMultiView("library"));
  }, [closeSessionAction, closeMultiView]);

  const handleTogglePlay = useCallback(async () => {
    const next = !isPlaying;
    setPlaying(next);
    if (sessionId == null) return;
    try {
      if (next) {
        await commands.syncPlay({ sessionId });
      } else {
        await commands.syncPause({ sessionId });
      }
    } catch {
      // Roll back optimistic toggle on failure.
      setPlaying(!next);
    }
  }, [isPlaying, sessionId, setPlaying]);

  const handleSeek = useCallback(
    async (wallClockTs: number) => {
      const leader = panes.find((p) => p.paneIndex === leaderPaneIndex);
      const v =
        leaderPaneIndex != null ? videoRefs.current.get(leaderPaneIndex) : null;
      if (leader && v) {
        v.currentTime = Math.max(
          0,
          Math.min(leader.durationSeconds, wallClockTs - leader.streamStartedAt)
        );
      }
      if (sessionId != null) {
        try {
          await commands.syncSeek({ sessionId, wallClockTs });
        } catch {
          /* the leader's local seek already happened; ignore. */
        }
      }
    },
    [leaderPaneIndex, panes, sessionId]
  );

  const handleSetSpeed = useCallback(
    async (rate: number) => {
      setPlaybackRate(rate);
      if (sessionId == null) return;
      try {
        await commands.syncSetSpeed({ sessionId, speed: rate });
      } catch {
        /* keep the optimistic UI; backend speed is informational. */
      }
    },
    [sessionId, setPlaybackRate]
  );

  const handlePromoteLeader = useCallback(
    (paneIndex: number) => {
      void setLeader(paneIndex);
    },
    [setLeader]
  );

  const renderedPanes = useMemo(
    () =>
      panes.map((pane) => (
        <MultiViewPane
          key={pane.paneIndex}
          pane={pane}
          isLeader={pane.paneIndex === leaderPaneIndex}
          isPlaying={isPlaying}
          playbackRate={playbackRate}
          onPromoteLeader={() => handlePromoteLeader(pane.paneIndex)}
          onVolumeChange={(v) => setVolume(pane.paneIndex, v)}
          onToggleMute={() => toggleMute(pane.paneIndex)}
          onAttachVideo={onAttachVideo}
        />
      )),
    [
      panes,
      leaderPaneIndex,
      isPlaying,
      playbackRate,
      handlePromoteLeader,
      setVolume,
      toggleMute,
      onAttachVideo,
    ]
  );

  return (
    <div
      className="relative h-full flex flex-col bg-black text-white"
      role="region"
      aria-label="Multi-view"
    >
      <header className="flex items-center justify-between px-4 py-2 border-b border-white/10 text-xs">
        <h2 id="mv-title" className="text-sm font-medium">
          Multi-View
          {phase === "opening" && (
            <span className="ml-2 text-white/60">(opening…)</span>
          )}
        </h2>
        <span className="text-white/60">
          {panes.length === 2
            ? `Leader: pane ${(leaderPaneIndex ?? 0) + 1}`
            : ""}
        </span>
      </header>

      {errorDetail && (
        <p
          role="alert"
          className="bg-red-900/70 text-red-100 text-xs px-4 py-2"
        >
          {errorDetail}
        </p>
      )}

      <div
        className="flex-1 grid grid-cols-2 gap-1 min-h-0"
        aria-label="Multi-view panes"
      >
        {renderedPanes}
      </div>

      <MultiViewTransportBar
        isPlaying={isPlaying}
        onTogglePlay={handleTogglePlay}
        overlap={overlap}
        leaderMomentUnixSeconds={leaderMoment}
        onSeek={handleSeek}
        playbackRate={playbackRate}
        onSetSpeed={handleSetSpeed}
        onClose={handleClose}
      />
    </div>
  );
}
