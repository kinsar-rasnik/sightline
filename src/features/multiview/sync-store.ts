import { create } from "zustand";

import { commands, type SyncSession } from "@/ipc";

import type { OverlapWindow } from "./sync-math";

export type SessionPhase = "idle" | "opening" | "active" | "closing" | "closed";

export interface PaneRuntime {
  paneIndex: number;
  vodId: string;
  /** Per-pane volume slider value (0..=1). */
  volume: number;
  muted: boolean;
  /** Wall-clock window the pane's VOD covers. Driven by `getOverlap`
   *  on session open + cached so the leader-bound seek slider can
   *  render before the next IPC tick. */
  streamStartedAt: number;
  durationSeconds: number;
  /** Whether the pane is currently outside the leader's wall-clock
   *  window. Hidden behind the "Out of range" overlay when true. */
  outOfRange: boolean;
  /** Last drift correction we issued, in ms (signed). null when no
   *  correction has happened yet this session. */
  lastDriftMs: number | null;
}

interface MultiViewState {
  phase: SessionPhase;
  sessionId: number | null;
  /** Pane state, in pane-index order (length 0 or 2). */
  panes: PaneRuntime[];
  leaderPaneIndex: number | null;
  /** Wall-clock overlap of the two panes — drives the seek slider. */
  overlap: OverlapWindow;
  /** Last error string from an IPC call, or null. */
  errorDetail: string | null;

  // Group transport state mirrored optimistically. The renderer drives
  // the actual <video> elements; this is what the controls render.
  isPlaying: boolean;
  playbackRate: number;

  // Actions —
  openSession: (vodIds: string[]) => Promise<void>;
  closeSession: () => Promise<void>;
  setLeader: (paneIndex: number) => Promise<void>;
  setVolume: (paneIndex: number, value: number) => void;
  toggleMute: (paneIndex: number) => void;
  setOutOfRange: (paneIndex: number, value: boolean) => void;
  recordDrift: (paneIndex: number, driftMs: number) => void;
  setPlaying: (playing: boolean) => void;
  setPlaybackRate: (rate: number) => void;
  reset: () => void;
}

const INITIAL_OVERLAP: OverlapWindow = { startAt: 0, endAt: 0 };

const INITIAL_STATE: Pick<
  MultiViewState,
  | "phase"
  | "sessionId"
  | "panes"
  | "leaderPaneIndex"
  | "overlap"
  | "errorDetail"
  | "isPlaying"
  | "playbackRate"
> = {
  phase: "idle",
  sessionId: null,
  panes: [],
  leaderPaneIndex: null,
  overlap: INITIAL_OVERLAP,
  errorDetail: null,
  isPlaying: false,
  playbackRate: 1,
};

function adoptSession(
  session: SyncSession,
  panesRuntime: PaneRuntime[]
): Partial<MultiViewState> {
  return {
    phase: session.status === "active" ? "active" : "closed",
    sessionId: session.id,
    panes: panesRuntime,
    leaderPaneIndex: session.leaderPaneIndex ?? 0,
  };
}

export const useMultiViewStore = create<MultiViewState>((set, get) => ({
  ...INITIAL_STATE,

  async openSession(vodIds) {
    if (vodIds.length !== 2) {
      set({ errorDetail: "Multi-view requires exactly two VODs" });
      return;
    }
    set({ phase: "opening", errorDetail: null });
    try {
      // Three concurrent requests: the overlap window for the
      // seek-bar bound, the session row, and the VOD metadata for
      // each pane. The backend already validates VOD existence on
      // open, so the per-pane getVod is purely for ranges.
      const [overlap, session, ...vods] = await Promise.all([
        commands.getOverlap({ vodIds }),
        commands.openSyncGroup({ vodIds, layout: "split-50-50" }),
        ...vodIds.map((id) => commands.getVod({ twitchVideoId: id })),
      ]);
      const vodById = new Map(vods.map((v) => [v.vod.twitchVideoId, v.vod]));
      const panes: PaneRuntime[] = session.panes.map((p) => {
        const v = vodById.get(p.vodId);
        return {
          paneIndex: p.paneIndex,
          vodId: p.vodId,
          volume: p.volume,
          muted: p.muted,
          streamStartedAt: v?.streamStartedAt ?? 0,
          durationSeconds: v?.durationSeconds ?? 0,
          outOfRange: false,
          lastDriftMs: null,
        };
      });
      set({
        ...adoptSession(session, panes),
        overlap: overlap.window,
        errorDetail: null,
      });
    } catch (err) {
      const detail = err instanceof Error ? err.message : "open failed";
      set({ phase: "closed", errorDetail: detail });
    }
  },

  async closeSession() {
    const id = get().sessionId;
    if (id == null) {
      set(INITIAL_STATE);
      return;
    }
    set({ phase: "closing" });
    try {
      await commands.closeSyncGroup({ sessionId: id });
    } catch (err) {
      const detail = err instanceof Error ? err.message : "close failed";
      set({ errorDetail: detail });
    } finally {
      set(INITIAL_STATE);
    }
  },

  async setLeader(paneIndex) {
    const id = get().sessionId;
    if (id == null) return;
    try {
      const session = await commands.setSyncLeader({
        sessionId: id,
        paneIndex,
      });
      set({ leaderPaneIndex: session.leaderPaneIndex ?? paneIndex });
    } catch (err) {
      const detail = err instanceof Error ? err.message : "set leader failed";
      set({ errorDetail: detail });
    }
  },

  setVolume(paneIndex, value) {
    const clamped = Math.min(1, Math.max(0, value));
    set((state) => ({
      panes: state.panes.map((p) =>
        p.paneIndex === paneIndex ? { ...p, volume: clamped } : p
      ),
    }));
  },

  toggleMute(paneIndex) {
    set((state) => ({
      panes: state.panes.map((p) =>
        p.paneIndex === paneIndex ? { ...p, muted: !p.muted } : p
      ),
    }));
  },

  setOutOfRange(paneIndex, value) {
    set((state) => ({
      panes: state.panes.map((p) =>
        p.paneIndex === paneIndex ? { ...p, outOfRange: value } : p
      ),
    }));
  },

  recordDrift(paneIndex, driftMs) {
    set((state) => ({
      panes: state.panes.map((p) =>
        p.paneIndex === paneIndex ? { ...p, lastDriftMs: driftMs } : p
      ),
    }));
  },

  setPlaying(playing) {
    set({ isPlaying: playing });
  },

  setPlaybackRate(rate) {
    set({ playbackRate: rate });
  },

  reset() {
    set(INITIAL_STATE);
  },
}));
