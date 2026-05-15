/**
 * Lane-packing + multi-perspective grouping — pure interval-geometry for
 * the v2.1 Single-Time-Axis timeline (ADR-0038 D1 + D5).
 *
 * Greedy Best-Fit is the primary packing algorithm; Interval Graph
 * Coloring is the opt-in fallback for deep-stack pathology (D1
 * "Fallback for Engineering implementation"). No React / Zustand / IPC
 * imports — this module is consumed via `useMemo` in TimelineLanes and
 * unit-tested in isolation.
 *
 * R-ADR-01 note. ADR-0038 D1's pseudo-code `LaneAssignment` carries a
 * `side` ('above' | 'below') field — load-bearing for §3.6's
 * axis-as-spine layout. The Mission-3 brief sketched the output as
 * `{ intervalId, laneIndex }` (no `side`); the spec source wins, so
 * `LaneAssignment` is `{ vodId, side, laneIdx }` exactly per D1.
 */

export type LaneSide = "above" | "below";

/**
 * Minimal structural shape of a timeline interval. Field-compatible
 * with the IPC `Interval` type — callers pass `Interval[]` directly —
 * without importing it, keeping this module free of IPC coupling.
 */
export interface PackInterval {
  vodId: string;
  streamerId: string;
  /** UTC unix seconds. */
  startAt: number;
  /** UTC unix seconds; the `startAt <= endAt` invariant is a DB CHECK. */
  endAt: number;
}

/**
 * One interval's placement on the time axis: which side, which lane on
 * that side. Lane indices are per-side (ADR-0038 D1 keeps `above` and
 * `below` as independent lane arrays).
 */
export interface LaneAssignment {
  vodId: string;
  side: LaneSide;
  laneIdx: number;
}

export type PackStrategy = "greedy" | "coloring";

export interface PackOptions {
  /**
   * D1: Greedy Best-Fit is the default. `"coloring"` selects the opt-in
   * Interval-Graph-Coloring fallback for deep-stack pathology.
   */
  strategy?: PackStrategy;
}

interface Lane {
  /** `endAt` of the last interval placed on this lane. */
  tailEnd: number;
}

/** Deterministic order: `startAt` ascending, then `vodId` for tie-break. */
function byStartThenVod(a: PackInterval, b: PackInterval): number {
  if (a.startAt !== b.startAt) return a.startAt - b.startAt;
  if (a.vodId < b.vodId) return -1;
  if (a.vodId > b.vodId) return 1;
  return 0;
}

/**
 * Assign every interval to an (`side`, `laneIdx`) slot.
 *
 * Greedy Best-Fit (D1): walk intervals by `startAt` ascending; for each,
 * pick the side with fewer active lanes (keeps the axis visually
 * centred), then first-fit the first lane on that side whose tail
 * precedes the new `startAt`. Touching endpoints (`tailEnd === startAt`)
 * reuse the lane — abutting intervals do not overlap.
 *
 * Stable under streaming updates: a newly-ingested interval is appended
 * to the sort and never shifts an already-placed assignment.
 */
export function packIntoLanes(
  intervals: readonly PackInterval[],
  opts: PackOptions = {},
): LaneAssignment[] {
  if (opts.strategy === "coloring") return packByColoring(intervals);
  return packGreedy(intervals);
}

function packGreedy(intervals: readonly PackInterval[]): LaneAssignment[] {
  const sorted = [...intervals].sort(byStartThenVod);
  const above: Lane[] = [];
  const below: Lane[] = [];
  const result: LaneAssignment[] = [];

  for (const iv of sorted) {
    // Side selection balances side-occupancy so the axis reads centred.
    const side: LaneSide = above.length <= below.length ? "above" : "below";
    const lanes = side === "above" ? above : below;

    const laneIdx = lanes.findIndex((lane) => lane.tailEnd <= iv.startAt);
    if (laneIdx === -1) {
      result.push({ vodId: iv.vodId, side, laneIdx: lanes.length });
      lanes.push({ tailEnd: iv.endAt });
    } else {
      const lane = lanes[laneIdx];
      if (lane) lane.tailEnd = iv.endAt;
      result.push({ vodId: iv.vodId, side, laneIdx });
    }
  }
  return result;
}

/**
 * Interval Graph Coloring fallback (D1). First-Fit colour assignment
 * over the start-sorted sweep is an optimal interval colouring; the
 * stable tie-break is `streamerId` then `vodId`. Colours interleave
 * onto the two sides (even → above, odd → below) so the axis stays
 * centred, mirroring the Greedy variant's output shape.
 */
