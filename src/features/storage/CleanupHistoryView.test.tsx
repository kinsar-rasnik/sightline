import "@testing-library/jest-dom/vitest";
import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import type { CleanupLogEntry } from "@/ipc";
import { CleanupHistoryView } from "./CleanupHistoryView";

const sample: CleanupLogEntry = {
  id: 1,
  ranAt: 1_700_000_000,
  mode: "scheduled",
  freedBytes: 1024 * 1024 * 1024,
  deletedVodCount: 3,
  status: "ok",
};

describe("CleanupHistoryView", () => {
  it("renders an empty state when no rows are present", () => {
    render(<CleanupHistoryView entries={[]} />);
    expect(screen.getByText(/no cleanup history yet/i)).toBeInTheDocument();
  });

  it("renders a row with mode, count, and freed bytes", () => {
    render(<CleanupHistoryView entries={[sample]} />);
    expect(screen.getByText(/scheduled/)).toBeInTheDocument();
    expect(screen.getByText(/3 VODs/i)).toBeInTheDocument();
    expect(screen.getByText("ok")).toBeInTheDocument();
  });

  it("colour-codes partial status differently from ok", () => {
    const { rerender } = render(
      <CleanupHistoryView entries={[{ ...sample, status: "ok" }]} />
    );
    const ok = screen.getByText("ok");
    expect(ok.className).toContain("text-emerald-600");
    rerender(
      <CleanupHistoryView entries={[{ ...sample, status: "partial" }]} />
    );
    const partial = screen.getByText("partial");
    expect(partial.className).toContain("text-amber-600");
  });
});
