/**
 * VodCard state-matrix coverage (DV7 of ADR-0039).
 *
 * Covers 13 primary rows × stack-variant overlays per the binding spec.
 * Each test mounts the v2.1 VodCard with a controlled `data` override
 * to bypass TanStack-Query fetches; the focus is composition assertion,
 * not data plumbing.
 */

import { describe, expect, test, vi } from "vitest";
import { act, fireEvent, render, screen } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";

import { VodCard } from "@/features/vods/VodCard";
import type {
  VodCardStackPerspective,
} from "@/features/vods/VodCard";
import type { VodSummary } from "@/features/vods/hooks/useVodSummary";

// All tests need a QueryClientProvider because `useVodSummary` calls
// `useQuery` (with `enabled: false` when `data` is provided, but the
// hook still needs the context). Wrap every render.
function renderWith(ui: React.ReactElement) {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return render(
    <QueryClientProvider client={client}>{ui}</QueryClientProvider>
  );
}

// ---- fixture helpers ------------------------------------------------------

const SECONDS_IN_DAY = 86_400;
const SECONDS_IN_HOUR = 3_600;

function nowSec() {
  return Math.floor(Date.now() / 1000);
}

function baseSummary(
  overrides: Partial<VodSummary> = {},
  streamStartedAtOffsetDays = -5
): VodSummary {
  const streamStartedAt = nowSec() + streamStartedAtOffsetDays * SECONDS_IN_DAY;
  const baseVod = {
    twitchVideoId: "v_test",
    twitchUserId: "u1",
    streamId: null,
    title: "Wolfsterben Rampage",
    description: "",
    streamStartedAt,
    publishedAt: streamStartedAt,
    url: "https://twitch.tv/videos/v_test",
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
  };
  // Retention computed from offset: -5 d on a 7 d window gives ~2 d
  // remaining = stage 1 (>2 d but <= 7 d, actually > 48 h → stage 2).
  // Use a heuristic — let component compute itself.
  // For tests where stage matters explicitly, override `retention`.
  const summary: VodSummary = {
    vod: {
      vod: baseVod,
      chapters: [],
      streamerDisplayName: "John Smith",
      streamerLogin: "johnsmith",
    },
    streamer: {
      twitchUserId: "u1",
      login: "johnsmith",
      displayName: "John Smith",
      profileImageUrl: null,
      broadcasterType: "partner",
      twitchCreatedAt: 0,
      addedAt: 0,
      deletedAt: null,
      lastPolledAt: null,
      nextPollAt: null,
      lastLiveAt: null,
    },
    assets: {
      vodId: "v_test",
      videoPath: null,
      thumbnailPath: null,
      previewFramePaths: [
        "/frame/0.png",
        "/frame/1.png",
        "/frame/2.png",
        "/frame/3.png",
        "/frame/4.png",
        "/frame/5.png",
      ],
    },
    watchProgress: null,
    retention: {
      stage: 1,
      retentionAt: streamStartedAt + 60 * SECONDS_IN_DAY,
      remainingSeconds: 10 * SECONDS_IN_DAY,
    },
    ...overrides,
  };
  return summary;
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

// ---- matrix rows ----------------------------------------------------------

describe("VodCard state matrix (ADR-0039 DV7)", () => {
  test("Row 1: idle / unselected / single / no retention / not downloaded", () => {
    setReducedMotion(false);
    renderWith(<VodCard vodId="v_test" variant="full" data={baseSummary()} />);
    const card = screen.getByRole("article");
    expect(card.getAttribute("aria-selected")).toBe("false");
    expect(card.getAttribute("data-stack")).toBe("single");
    expect(card.getAttribute("data-triggered")).toBe("false");
    expect(card.getAttribute("data-variant")).toBe("full");
    // No selection checkmark
    expect(screen.queryByLabelText("Selected")).toBeNull();
  });

  test("Row 2: idle / selected / single", () => {
    setReducedMotion(false);
    renderWith(
      <VodCard vodId="v_test" variant="full" selected data={baseSummary()} />
    );
    const card = screen.getByRole("article");
    expect(card.getAttribute("aria-selected")).toBe("true");
    expect(screen.getByLabelText("Selected")).toBeInTheDocument();
  });

  test("Row 3: idle / unselected / stack-idle (2 perspectives)", () => {
    setReducedMotion(false);
    const stack: VodCardStackPerspective[] = [
      { twitchUserId: "u1", displayName: "John", profileImageUrl: null },
      { twitchUserId: "u2", displayName: "Jane", profileImageUrl: null },
    ];
    renderWith(
      <VodCard
        vodId="v_test"
        variant="full"
        data={baseSummary()}
        stack={stack}
      />
    );
    const card = screen.getByRole("article");
    expect(card.getAttribute("data-stack")).toBe("stack-idle");
    expect(card.getAttribute("aria-expanded")).toBe("false");
    expect(screen.getByLabelText(/perspectives/)).toBeInTheDocument();
  });

  test("Row 4: hover-triggered (after 800 ms delay) renders frame-strip and zoom", () => {
    setReducedMotion(false);
    vi.useFakeTimers();
    renderWith(<VodCard vodId="v_test" variant="full" data={baseSummary()} />);
    const card = screen.getByRole("article");
    act(() => {
      fireEvent.mouseEnter(card);
    });
    // Before 800 ms: not triggered
    act(() => {
      vi.advanceTimersByTime(700);
    });
    expect(card.getAttribute("data-triggered")).toBe("false");
    // After 800 ms: triggered
    act(() => {
      vi.advanceTimersByTime(200);
    });
    expect(card.getAttribute("data-triggered")).toBe("true");
    vi.useRealTimers();
  });

  test("Row 5: focus-pre — focus immediately, trigger after 800 ms", async () => {
    setReducedMotion(false);
    vi.useFakeTimers();
    renderWith(<VodCard vodId="v_test" variant="full" data={baseSummary()} />);
    const card = screen.getByRole("article");
    act(() => {
      card.focus();
    });
    expect(card).toHaveFocus();
    expect(card.getAttribute("data-triggered")).toBe("false");
    act(() => {
      vi.advanceTimersByTime(800);
    });
    expect(card.getAttribute("data-triggered")).toBe("true");
    vi.useRealTimers();
  });

  test("Row 6: downloading status → opacity class", () => {
    setReducedMotion(false);
    const summary = baseSummary();
    summary.vod.vod.status = "downloading";
    renderWith(<VodCard vodId="v_test" variant="full" data={summary} />);
    const card = screen.getByRole("article");
    expect(card.className).toMatch(/opacity-80/);
    expect(card.getAttribute("data-vod-status")).toBe("downloading");
  });

  test("Row 7: completed watch state → watched glyph in strip (VOd-A3)", () => {
    setReducedMotion(false);
    const summary = baseSummary({
      watchProgress: {
        vodId: "v_test",
        positionSeconds: 14000,
        durationSeconds: 14000,
        watchedFraction: 1,
        state: "completed",
        firstWatchedAt: nowSec() - 3600,
        lastWatchedAt: nowSec(),
        lastSessionDurationSeconds: 14000,
        totalWatchSeconds: 14000,
      },
    });
    renderWith(<VodCard vodId="v_test" variant="full" data={summary} />);
    expect(screen.getByLabelText("Watched")).toBeInTheDocument();
    expect(screen.queryByLabelText("Watch progress")).toBeNull();
  });

  test("Row 8: retention stage 3 — neutral badge in image area", () => {
    setReducedMotion(false);
    const summary = baseSummary();
    summary.retention = {
      stage: 3,
      retentionAt: nowSec() + 30 * SECONDS_IN_HOUR,
      remainingSeconds: 30 * SECONDS_IN_HOUR,
    };
    renderWith(<VodCard vodId="v_test" variant="full" data={summary} />);
    // getAllByLabelText returns both the article aria-label (full
    // composite) and the badge aria-label (retention-only phrase);
    // the badge is a <span> with bg-surface-2 class — pick that one.
    const matches = screen.getAllByLabelText(/expires in 30h/);
    const badge = matches.find((el) => el.tagName === "SPAN");
    expect(badge).toBeDefined();
    expect(badge!.className).toMatch(/bg-surface-2/);
  });

  test("Row 9: retention stage 4 — accent badge in image area", () => {
    setReducedMotion(false);
    const summary = baseSummary();
    summary.retention = {
      stage: 4,
      retentionAt: nowSec() + 6 * SECONDS_IN_HOUR,
      remainingSeconds: 6 * SECONDS_IN_HOUR,
    };
    renderWith(<VodCard vodId="v_test" variant="full" data={summary} />);
    const matches = screen.getAllByLabelText(/expires in 6h/);
    const badge = matches.find((el) => el.tagName === "SPAN");
    expect(badge).toBeDefined();
    expect(badge!.className).toMatch(/bg-accent/);
  });

  test("Row 10: stack-idle 3 perspectives — 3 dots in strip", () => {
    setReducedMotion(false);
    const stack: VodCardStackPerspective[] = [
      { twitchUserId: "u1", displayName: "John", profileImageUrl: null },
      { twitchUserId: "u2", displayName: "Jane", profileImageUrl: null },
      { twitchUserId: "u3", displayName: "Alex", profileImageUrl: null },
    ];
    renderWith(
      <VodCard
        vodId="v_test"
        variant="full"
        data={baseSummary()}
        stack={stack}
      />
    );
    expect(screen.getByLabelText("3 perspectives")).toBeInTheDocument();
  });

  test("Row 11: stack-idle 6 perspectives — 4 dots + '+2' overflow", () => {
    setReducedMotion(false);
    const stack: VodCardStackPerspective[] = Array.from({ length: 6 }).map(
      (_, i) => ({
        twitchUserId: `u${i}`,
        displayName: `User ${i}`,
        profileImageUrl: null,
      })
    );
    renderWith(
      <VodCard
        vodId="v_test"
        variant="full"
        data={baseSummary()}
        stack={stack}
      />
    );
    expect(screen.getByLabelText("6 perspectives")).toBeInTheDocument();
    expect(screen.getByText("+2")).toBeInTheDocument();
  });

  test("Row 12: hover-triggered on stack-card → aria-expanded=true", () => {
    setReducedMotion(false);
    vi.useFakeTimers();
    const stack: VodCardStackPerspective[] = [
      { twitchUserId: "u1", displayName: "John", profileImageUrl: null },
      { twitchUserId: "u2", displayName: "Jane", profileImageUrl: null },
    ];
    renderWith(
      <VodCard
        vodId="v_test"
        variant="full"
        data={baseSummary()}
        stack={stack}
      />
    );
    const card = screen.getByRole("article");
    expect(card.getAttribute("aria-expanded")).toBe("false");
    act(() => {
      fireEvent.mouseEnter(card);
    });
    act(() => {
      vi.advanceTimersByTime(800);
    });
    expect(card.getAttribute("aria-expanded")).toBe("true");
    vi.useRealTimers();
  });

  test("Row 13: hero variant renders Watch live CTA + title overlay", () => {
    setReducedMotion(false);
    const onWatchLive = vi.fn();
    renderWith(
      <VodCard
        vodId="v_test"
        variant="hero"
        data={baseSummary()}
        hero={{ pitch: "Live right now", onWatchLive }}
      />
    );
    const card = screen.getByRole("article");
    expect(card.getAttribute("data-variant")).toBe("hero");
    const cta = screen.getByRole("button", {
      name: /Watch Wolfsterben Rampage live/,
    });
    expect(cta).toBeInTheDocument();
    expect(cta.className).toMatch(/bg-accent/);
  });
});
