import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, test, vi } from "vitest";

import {
  statusLabel,
  statusOpacityClass,
  VodCard,
} from "@/features/vods/VodCard";
import type { DownloadRow, VodWithChapters } from "@/ipc";
import type * as IpcModule from "@/ipc";

vi.mock("@/ipc", async () => {
  const actual = await vi.importActual<typeof IpcModule>("@/ipc");
  return {
    ...actual,
    commands: {
      ...actual.commands,
      getVodAssets: vi.fn(),
    },
  };
});

import { commands } from "@/ipc";

function renderWith(ui: React.ReactElement) {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return render(<QueryClientProvider client={client}>{ui}</QueryClientProvider>);
}

function baseRow(): VodWithChapters {
  return {
    vod: {
      twitchVideoId: "v1",
      twitchUserId: "100",
      streamId: null,
      title: "Card Test VOD",
      description: "",
      streamStartedAt: 1_775_088_000,
      publishedAt: 1_775_088_000,
      url: "https://twitch.tv/videos/v1",
      thumbnailUrl: null,
      durationSeconds: 3600,
      viewCount: 0,
      language: "en",
      mutedSegments: [],
      isSubOnly: false,
      helixGameId: null,
      helixGameName: null,
      ingestStatus: "eligible",
      statusReason: "",
      firstSeenAt: 0,
      lastSeenAt: 0,
      status: "available",
    },
    chapters: [],
    streamerDisplayName: "Sampler",
    streamerLogin: "sampler",
  };
}

function completed(): DownloadRow {
  return {
    vodId: "v1",
    state: "completed",
    priority: 100,
    qualityPreset: "source",
    qualityResolved: "source",
    stagingPath: null,
    finalPath: "/library/vod.mp4",
    bytesTotal: 1,
    bytesDone: 1,
    speedBps: null,
    etaSeconds: null,
    attempts: 0,
    lastError: null,
    lastErrorAt: null,
    queuedAt: 0,
    startedAt: 0,
    finishedAt: 1,
    pauseRequested: false,
    title: "Card Test VOD",
    streamerDisplayName: "Sampler",
    streamerLogin: "sampler",
    thumbnailUrl: null,
  };
}

describe("VodCard", () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });
  afterEach(() => {
    vi.useRealTimers();
  });

  test("renders thumbnail placeholder when no asset is loaded yet", () => {
    vi.mocked(commands.getVodAssets).mockResolvedValue({
      vodId: "v1",
      videoPath: null,
      thumbnailPath: null,
      previewFramePaths: [],
    });
    renderWith(
      <VodCard row={baseRow()} download={null} selected={false} onSelect={() => {}} />
    );
    expect(screen.getByText("Card Test VOD")).toBeInTheDocument();
    expect(screen.getByText(/no thumbnail yet/i)).toBeInTheDocument();
  });

  test("renders a progress bar when partial watch progress is supplied", () => {
    vi.mocked(commands.getVodAssets).mockResolvedValue({
      vodId: "v1",
      videoPath: null,
      thumbnailPath: null,
      previewFramePaths: [],
    });
    renderWith(
      <VodCard
        row={baseRow()}
        download={completed()}
        selected={false}
        onSelect={() => {}}
        watchedFraction={0.42}
      />
    );
    const bar = screen.getByRole("progressbar", { name: /watch progress/i });
    expect(bar).toHaveAttribute("aria-valuenow", "42");
  });

  test("hides the progress bar once watched is true", () => {
    vi.mocked(commands.getVodAssets).mockResolvedValue({
      vodId: "v1",
      videoPath: null,
      thumbnailPath: null,
      previewFramePaths: [],
    });
    renderWith(
      <VodCard
        row={baseRow()}
        download={completed()}
        selected={false}
        onSelect={() => {}}
        watchedFraction={1}
        watched
      />
    );
    expect(
      screen.queryByRole("progressbar", { name: /watch progress/i })
    ).toBeNull();
    expect(screen.getByLabelText(/watched/i)).toBeInTheDocument();
  });

  test("shows a download badge when a download row is passed", async () => {
    vi.mocked(commands.getVodAssets).mockResolvedValue({
      vodId: "v1",
      videoPath: null,
      thumbnailPath: null,
      previewFramePaths: [],
    });
    renderWith(
      <VodCard
        row={baseRow()}
        download={completed()}
        selected={false}
        onSelect={() => {}}
      />
    );
    await act(async () => {
      await Promise.resolve();
    });
    // The download badge is suppressed for ready/archived rows
    // (the lifecycle status badge takes precedence) so we test
    // a non-ready row instead.
    const queuedRow = baseRow();
    queuedRow.vod.status = "queued";
    renderWith(
      <VodCard
        row={queuedRow}
        download={{ ...completed(), state: "downloading" }}
        selected={false}
        onSelect={() => {}}
      />
    );
    expect(screen.getByText(/downloading/i)).toBeInTheDocument();
  });
});

describe("statusOpacityClass", () => {
  test("returns the documented tier per ADR-0033 for each status", () => {
    expect(statusOpacityClass("available")).toBe("opacity-60");
    expect(statusOpacityClass("queued")).toBe("opacity-70");
    expect(statusOpacityClass("downloading")).toBe("opacity-80");
    expect(statusOpacityClass("ready")).toBe("opacity-100");
    expect(statusOpacityClass("archived")).toBe("opacity-100");
    expect(statusOpacityClass("deleted")).toBe("opacity-50");
  });
});

describe("statusLabel", () => {
  test("maps each status to a human label", () => {
    expect(statusLabel("available")).toBe("Available");
    expect(statusLabel("queued")).toBe("Queued");
    expect(statusLabel("downloading")).toBe("Downloading");
    expect(statusLabel("ready")).toBe("Ready");
    expect(statusLabel("archived")).toBe("Watched");
    expect(statusLabel("deleted")).toBe("Removed");
  });
});