function packByColoring(intervals: readonly PackInterval[]): LaneAssignment[] {
  const sorted = [...intervals].sort((a, b) => {
    if (a.startAt !== b.startAt) return a.startAt - b.startAt;
    if (a.streamerId !== b.streamerId)
      return a.streamerId < b.streamerId ? -1 : 1;
    return byStartThenVod(a, b);
  });

  const colorTailEnd: number[] = [];
  const result: LaneAssignment[] = [];

  for (const iv of sorted) {
    let color = colorTailEnd.findIndex((tail) => tail <= iv.startAt);
    if (color === -1) {
      color = colorTailEnd.length;
      colorTailEnd.push(iv.endAt);
    } else {
      colorTailEnd[color] = iv.endAt;
    }
    result.push({
      vodId: iv.vodId,
      side: color % 2 === 0 ? "above" : "below",
      laneIdx: Math.floor(color / 2),
    });
  }
  return result;
}

/* ---------------------------------------------------------------------------
 * Multi-perspective grouping (ADR-0038 D5 + CEO-A2).
 * ------------------------------------------------------------------------- */

/**
 * D5 + CEO-A2: two intervals belong to the same multi-perspective
 * moment when their time-overlap is at least 50 % of the shorter
 * interval. CEO-A2 corrected this floor from the PROPOSED 75 % — the
 * 50 % bar is what catches GTA-RP's signature late-join pattern.
 */
export const MULTI_PERSPECTIVE_OVERLAP_THRESHOLD = 0.5;

/**
 * Fraction of the *shorter* interval that overlaps the other interval.
 * `0` when the intervals are disjoint or either has zero duration.
 */
export function overlapFractionOfShorter(
  a: PackInterval,
  b: PackInterval,
): number {
  const overlap = Math.min(a.endAt, b.endAt) - Math.max(a.startAt, b.startAt);
  if (overlap <= 0) return 0;
  const shorter = Math.min(a.endAt - a.startAt, b.endAt - b.startAt);
  if (shorter <= 0) return 0;
  return overlap / shorter;
}

/**
 * A set of intervals covering one moment from multiple streamer
 * perspectives. Renders as a single stack-silhouette card at the
 * longest-running interval's position (D5).
 */
export interface MultiPerspectiveGroup {
  /** `vodId` of the longest-running interval — the stack anchors here. */
  leadVodId: string;
  /** All member intervals, longest-running first. */
  members: PackInterval[];
}

/**
 * Cluster intervals into multi-perspective moments. Two intervals link
 * when {@link overlapFractionOfShorter} ≥ 50 % (D5); groups are the
 * connected components of that relation. A component is a genuine
 * multi-perspective moment only when it spans ≥ 2 intervals from ≥ 2
 * distinct streamers — a single streamer's own overlapping VODs are not
 * a second perspective. (All timeline intervals are from the user's
 * tracked set by construction, so D5's tracked-set guard is implicit.)
 */
export function groupMultiPerspective(
  intervals: readonly PackInterval[],
): MultiPerspectiveGroup[] {
  const list = [...intervals];
  const n = list.length;
  const visited = new Array<boolean>(n).fill(false);
  const groups: MultiPerspectiveGroup[] = [];

  for (let i = 0; i < n; i++) {
    if (visited[i]) continue;
    visited[i] = true;
    const stack = [i];
    const component: PackInterval[] = [];

    while (stack.length > 0) {
      const idx = stack.pop();
      if (idx === undefined) continue;
      const current = list[idx];
      if (!current) continue;
      component.push(current);

      for (let j = 0; j < n; j++) {
        if (visited[j]) continue;
        const other = list[j];
        if (!other) continue;
        if (
          overlapFractionOfShorter(current, other) >=
          MULTI_PERSPECTIVE_OVERLAP_THRESHOLD
        ) {
          visited[j] = true;
          stack.push(j);
        }
      }
    }

    if (component.length < 2) continue;
    const streamers = new Set(component.map((iv) => iv.streamerId));
    if (streamers.size < 2) continue;

    // `component` is in DFS post-order; this longest-first sort is
    // load-bearing — `leadVodId` is `members[0]` (D5: the stack anchors
    // at the longest-running interval). The `vodId` tie-break keeps the
    // lead deterministic when two members share a duration.
    const members = component.sort((a, b) => {
      const byDuration = b.endAt - b.startAt - (a.endAt - a.startAt);
      if (byDuration !== 0) return byDuration;
      if (a.vodId < b.vodId) return -1;
      if (a.vodId > b.vodId) return 1;
      return 0;
    });
    const lead = members[0];
    if (!lead) continue;
    groups.push({ leadVodId: lead.vodId, members });
  }
  return groups;
}
