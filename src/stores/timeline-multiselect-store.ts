// Timeline multi-selection state — drives the floating selection
// toolbar and the "Open in Multiview" hand-off (ADR-0038 D7 + D8).
//
// R-ADR-01 note. As with timeline-view-store, ADR-0038 D8 lists this
// under `src/stores/`; the brief's `src/features/timeline/` placement
// is superseded by the spec source.

import { create } from "zustand";

import { useNavStore } from "./nav-store";

/**
 * D7: the Multiview selection is firmly capped at 4 panes, matching the
 * CEO-A1 4-pane Multiview capacity. Adds beyond the cap are no-ops.
 */
export const MAX_MULTIVIEW_SELECTION = 4;

/** Minimum panes for a Multiview group (a sync group needs ≥ 2). */
export const MIN_MULTIVIEW_SELECTION = 2;

export type SelectionMode = "single" | "multi";

interface TimelineMultiselectState {
  /** VODs currently selected for Multiview. */
  selectedVodIds: ReadonlySet<string>;
  /** `"multi"` whenever the selection is non-empty (D7 gesture model). */
  selectionMode: SelectionMode;
  /**
   * Toggle one VOD in/out of the selection. Adds are capped at
   * {@link MAX_MULTIVIEW_SELECTION}; removes always succeed.
   */
  toggle: (vodId: string) => void;
  /**
   * Add a contiguous run of VODs (D7 Shift+Click-on-stack bulk-add).
   * `ordered` is the caller's chronological VOD ordering; the run fills
   * up to the selection cap.
   */
  selectRange: (
    fromVodId: string,
    toVodId: string,
    ordered: readonly string[],
  ) => void;
  /** Empty the selection and dismiss the toolbar. */
  clear: () => void;
  /** D7/D8: hand the selection to the Multiview surface via nav-store. */
  commit: () => void;
}

function modeFor(set: ReadonlySet<string>): SelectionMode {
  return set.size > 0 ? "multi" : "single";
}

export const useTimelineMultiselectStore = create<TimelineMultiselectState>(
  (set, get) => ({
    selectedVodIds: new Set<string>(),
    selectionMode: "single",
    toggle: (vodId) =>
      set((s) => {
        const next = new Set(s.selectedVodIds);
        if (next.has(vodId)) {
          next.delete(vodId);
        } else if (next.size < MAX_MULTIVIEW_SELECTION) {
          next.add(vodId);
        } else {
          return s; // cap reached — adding is a no-op
        }
        return { selectedVodIds: next, selectionMode: modeFor(next) };
      }),
    selectRange: (fromVodId, toVodId, ordered) =>
      set((s) => {
        const i = ordered.indexOf(fromVodId);
        const j = ordered.indexOf(toVodId);
        if (i === -1 || j === -1) return s;
        const lo = Math.min(i, j);
        const hi = Math.max(i, j);
        const next = new Set(s.selectedVodIds);
        for (let k = lo; k <= hi && next.size < MAX_MULTIVIEW_SELECTION; k++) {
          const id = ordered[k];
          if (id) next.add(id);
        }
        return { selectedVodIds: next, selectionMode: modeFor(next) };
      }),
    clear: () => set({ selectedVodIds: new Set(), selectionMode: "single" }),
    commit: () => {
      const ids = [...get().selectedVodIds];
      if (ids.length < MIN_MULTIVIEW_SELECTION) return;
      useNavStore.getState().openMultiView({ vodIds: ids });
    },
  }),
);

/** Per-card selection subscription — re-renders only this VOD's card. */
export function useIsVodSelected(vodId: string): boolean {
  return useTimelineMultiselectStore((s) => s.selectedVodIds.has(vodId));
}
