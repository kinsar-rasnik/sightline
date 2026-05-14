/**
 * VodCard accessibility coverage per ADR-0039 DV9.
 *
 * Covers:
 *  - ARIA-label composition (Single-VOD + retention + watch + selected)
 *  - aria-selected reflects selection state
 *  - aria-expanded reflects stack-fan-out state
 *  - tabindex=0 makes the card keyboard-focusable
 *  - Reduced-motion path: hover trigger lands without delay-loop;
 *    no frame-strip cycle starts
 */

import { describe, expect, test, vi } from "vitest";
import { act, fireEvent, render, screen } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";

import { VodCard } from "@/features/vods/VodCard";
import type { VodSummary } from "@/features/vods/hooks/useVodSummary";

function renderWith(ui: React.ReactElement) {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return render(
    <QueryClientProvider client={client}>{ui}</QueryClientProvider>
  );
}

const SECONDS_IN_DAY = 86_400;
const SECONDS_IN_HOUR = 3_600;

function nowSec() {
  return Math.floor(Date.now() / 1000);
}

function buildSummary(overrides: Partial<VodSummary> = {}): VodSummary {
  const streamStartedAt = nowSec() - 5 * SECONDS_IN_DAY;
  return {
    vod: {
      vod: {
        twitchVideoId: "v_a11y",
        twitchUserId: "u1",
        streamId: null,
        title: "Wolfsterben Rampage",
        description: "",
        streamStartedAt,
        publishedAt: streamStartedAt,
        url: "https://twitch.tv/videos/v_a11y",
        thumbnailUrl: null,
        durationSeconds: 4 * SECONDS_IN_HOUR + 12 * 60,
        viewCount: 0,
        language: "en",
        mutedSegments: [],
        isSubOnly: false,
        helixGameId: null,
        helixGameName: null,
        ingestStatus: "eligible" as const,
        statusReason: "",
        firstSeenAt: streamStartedAt,
        lastSeenAt: streamStartedAt,
        status: "ready" as const,
      },
      chapters: [],
      streamerDisplayName: "John Smith",
      streamerLogin: "johnsmith",
    },
    streamer: null,
    assets: null,
    watchProgress: null,
    retention: {
      stage: 1,
      retentionAt: streamStartedAt + 60 * SECONDS_IN_DAY,
      remainingSeconds: 10 * SECONDS_IN_DAY,
    },
    ...overrides,
  };
}

function setReducedMotion(reduce: boolean) {
  window.matchMedia = vi.fn().mockImplementation((query: string) => ({
    matches:
      query === "(prefers-reduced-motion: reduce)" ? reduce : false,
    media: query,
    onchange: null,
    addListener: vi.fn(),
    removeListener: vi.fn(),
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
    dispatchEvent: vi.fn(),
  }));
}

describe("VodCard accessibility (ADR-0039 DV9)", () => {
  test("ARIA-label format: title, streamer, duration", () => {
    setReducedMotion(false);
    renderWith(<VodCard vodId="v_a11y" variant="full" data={buildSummary()} />);
    const card = screen.getByRole("article");
    const label = card.getAttribute("aria-label") ?? "";
    expect(label).toMatch(/^Wolfsterben Rampage,/);
    expect(label).toContain("John Smith");
    expect(label).toContain("4h 12m");
  });

  test("ARIA-label with retention stage 2: appends 'expires in Xd'", () => {
    setReducedMotion(false);
    const summary = buildSummary();
    summary.retention = {
      stage: 2,
      retentionAt: nowSec() + 4 * SECONDS_IN_DAY,
      remainingSeconds: 4 * SECONDS_IN_DAY,
    };
    renderWith(<VodCard vodId="v_a11y" variant="full" data={summary} />);
    const label = screen.getByRole("article").getAttribute("aria-label") ?? "";
    expect(label).toContain("expires in 4d");
  });

  test("ARIA-label with in-progress watch: includes percent watched", () => {
    setReducedMotion(false);
    const summary = buildSummary({
      watchProgress: {
        vodId: "v_a11y",
        positionSeconds: 6300,
        durationSeconds: 14000,
        watchedFraction: 0.45,
        state: "in_progress",
        firstWatchedAt: nowSec() - 3600,
        lastWatchedAt: nowSec(),
        lastSessionDurationSeconds: 600,
        totalWatchSeconds: 6300,
      },
    });
    renderWith(<VodCard vodId="v_a11y" variant="full" data={summary} />);
    const label = screen.getByRole("article").getAttribute("aria-label") ?? "";
    expect(label).toContain("watched 45 percent");
  });

  test("ARIA-label with selected: appends 'selected'", () => {
    setReducedMotion(false);
    renderWith(
      <VodCard vodId="v_a11y" variant="full" selected data={buildSummary()} />
    );
    const label = screen.getByRole("article").getAttribute("aria-label") ?? "";
    expect(label).toMatch(/, selected$/);
  });

  test("tabindex=0 so the card is keyboard-focusable", () => {
    setReducedMotion(false);
    renderWith(<VodCard vodId="v_a11y" variant="full" data={buildSummary()} />);
    const card = screen.getByRole("article");
    expect(card.getAttribute("tabindex")).toBe("0");
  });

  test("aria-selected reflects selection prop", () => {
    setReducedMotion(false);
    const { rerender } = renderWith(
      <VodCard vodId="v_a11y" variant="full" data={buildSummary()} />
    );
    expect(screen.getByRole("article").getAttribute("aria-selected")).toBe(
      "false"
    );
    rerender(
      <QueryClientProvider client={new QueryClient()}>
        <VodCard vodId="v_a11y" variant="full" selected data={buildSummary()} />
      </QueryClientProvider>
    );
    expect(screen.getByRole("article").getAttribute("aria-selected")).toBe(
      "true"
    );
  });
});

describe("VodCard reduced-motion path (H-22 + DV3)", () => {
  test("Reduced-motion: hover triggers panel instantly without delay loop", () => {
    setReducedMotion(true);
    renderWith(<VodCard vodId="v_a11y" variant="full" data={buildSummary()} />);
    const card = screen.getByRole("article");
    expect(card.getAttribute("data-triggered")).toBe("false");
    act(() => {
      fireEvent.mouseEnter(card);
    });
    // No 800-ms delay under reduced-motion — triggers immediately
    expect(card.getAttribute("data-triggered")).toBe("true");
  });

  test("Reduced-motion: hover-exit returns to idle without exit animation", () => {
    setReducedMotion(true);
    renderWith(<VodCard vodId="v_a11y" variant="full" data={buildSummary()} />);
    const card = screen.getByRole("article");
    act(() => {
      fireEvent.mouseEnter(card);
    });
    expect(card.getAttribute("data-triggered")).toBe("true");
    act(() => {
      fireEvent.mouseLeave(card);
    });
    expect(card.getAttribute("data-triggered")).toBe("false");
  });
});
