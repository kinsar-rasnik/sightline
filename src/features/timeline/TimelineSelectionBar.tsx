/**
 * TimelineSelectionBar — the floating multi-selection toolbar
 * (ADR-0038 D7).
 *
 * Mounts at the bottom-centre of the viewport whenever the selection is
 * non-empty. The "Open in Multiview" CTA is enabled in the [2, 4] range
 * — matching the CEO-A1 4-pane Multiview capacity — and hands the
 * selection to the Multiview surface via the multiselect store's
 * `commit` action (D8). "Clear" empties the selection and dismisses the
 * bar.
 */

import { X } from "lucide-react";

import {
  MAX_MULTIVIEW_SELECTION,
  MIN_MULTIVIEW_SELECTION,
  useTimelineMultiselectStore,
} from "@/stores/timeline-multiselect-store";

export function TimelineSelectionBar() {
  const selectedCount = useTimelineMultiselectStore(
    (s) => s.selectedVodIds.size,
  );
  const clear = useTimelineMultiselectStore((s) => s.clear);
  const commit = useTimelineMultiselectStore((s) => s.commit);

  if (selectedCount === 0) return null;

  const canOpenMultiview =
    selectedCount >= MIN_MULTIVIEW_SELECTION &&
    selectedCount <= MAX_MULTIVIEW_SELECTION;

  return (
    <div
      role="region"
      aria-label="Timeline selection"
      className="sightline-fade-in absolute bottom-[80px] left-1/2 z-[var(--z-selection-bar)] flex -translate-x-1/2 items-center gap-3 rounded-md bg-surface-2 px-4 py-2 shadow-toolbar"
    >
      <span className="font-tabular text-sm text-fg" aria-live="polite">
        {selectedCount} selected
      </span>
      <button
        type="button"
        onClick={commit}
        disabled={!canOpenMultiview}
        className="rounded-sm bg-accent px-3 py-1.5 text-sm font-mid text-accent-fg disabled:opacity-50"
      >
        Open in Multiview
      </button>
      <button
        type="button"
        onClick={clear}
        className="inline-flex items-center gap-1 rounded-sm bg-surface-3 px-3 py-1.5 text-sm text-fg"
      >
        <X width={14} height={14} strokeWidth={1.5} aria-hidden="true" />
        Clear
      </button>
    </div>
  );
}
