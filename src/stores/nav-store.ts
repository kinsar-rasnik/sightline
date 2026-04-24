import { create } from "zustand";

export type NavPage = "library" | "streamers" | "settings";

interface NavState {
  page: NavPage;
  setPage: (page: NavPage) => void;
}

export const useNavStore = create<NavState>((set) => ({
  page: "library",
  setPage: (page) => set({ page }),
}));
