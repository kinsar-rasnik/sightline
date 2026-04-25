import { useEffect, useRef } from "react";

import { commands } from "@/ipc";

import {
  decideDriftCorrection,
  isMemberOutOfRange,
  SYNC_LOOP_TICK_MS,
} from "./sync-math";
import type { PaneRuntime } from "./sync-store";

export interface SyncLoopInputs {
  sessionId: number | null;
  leaderPaneIndex: number | null;
  panes: PaneRuntime[];
  /** Drift threshold in ms (from app_settings). */
  thresholdMs: number;
  /** Per-pane <video> refs, keyed by pane index. */
  videoRefs: React.MutableRefObject<Map<number, HTMLVideoElement | null>>;
  /** Frontend-side state mutators. */
  onOutOfRange: (paneIndex: number, value: boolean) => void;
  onDriftCorrected: (paneIndex: number, driftMs: number) => void;
}

/**
 * 250 ms-tick sync loop. ADR-0022:
 *   - setInterval (NOT requestAnimationFrame) — drift detection at
 *     the threshold's cadence is the right rate.
 *   - Pause-when-hidden via document.visibilityState — browser
 *     already throttles `<video>` decoders in background tabs.
 *   - Per-pane cooldown (1 s) on corrective seeks to break a
 *     potential thrash loop between measurement → seek → measurement.
 *
 * Returns nothing — the loop manages its own interval; the caller
 * just mounts the hook with up-to-date `inputs`.
 */
export function useSyncLoop(inputs: SyncLoopInputs): void {
  const lastCorrectionAt = useRef<Map<number, number>>(new Map());

  useEffect(() => {
    if (
      inputs.sessionId == null ||
      inputs.leaderPaneIndex == null ||
      inputs.panes.length < 2
    ) {
      return;
    }

    let cancelled = false;

    const tick = () => {
      if (cancelled || document.visibilityState === "hidden") return;
      const leader = inputs.panes.find(
        (p) => p.paneIndex === inputs.leaderPaneIndex
      );
      if (!leader) return;
      const leaderVideo = inputs.videoRefs.current.get(leader.paneIndex);
      if (!leaderVideo) return;
      const leaderMoment =
        leader.streamStartedAt + Math.max(0, leaderVideo.currentTime);

      for (const pane of inputs.panes) {
        if (pane.paneIndex === leader.paneIndex) continue;
        const followerVideo = inputs.videoRefs.current.get(pane.paneIndex);
        if (!followerVideo) continue;

        // Out-of-range check first — there's no point correcting a
        // pane that doesn't have video for the current moment.
        const oor = isMemberOutOfRange(
          leaderMoment,
          pane.streamStartedAt,
          pane.durationSeconds
        );
        if (oor !== pane.outOfRange) {
          inputs.onOutOfRange(pane.paneIndex, oor);
          if (oor && inputs.sessionId != null) {
            void commands.reportSyncOutOfRange({
              sessionId: inputs.sessionId,
              paneIndex: pane.paneIndex,
            });
          }
        }
        if (oor) {
          if (!followerVideo.paused) followerVideo.pause();
          continue;
        }

        const last = lastCorrectionAt.current.get(pane.paneIndex) ?? 0;
        const decision = decideDriftCorrection({
          leaderMomentUnixSeconds: leaderMoment,
          followerStreamStartedAt: pane.streamStartedAt,
          followerDurationSeconds: pane.durationSeconds,
          followerCurrentTimeSeconds: followerVideo.currentTime,
          thresholdMs: inputs.thresholdMs,
          msSinceLastCorrection: Date.now() - last,
        });

        if (decision.shouldCorrect) {
          followerVideo.currentTime = decision.expectedPositionSeconds;
          lastCorrectionAt.current.set(pane.paneIndex, Date.now());
          inputs.onDriftCorrected(pane.paneIndex, decision.driftMs);
          if (inputs.sessionId != null) {
            void commands.recordSyncDrift({
              sessionId: inputs.sessionId,
              measurement: {
                paneIndex: pane.paneIndex,
                followerPositionSeconds: followerVideo.currentTime,
                expectedPositionSeconds: decision.expectedPositionSeconds,
                driftMs: decision.driftMs,
              },
            });
          }
        }
      }
    };

    const interval = window.setInterval(tick, SYNC_LOOP_TICK_MS);
    return () => {
      cancelled = true;
      window.clearInterval(interval);
    };
    // The loop reads from props it captures on each tick; we re-mount
    // the interval whenever the session identity / leader / pane
    // count changes so a fresh closure picks up the new shape.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [
    inputs.sessionId,
    inputs.leaderPaneIndex,
    inputs.panes.length,
    inputs.thresholdMs,
  ]);
}
