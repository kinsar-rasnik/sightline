/**
 * TimelineCard tests — width→variant selection (TV-A2) and D7 click
 * routing (plain click activates, modifier click toggles selection).
 *
 * `useVodSummary` is mocked so the wrapped VodCard renders its loaded
 * state deterministically without touching the IPC layer.
 */

import { afterEach, describe, expect, test, vi } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";

import { useTimelineMultiselectStore } from "@/stores/timeline-multiselect-store";

import { CARD_WIDTH_COMPACT_PX, CARD_WIDTH_FULL_PX, TimelineCard, variantForWidth } from "./TimelineCard";

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

afterEach(() => {
  cleanup();
  useTimelineMultiselectStore.getState().clear();
});

describe("variantForWidth (TV-A2 thresholds)", () => {
  test("width ≥ 160 px → full", () => {
    expect(variantForWidth(CARD_WIDTH_FULL_PX)).toBe("full");
    expect(variantForWidth(400)).toBe("full");
  });

  test("96–160 px → compact", () => {
    expect(variantForWidth(CARD_WIDTH_COMPACT_PX)).toBe("compact");
    expect(variantForWidth(CARD_WIDTH_FULL_PX - 1)).toBe("compact");
  });

  test("width < 96 px → sliver", () => {
    expect(variantForWidth(CARD_WIDTH_COMPACT_PX - 1)).toBe("sliver");
    expect(variantForWidth(0)).toBe("sliver");
  });
});

describe("TimelineCard rendering", () => {
  test("a wide interval renders the full variant", () => {
    render(
      <TimelineCard
        vodId="v1"
        widthPx={240}
        offsetPx={0}
        topPx={0}
        onActivate={vi.fn()}
        onToggleSelect={vi.fn()}
      />,
    );
    expect(screen.getByRole("article").getAttribute("data-variant")).toBe(
      "full",
    );
  });

  test("a narrow interval renders the sliver variant", () => {
    render(
      <TimelineCard
        vodId="v2"
        widthPx={40}
        offsetPx={0}
        topPx={0}
        onActivate={vi.fn()}
        onToggleSelect={vi.fn()}
      />,
    );
    expect(
      screen.getByRole("button").getAttribute("data-variant"),
    ).toBe("sliver");
  });

  test("the card is positioned with a translateX offset (D6)", () => {
    const { container } = render(
      <TimelineCard
        vodId="v3"
        widthPx={200}
        offsetPx={320}
        topPx={80}
        onActivate={vi.fn()}
        onToggleSelect={vi.fn()}
      />,
    );
    const wrapper = container.querySelector<HTMLElement>(
      '[data-timeline-card="v3"]',
    );
    expect(wrapper?.style.transform).toBe("translateX(320px)");
    expect(wrapper?.style.top).toBe("80px");
  });
});

describe("TimelineCard D7 click routing", () => {
  test("a plain click activates the VOD", () => {
    const onActivate = vi.fn();
    const onToggleSelect = vi.fn();
    render(
      <TimelineCard
        vodId="v1"
        widthPx={200}
        offsetPx={0}
        topPx={0}
        onActivate={onActivate}
        onToggleSelect={onToggleSelect}
      />,
    );
    fireEvent.click(screen.getByRole("button", { name: /Open details/ }));
    expect(onActivate).toHaveBeenCalledWith("v1");
    expect(onToggleSelect).not.toHaveBeenCalled();
  });

  test("a Shift+click toggles selection instead of activating", () => {
    const onActivate = vi.fn();
    const onToggleSelect = vi.fn();
    render(
      <TimelineCard
        vodId="v1"
        widthPx={200}
        offsetPx={0}
        topPx={0}
        onActivate={onActivate}
        onToggleSelect={onToggleSelect}
      />,
    );
    fireEvent.click(screen.getByRole("button", { name: /Open details/ }), {
      shiftKey: true,
    });
    expect(onToggleSelect).toHaveBeenCalledWith("v1");
    expect(onActivate).not.toHaveBeenCalled();
  });
});
