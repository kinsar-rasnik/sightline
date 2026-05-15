/**
 * Lane-packing + multi-perspective grouping unit tests (ADR-0038 D1 + D5).
 *
 * D1 packs intervals into per-side (`above` | `below`) lanes; the brief's
 * "lanes 0 + 1" shorthand maps onto D1's per-side model as distinct
 * (side, laneIdx) slots. The 50 % multi-perspective threshold is the
 * CEO-A2 correction of the PROPOSED 75 % — asserted explicitly below.
 */

import { describe, expect, test } from "vitest";

import {
  groupMultiPerspective,
  MULTI_PERSPECTIVE_OVERLAP_THRESHOLD,
  overlapFractionOfShorter,
  packIntoLanes,
  type LaneAssignment,
  type PackInterval,
} from "./lane-packing";

function iv(
  vodId: string,
  startAt: number,
  endAt: number,
  streamerId: string = vodId,
): PackInterval {
  return { vodId, streamerId, startAt, endAt };
}

function timeOverlaps(a: PackInterval, b: PackInterval): boolean {
  return a.startAt < b.endAt && b.startAt < a.endAt;
}

/** No two time-overlapping intervals may land on the same lane slot. */
function assertNoSlotCollision(
  intervals: PackInterval[],
  result: LaneAssignment[],
): void {
  const byVod = new Map(result.map((r) => [r.vodId, r]));
  for (let i = 0; i < intervals.length; i++) {
    for (let j = i + 1; j < intervals.length; j++) {
      const a = intervals[i]!;
      const b = intervals[j]!;
      if (!timeOverlaps(a, b)) continue;
      const ra = byVod.get(a.vodId)!;
      const rb = byVod.get(b.vodId)!;
      expect(ra.side === rb.side && ra.laneIdx === rb.laneIdx).toBe(false);
    }
  }
}

function slotKey(a: LaneAssignment): string {
  return `${a.side}:${a.laneIdx}`;
}

describe("packIntoLanes — Greedy Best-Fit (ADR-0038 D1)", () => {
  test("empty input yields no assignments", () => {
    expect(packIntoLanes([])).toEqual([]);
  });

  test("single interval lands on the first lane (above, 0)", () => {
    const result = packIntoLanes([iv("a", 0, 100)]);
    expect(result).toEqual([{ vodId: "a", side: "above", laneIdx: 0 }]);
  });

  test("two overlapping intervals occupy two distinct lanes", () => {
    // D1 alternates sides: lane 0 above + lane 0 below — the brief's
    // "lanes 0 + 1" shorthand under D1's per-side model.
    const intervals = [iv("a", 0, 100), iv("b", 50, 150)];
    const result = packIntoLanes(intervals);
    expect(result).toContainEqual({ vodId: "a", side: "above", laneIdx: 0 });
    expect(result).toContainEqual({ vodId: "b", side: "below", laneIdx: 0 });
    expect(new Set(result.map(slotKey)).size).toBe(2);
    assertNoSlotCollision(intervals, result);
  });

  test("three overlapping intervals occupy three distinct lanes", () => {
    const intervals = [iv("a", 0, 100), iv("b", 20, 120), iv("c", 40, 140)];
    const result = packIntoLanes(intervals);
    expect(result).toContainEqual({ vodId: "a", side: "above", laneIdx: 0 });
    expect(result).toContainEqual({ vodId: "b", side: "below", laneIdx: 0 });
    expect(result).toContainEqual({ vodId: "c", side: "above", laneIdx: 1 });
    expect(new Set(result.map(slotKey)).size).toBe(3);
    assertNoSlotCollision(intervals, result);
  });

  test("non-overlapping sequential intervals reuse lane 0", () => {
    const intervals = [iv("a", 0, 100), iv("b", 100, 200), iv("c", 200, 300)];
    const result = packIntoLanes(intervals);
    // Abutting endpoints (tailEnd === startAt) reuse the lane.
    expect(result.every((r) => r.laneIdx === 0)).toBe(true);
    assertNoSlotCollision(intervals, result);
  });

  test("first-fit picks the lowest free lane index when several fit", () => {
    // a/b/c/d fill above[0..1] + below[0..1]; e fits above[0] and
    // above[1] — Best-Fit first-fit takes the lower index.
    const intervals = [
      iv("a", 0, 50),
      iv("b", 0, 50),
      iv("c", 0, 50),
      iv("d", 0, 50),
      iv("e", 100, 150),
    ];
    const result = packIntoLanes(intervals);
    expect(result).toContainEqual({ vodId: "e", side: "above", laneIdx: 0 });
  });

  test("tie-break on equal startAt is deterministic and stable", () => {
    const intervals = [iv("b", 0, 100), iv("a", 0, 100)];
    const first = packIntoLanes(intervals);
    const second = packIntoLanes([...intervals].reverse());
    expect(first).toEqual(second);
    // Lower vodId sorts first → takes the 'above' side.
    expect(first).toContainEqual({ vodId: "a", side: "above", laneIdx: 0 });
    expect(first).toContainEqual({ vodId: "b", side: "below", laneIdx: 0 });
  });

  test("mixed overlap with gaps never collides on a slot", () => {
    const intervals = [
      iv("a", 0, 100),
      iv("b", 50, 150),
      iv("c", 120, 180),
      iv("d", 200, 300),
      iv("e", 250, 350),
    ];
    assertNoSlotCollision(intervals, packIntoLanes(intervals));
  });

  test("a chain of four overlapping intervals opens four distinct lanes", () => {
    const intervals = [
      iv("a", 0, 100),
      iv("b", 10, 110),
      iv("c", 20, 120),
      iv("d", 30, 130),
    ];
    const result = packIntoLanes(intervals);
    expect(new Set(result.map(slotKey)).size).toBe(4);
    assertNoSlotCollision(intervals, result);
  });

  test("greedy degenerates gracefully — a fifth overlap opens a new lane", () => {
    // Best-Fit cannot reuse any occupied lane; the new interval jumps
    // into a freshly-opened lane rather than colliding (D1 rationale 1).
    const intervals = [
      iv("a", 0, 100),
      iv("b", 10, 110),
      iv("c", 20, 120),
      iv("d", 30, 130),
      iv("e", 40, 140),
    ];
    const result = packIntoLanes(intervals);
    const e = result.find((r) => r.vodId === "e")!;
    expect(e).toEqual({ vodId: "e", side: "above", laneIdx: 2 });
    expect(new Set(result.map(slotKey)).size).toBe(5);
    assertNoSlotCollision(intervals, result);
  });

  test("zero-duration intervals (startAt === endAt) pack without error", () => {
    // The DB CHECK permits startAt === endAt; an instantaneous row
    // must still receive a lane assignment.
    const result = packIntoLanes([iv("a", 100, 100), iv("b", 100, 100)]);
    expect(result).toHaveLength(2);
    expect(result.map((r) => r.vodId).sort()).toEqual(["a", "b"]);
  });
});

