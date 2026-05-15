/**
 * timeline-multiselect-store tests — D7 toggle/range gestures, the
 * 4-pane selection cap, and the D8 commit hand-off to nav-store.
 */

import { beforeEach, describe, expect, test } from "vitest";

import { useNavStore } from "./nav-store";
import {
  MAX_MULTIVIEW_SELECTION,
  useTimelineMultiselectStore,
} from "./timeline-multiselect-store";

beforeEach(() => {
  useTimelineMultiselectStore.getState().clear();
  useNavStore.setState({ page: "library", watch: null, multiView: null });
});

describe("toggle", () => {
  test("adds a VOD and flips the mode to multi", () => {
    useTimelineMultiselectStore.getState().toggle("a");
    const state = useTimelineMultiselectStore.getState();
    expect([...state.selectedVodIds]).toEqual(["a"]);
    expect(state.selectionMode).toBe("multi");
  });

  test("toggling the same VOD twice removes it and returns to single", () => {
    const store = useTimelineMultiselectStore.getState();
    store.toggle("a");
    store.toggle("a");
    const state = useTimelineMultiselectStore.getState();
    expect(state.selectedVodIds.size).toBe(0);
    expect(state.selectionMode).toBe("single");
  });

  test("adds are capped at the 4-pane Multiview limit", () => {
    const store = useTimelineMultiselectStore.getState();
    for (const id of ["a", "b", "c", "d", "e"]) store.toggle(id);
    expect(useTimelineMultiselectStore.getState().selectedVodIds.size).toBe(
      MAX_MULTIVIEW_SELECTION,
    );
  });
});

describe("selectRange", () => {
  test("adds the contiguous run between two VODs", () => {
    useTimelineMultiselectStore
      .getState()
      .selectRange("b", "d", ["a", "b", "c", "d", "e"]);
    expect([...useTimelineMultiselectStore.getState().selectedVodIds].sort()).toEqual(
      ["b", "c", "d"],
    );
  });

  test("a range longer than the cap fills only up to the limit", () => {
    useTimelineMultiselectStore
      .getState()
      .selectRange("a", "f", ["a", "b", "c", "d", "e", "f"]);
    expect(useTimelineMultiselectStore.getState().selectedVodIds.size).toBe(
      MAX_MULTIVIEW_SELECTION,
    );
  });
});

describe("clear", () => {
  test("empties the selection and resets the mode", () => {
    const store = useTimelineMultiselectStore.getState();
    store.toggle("a");
    store.clear();
    const state = useTimelineMultiselectStore.getState();
    expect(state.selectedVodIds.size).toBe(0);
    expect(state.selectionMode).toBe("single");
  });
});

describe("commit (D8 nav-store hand-off)", () => {
  test("a selection of ≥ 2 opens Multiview with those VOD ids", () => {
    const store = useTimelineMultiselectStore.getState();
    store.toggle("a");
    store.toggle("b");
    store.commit();
    const nav = useNavStore.getState();
    expect(nav.page).toBe("multiview");
    expect(nav.multiView?.vodIds.sort()).toEqual(["a", "b"]);
  });

  test("a single-VOD selection does not open Multiview", () => {
    const store = useTimelineMultiselectStore.getState();
    store.toggle("a");
    store.commit();
    expect(useNavStore.getState().page).toBe("library");
  });
});
