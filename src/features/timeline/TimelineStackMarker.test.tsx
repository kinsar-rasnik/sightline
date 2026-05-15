/**
 * TimelineStackMarker tests — multi-perspective stack rendering and D7
 * group bulk-select.
 *
 * The 50 % grouping threshold itself lives in `groupMultiPerspective`
 * (covered exhaustively in `lane-packing.test.ts`); this file confirms
 * the threshold's downstream consumer renders a stack and that a
 * modifier click bulk-adds every member.
 */

import { afterEach, describe, expect, test, vi } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";

import type { VodCardStackPerspective } from "@/features/vods/VodCard";
import { useTimelineMultiselectStore } from "@/stores/timeline-multiselect-store";

import {
  groupMultiPerspective,
  MULTI_PERSPECTIVE_OVERLAP_THRESHOLD,
} from "./lane-packing";
import { TimelineStackMarker } from "./TimelineStackMarker";

vi.mock("@/features/vods/hooks/useVodSummary", () => {
  function makeSummary(vodId: string) {
    return {
      vod: {
        vod: {
          twitchVideoId: vodId,
          twitchUserId: `u_${vodId}`,
          title: `VOD ${vodId}`,
          status: "ready",
          durationSeconds: 7200,
          streamStartedAt: 1_700_000_000,
          thumbnailUrl: null,
        },
        chapters: [],
        streamerDisplayName: `Streamer ${vodId}`,
        streamerLogin: vodId,
      },
      streamer: null,
      assets: null,
      watchProgress: null,
      retention: { stage: 1, retentionAt: 0, remainingSeconds: 999_999 },
    };
  }
  return {
    useVodSummary: (vodId: string | null) => ({
      data: vodId === null ? null : makeSummary(vodId),
      isLoading: false,
      error: null,
    }),
  };
});

const PERSPECTIVES: VodCardStackPerspective[] = [
  { twitchUserId: "s1", displayName: "Streamer One", profileImageUrl: null },
  { twitchUserId: "s2", displayName: "Streamer Two", profileImageUrl: null },
];

afterEach(() => {
  cleanup();
  useTimelineMultiselectStore.getState().clear();
});

describe("TimelineStackMarker rendering", () => {
  test("renders a stack card carrying every perspective as a dot", () => {
    render(
      <TimelineStackMarker
        leadVodId="lead"
        memberVodIds={["lead", "other"]}
        perspectives={PERSPECTIVES}
        widthPx={240}
        offsetPx={0}
        topPx={0}
        onActivate={vi.fn()}
        onToggleSelectGroup={vi.fn()}
      />,
    );
    const card = screen.getByRole("article");
    expect(card.getAttribute("data-stack")).toBe("stack-idle");
    expect(screen.getByLabelText("2 perspectives")).toBeInTheDocument();
  });

  test("only forms above the D5 / CEO-A2 50 % overlap threshold", () => {
    expect(MULTI_PERSPECTIVE_OVERLAP_THRESHOLD).toBe(0.5);
    // A pair overlapping exactly 50 % of the shorter interval groups.
    const grouped = groupMultiPerspective([
      { vodId: "lead", streamerId: "s1", startAt: 0, endAt: 100 },
      { vodId: "other", streamerId: "s2", startAt: 50, endAt: 150 },
    ]);
    expect(grouped).toHaveLength(1);
  });
});

describe("TimelineStackMarker D7 group bulk-select", () => {
  test("a modifier click adds every member of the group", () => {
    const onToggleSelectGroup = vi.fn();
    const onActivate = vi.fn();
    render(
      <TimelineStackMarker
        leadVodId="lead"
        memberVodIds={["lead", "other"]}
        perspectives={PERSPECTIVES}
        widthPx={240}
        offsetPx={0}
        topPx={0}
        onActivate={onActivate}
        onToggleSelectGroup={onToggleSelectGroup}
      />,
    );
    fireEvent.click(screen.getByRole("button", { name: /Open details/ }), {
      metaKey: true,
    });
    expect(onToggleSelectGroup).toHaveBeenCalledWith(["lead", "other"]);
    expect(onActivate).not.toHaveBeenCalled();
  });
});
