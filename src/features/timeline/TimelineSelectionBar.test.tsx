/**
 * TimelineSelectionBar tests — D7 multi-select toolbar visibility, the
 * [2, 4] "Open in Multiview" enablement window, and the reduced-motion
 * entry animation.
 */

import { afterEach, describe, expect, test } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";

import { useTimelineMultiselectStore } from "@/stores/timeline-multiselect-store";

import { TimelineSelectionBar } from "./TimelineSelectionBar";

afterEach(() => {
  cleanup();
  useTimelineMultiselectStore.getState().clear();
});

describe("TimelineSelectionBar visibility", () => {
  test("renders nothing while the selection is empty", () => {
    const { container } = render(<TimelineSelectionBar />);
    expect(container.firstChild).toBeNull();
  });

  test("appears with a count once a VOD is selected", () => {
    useTimelineMultiselectStore.getState().toggle("v1");
    render(<TimelineSelectionBar />);
    expect(screen.getByText("1 selected")).toBeInTheDocument();
  });
});

describe("TimelineSelectionBar — Open in Multiview enablement (D7)", () => {
  test("a single selection cannot open Multiview (needs ≥ 2)", () => {
    useTimelineMultiselectStore.getState().toggle("v1");
    render(<TimelineSelectionBar />);
    expect(
      screen.getByRole("button", { name: "Open in Multiview" }),
    ).toBeDisabled();
  });

  test("two selections enable Open in Multiview", () => {
    useTimelineMultiselectStore.getState().toggle("v1");
    useTimelineMultiselectStore.getState().toggle("v2");
    render(<TimelineSelectionBar />);
    expect(
      screen.getByRole("button", { name: "Open in Multiview" }),
    ).toBeEnabled();
  });
});

describe("TimelineSelectionBar — Clear", () => {
  test("the Clear button empties the selection and dismisses the bar", () => {
    useTimelineMultiselectStore.getState().toggle("v1");
    render(<TimelineSelectionBar />);
    fireEvent.click(screen.getByRole("button", { name: /Clear/ }));
    expect(useTimelineMultiselectStore.getState().selectedVodIds.size).toBe(0);
    expect(screen.queryByText(/selected/)).toBeNull();
  });
});

describe("TimelineSelectionBar — reduced motion", () => {
  test("entry uses the globals.css RM-governed fade class", () => {
    // `.sightline-fade-in` is disabled under `prefers-reduced-motion`
    // by the globals.css override block — the bar opts into that
    // mechanism rather than carrying a bespoke animation.
    useTimelineMultiselectStore.getState().toggle("v1");
    render(<TimelineSelectionBar />);
    expect(screen.getByRole("region", { name: "Timeline selection" })).toHaveClass(
      "sightline-fade-in",
    );
  });
});
