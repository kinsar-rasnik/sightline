// Tracks which streamers are currently being polled by the backend.
// The source of truth is the `poll:started` / `poll:finished` Tauri
// event pair; `src/lib/event-subscriptions.ts` pipes those into the
// `add` / `remove` actions. Consumers call `useIsPolling(id)` to drive
// UI spinners.

import { create } from "zustand";

interface ActivePollsState {
  activePolls: ReadonlySet<string>;
  add: (twitchUserId: string) => void;
  remove: (twitchUserId: string) => void;
  clear: () => void;
}

export const useActivePollsStore = create<ActivePollsState>((set) => ({
  activePolls: new Set(),
  add: (id) =>
    set((s) => {
      if (s.activePolls.has(id)) return s;
      const next = new Set(s.activePolls);
      next.add(id);
      return { activePolls: next };
    }),
  remove: (id) =>
    set((s) => {
      if (!s.activePolls.has(id)) return s;
      const next = new Set(s.activePolls);
      next.delete(id);
      return { activePolls: next };
    }),
  clear: () => set({ activePolls: new Set() }),
}));

/** Zustand selector hook — only re-renders when this streamer's flag changes. */
export function useIsPolling(twitchUserId: string): boolean {
  return useActivePollsStore((s) => s.activePolls.has(twitchUserId));
}
