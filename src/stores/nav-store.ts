import { create } from "zustand";

export type NavPage =
  | "library"
  | "timeline"
  | "streamers"
  | "downloads"
  | "settings"
  | "watch"
  | "multiview";

interface WatchContext {
  /** VOD id currently being played. */
  vodId: string;
  /** Optional initial seek position in seconds (used by cross-streamer deep link). */
  initialPositionSeconds?: number;
  /** Whether to auto-play on mount. Respected by the player alongside the Settings
   *  "Auto-play on open" toggle. */
  autoplay?: boolean;
}

interface MultiViewContext {
  /** VOD ids the user picked, in pane order (index 0 = primary). */
  vodIds: string[];
}

interface NavState {
  page: NavPage;
  watch: WatchContext | null;
  multiView: MultiViewContext | null;
  setPage: (page: NavPage) => void;
  openPlayer: (ctx: WatchContext) => void;
  closePlayer: (fallback?: NavPage) => void;
  openMultiView: (ctx: MultiViewContext) => void;
  closeMultiView: (fallback?: NavPage) => void;
}

export const useNavStore = create<NavState>((set) => ({
  page: "library",
  watch: null,
  multiView: null,
  setPage: (page) =>
    set((state) => ({
      page,
      watch: page === "watch" ? state.watch : null,
      multiView: page === "multiview" ? state.multiView : null,
    })),
  openPlayer: (ctx) => set({ page: "watch", watch: ctx, multiView: null }),
  closePlayer: (fallback = "library") => set({ page: fallback, watch: null }),
  openMultiView: (ctx) =>
    set({ page: "multiview", multiView: ctx, watch: null }),
  closeMultiView: (fallback = "library") =>
    set({ page: fallback, multiView: null }),
}));
