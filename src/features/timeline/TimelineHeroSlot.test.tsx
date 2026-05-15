/**
 * TimelineHeroSlot tests — the D9 / TV-A6 hero-streamer selection across
 * its four state rows, plus the TV10 total-collapse behaviour.
 *
 * D9 trigger is favourited AND live (R-ADR-01: the IPC fields are
 * `streamer.favorite` + `liveNow`, not the brief's `is_favourited` /
 * `is_live`). Ties break to the most-recently-started stream (TV-A6).
 */

import { afterEach, describe, expect, test, vi } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";

import type { Interval, StreamerSummary } from "@/ipc";

import { selectHeroStreamer, TimelineHeroSlot } from "./TimelineHeroSlot";

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

function mkStreamer(
  id: string,
  opts: { favorite?: boolean; liveNow: boolean; lastLiveAt: number | null },
): StreamerSummary {
  return {
    streamer: {
      twitchUserId: id,
      login: id,
      displayName: id,
      profileImageUrl: null,
      broadcasterType: "partner",
      twitchCreatedAt: 0,
      addedAt: 0,
      deletedAt: null,
      lastPolledAt: null,
      nextPollAt: null,
      lastLiveAt: opts.lastLiveAt,
      favorite: opts.favorite,
    },
    vodCount: 0,
    eligibleVodCount: 0,
    liveNow: opts.liveNow,
    nextPollEtaSeconds: null,
  };
}

function mkInterval(vodId: string, streamerId: string, startAt: number): Interval {
  return { vodId, streamerId, startAt, endAt: startAt + 3600 };
}

afterEach(cleanup);

describe("selectHeroStreamer — D9 four state rows", () => {
  test("row 1: no favourited streamer → no hero", () => {
    const streamers = [mkStreamer("a", { liveNow: true, lastLiveAt: 5 })];
    expect(selectHeroStreamer(streamers)).toBeNull();
  });

  test("row 2: favourited but not live → no hero", () => {
    const streamers = [
      mkStreamer("a", { favorite: true, liveNow: false, lastLiveAt: 5 }),
    ];
    expect(selectHeroStreamer(streamers)).toBeNull();
  });

  test("row 3: one favourited-and-live → that streamer is the hero", () => {
    const streamers = [
      mkStreamer("a", { favorite: true, liveNow: true, lastLiveAt: 5 }),
    ];
    expect(selectHeroStreamer(streamers)?.streamer.twitchUserId).toBe("a");
  });

  test("row 4: many favourited-and-live → most-recently-started wins", () => {
    const streamers = [
      mkStreamer("a", { favorite: true, liveNow: true, lastLiveAt: 10 }),
      mkStreamer("b", { favorite: true, liveNow: true, lastLiveAt: 99 }),
      mkStreamer("c", { favorite: true, liveNow: true, lastLiveAt: 42 }),
    ];
    expect(selectHeroStreamer(streamers)?.streamer.twitchUserId).toBe("b");
  });
});

describe("TimelineHeroSlot — TV10 total collapse", () => {
  test("renders nothing when no favourited streamer is live", () => {
    const { container } = render(
      <TimelineHeroSlot
        streamers={[mkStreamer("a", { liveNow: true, lastLiveAt: 1 })]}
        intervals={[mkInterval("v1", "a", 100)]}
        onWatchLive={vi.fn()}
      />,
    );
    expect(container.firstChild).toBeNull();
  });

  test("renders nothing when the hero streamer has no interval yet", () => {
    const { container } = render(
      <TimelineHeroSlot
        streamers={[
          mkStreamer("a", { favorite: true, liveNow: true, lastLiveAt: 1 }),
        ]}
        intervals={[]}
        onWatchLive={vi.fn()}
      />,
    );
    expect(container.firstChild).toBeNull();
  });

  test("reserves the slot when a favourited streamer is live", () => {
    render(
      <TimelineHeroSlot
        streamers={[
          mkStreamer("a", { favorite: true, liveNow: true, lastLiveAt: 1 }),
        ]}
        intervals={[mkInterval("v_hero", "a", 500)]}
        onWatchLive={vi.fn()}
      />,
    );
    expect(screen.getByTestId("timeline-hero-slot")).toBeInTheDocument();
    expect(screen.getByRole("article").getAttribute("data-variant")).toBe(
      "hero",
    );
  });
});
