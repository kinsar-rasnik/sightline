import { create } from "zustand";

export type NavPage =
  | "library"
  | "timeline"
  | "streamers"
  | "downloads"
  | "settings"
  | "watch";

interface WatchContext {
  /** VOD id currently being played. */
  vodId: string;
  /** Optional initial seek position in seconds (used by cross-streamer deep link). */
  initialPositionSeconds?: number;
  /** Whether to auto-play on mount. Respected by the player alongside the Settings
   *  "Auto-play on open" toggle. */
  autoplay?: boolean;
}

interface NavState {
  page: NavPage;
  watch: WatchContext | null;
  setPage: (page: NavPage) => void;
  openPlayer: (ctx: WatchContext) => void;
  closePlayer: (fallback?: NavPage) => void;
}

export const useNavStore = create<NavState>((set) => ({
  page: "library",
  watch: null,
  setPage: (page) =>
    set((state) => ({
      page,
      watch: page === "watch" ? state.watch : null,
    })),
  openPlayer: (ctx) => set({ page: "watch", watch: ctx }),
  closePlayer: (fallback = "library") => set({ page: fallback, watch: null }),
}));