describe("packIntoLanes — Interval Graph Coloring fallback (ADR-0038 D1)", () => {
  test("coloring fallback also packs four overlaps into four lanes", () => {
    const intervals = [
      iv("a", 0, 100),
      iv("b", 10, 110),
      iv("c", 20, 120),
      iv("d", 30, 130),
    ];
    const result = packIntoLanes(intervals, { strategy: "coloring" });
    expect(new Set(result.map(slotKey)).size).toBe(4);
    assertNoSlotCollision(intervals, result);
  });

  test("coloring fallback is deterministic and collision-free on a mix", () => {
    const intervals = [
      iv("a", 0, 100),
      iv("b", 50, 150),
      iv("c", 120, 180),
      iv("d", 200, 300),
    ];
    const first = packIntoLanes(intervals, { strategy: "coloring" });
    const second = packIntoLanes(intervals, { strategy: "coloring" });
    expect(first).toEqual(second);
    assertNoSlotCollision(intervals, first);
  });
});

describe("multi-perspective grouping (ADR-0038 D5 + CEO-A2)", () => {
  test("the overlap threshold is 50 %, not the PROPOSED 75 %", () => {
    expect(MULTI_PERSPECTIVE_OVERLAP_THRESHOLD).toBe(0.5);
  });

  test("overlapFractionOfShorter measures against the shorter interval", () => {
    // a dur 200, b dur 100, overlap 100 → 100 / 100 (shorter) = 1.0.
    expect(overlapFractionOfShorter(iv("a", 0, 200), iv("b", 50, 150))).toBe(1);
    // disjoint intervals → 0.
    expect(overlapFractionOfShorter(iv("a", 0, 100), iv("b", 200, 300))).toBe(
      0,
    );
  });

  test("intervals overlapping exactly 50 % of the shorter are grouped", () => {
    const a = iv("a", 0, 100, "s1");
    const b = iv("b", 50, 150, "s2");
    expect(overlapFractionOfShorter(a, b)).toBe(0.5);
    const groups = groupMultiPerspective([a, b]);
    expect(groups).toHaveLength(1);
    expect(new Set(groups[0]!.members.map((m) => m.vodId))).toEqual(
      new Set(["a", "b"]),
    );
  });

  test("intervals just under 50 % overlap are NOT grouped", () => {
    const a = iv("a", 0, 100, "s1");
    const b = iv("b", 51, 151, "s2");
    expect(overlapFractionOfShorter(a, b)).toBeLessThan(0.5);
    expect(groupMultiPerspective([a, b])).toHaveLength(0);
  });

  test("the group leads with the longest-running interval (D5 position)", () => {
    const long = iv("long", 0, 400, "s1");
    const short = iv("short", 100, 300, "s2");
    const groups = groupMultiPerspective([short, long]);
    expect(groups).toHaveLength(1);
    expect(groups[0]!.leadVodId).toBe("long");
    expect(groups[0]!.members[0]!.vodId).toBe("long");
  });

  test("overlapping VODs from a single streamer are not multi-perspective", () => {
    const a = iv("a", 0, 100, "s1");
    const b = iv("b", 10, 110, "s1");
    expect(groupMultiPerspective([a, b])).toHaveLength(0);
  });

  test("disjoint intervals produce no groups", () => {
    const a = iv("a", 0, 100, "s1");
    const b = iv("b", 200, 300, "s2");
    expect(groupMultiPerspective([a, b])).toHaveLength(0);
  });

  test("grouping is transitive across the connected component (D5)", () => {
    // A↔B and B↔C each clear 50 %, but A↔C do not — D5 groups the
    // connected component, so C joins via B even though C shares its
    // streamer with A. The stack stays inspectable per D5.
    const a = iv("a", 0, 100, "s1");
    const b = iv("b", 50, 150, "s2");
    const c = iv("c", 80, 180, "s1");
    expect(overlapFractionOfShorter(a, c)).toBeLessThan(0.5);
    const groups = groupMultiPerspective([a, b, c]);
    expect(groups).toHaveLength(1);
    expect(new Set(groups[0]!.members.map((m) => m.vodId))).toEqual(
      new Set(["a", "b", "c"]),
    );
  });
});
