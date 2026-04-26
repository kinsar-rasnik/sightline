import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, test, vi } from "vitest";

import { LibraryPage } from "@/features/vods/LibraryPage";
import type { VodWithChapters } from "@/ipc";

import type * as IpcModule from "@/ipc";

vi.mock("@tauri-apps/api/event", () => ({
  // The Library page subscribes to distribution / download events
  // for cache invalidation. In the test environment there is no
  // Tauri runtime — short-circuit listen() to a no-op unsubscribe.
  listen: vi.fn().mockResolvedValue(() => {}),
}));

vi.mock("@/ipc", async () => {
  const actual = await vi.importActual<typeof IpcModule>("@/ipc");
  return {
    ...actual,
    commands: {
      ...actual.commands,
      listVods: vi.fn(),
      listStreamers: vi.fn(),
      listDownloads: vi.fn().mockResolvedValue([]),
      enqueueDownload: vi.fn(),
      retryDownload: vi.fn(),
      pickVod: vi.fn().mockResolvedValue({ vodId: "v1", status: "queued" }),
      unpickVod: vi
        .fn()
        .mockResolvedValue({ vodId: "v1", status: "available" }),
      getVodAssets: vi.fn().mockResolvedValue({
        vodId: "v1",
        videoPath: null,
        thumbnailPath: null,
        previewFramePaths: [],
      }),
    },
  };
});

import { commands } from "@/ipc";

function renderWith(ui: React.ReactElement) {
  const client = new QueryClient({
    defaultOptions: {
      queries: { retry: false, refetchInterval: false },
      mutations: { retry: false },
    },
  });
  return render(<QueryClientProvider client={client}>{ui}</QueryClientProvider>);
}

function stubVod(overrides: Partial<VodWithChapters["vod"]> = {}): VodWithChapters {
  return {
    vod: {
      twitchVideoId: "v1",
      twitchUserId: "100",
      streamId: null,
      title: "Fake VOD",
      description: "",
      streamStartedAt: 1_700_000_000,
      publishedAt: 1_700_000_000,
      url: "https://twitch.tv/videos/v1",
      thumbnailUrl: null,
      durationSeconds: 3600,
      viewCount: 10,
      language: "en",
      mutedSegments: [],
      isSubOnly: false,
      helixGameId: null,
      helixGameName: null,
      ingestStatus: "eligible",
      statusReason: "",
      firstSeenAt: 1_700_000_000,
      lastSeenAt: 1_700_000_000,
      status: "available",
      ...overrides,
    },
    chapters: [
      {
        positionMs: 0,
        durationMs: 1_800_000,
        gameId: "32982",
        gameName: "Grand Theft Auto V",
        chapterType: "GAME_CHANGE",
      },
    ],
    streamerDisplayName: "Sampler",
    streamerLogin: "sampler",
  };
}

describe("LibraryPage", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(commands.listStreamers).mockResolvedValue([]);
    vi.mocked(commands.getVodAssets).mockResolvedValue({
      vodId: "v1",
      videoPath: null,
      thumbnailPath: null,
      previewFramePaths: [],
    });
  });

  test("shows empty state when no VODs match", async () => {
    vi.mocked(commands.listVods).mockResolvedValue([]);
    renderWith(<LibraryPage />);
    await waitFor(() =>
      expect(
        screen.getByText(/no vods match these filters/i)
      ).toBeInTheDocument()
    );
  });

  test("renders a VOD card with title and streamer", async () => {
    vi.mocked(commands.listVods).mockResolvedValue([stubVod()]);
    renderWith(<LibraryPage />);
    await waitFor(() =>
      expect(screen.getByText("Fake VOD")).toBeInTheDocument()
    );
    expect(screen.getAllByText(/Sampler/i).length).toBeGreaterThan(0);
  });

  test("clicking a card opens the detail drawer with chapters", async () => {
    vi.mocked(commands.listVods).mockResolvedValue([stubVod()]);
    renderWith(<LibraryPage />);
    await waitFor(() =>
      expect(screen.getByText("Fake VOD")).toBeInTheDocument()
    );
    await userEvent.click(
      screen.getByRole("button", { name: /open details for fake vod/i })
    );
    await waitFor(() =>
      expect(screen.getByText(/Grand Theft Auto V/i)).toBeInTheDocument()
    );
    expect(
      screen.getByRole("link", { name: /open on twitch/i })
    ).toBeInTheDocument();
  });

  test("lifecycle filter chips drive client-side filtering by vods.status", async () => {
    // ADR-0033: lifecycle filters operate on the already-fetched
    // VodWithChapters list; the listVods query is NOT re-narrowed
    // (the entire library is fetched once, filtered client-side).
    const available = stubVod({
      twitchVideoId: "v_avail",
      title: "Available VOD",
      status: "available",
    });
    const ready = stubVod({
      twitchVideoId: "v_ready",
      title: "Ready VOD",
      status: "ready",
    });
    const archived = stubVod({
      twitchVideoId: "v_arch",
      title: "Archived VOD",
      status: "archived",
    });
    vi.mocked(commands.listVods).mockResolvedValue([available, ready, archived]);
    renderWith(<LibraryPage />);

    // All three visible under the default "All" chip.
    await waitFor(() =>
      expect(screen.getByText("Available VOD")).toBeInTheDocument()
    );
    expect(screen.getByText("Ready VOD")).toBeInTheDocument();
    expect(screen.getByText("Archived VOD")).toBeInTheDocument();

    // Click "Watched" — only the archived VOD should remain.
    await userEvent.click(screen.getByRole("button", { name: /^watched$/i }));
    await waitFor(() =>
      expect(screen.queryByText("Available VOD")).not.toBeInTheDocument()
    );
    expect(screen.queryByText("Ready VOD")).not.toBeInTheDocument();
    expect(screen.getByText("Archived VOD")).toBeInTheDocument();
  });

  test("clicking Download on an available VOD calls pickVod", async () => {
    const pickVodMock = vi.fn().mockResolvedValue({
      vodId: "v_avail",
      status: "queued",
    });
    vi.mocked(commands).pickVod = pickVodMock;
    vi.mocked(commands.listVods).mockResolvedValue([
      stubVod({
        twitchVideoId: "v_avail",
        title: "Available VOD",
        status: "available",
      }),
    ]);
    renderWith(<LibraryPage />);
    await waitFor(() =>
      expect(screen.getByText("Available VOD")).toBeInTheDocument()
    );
    // The hover-revealed "Download" button is in the DOM regardless
    // of pointer state — the visibility is opacity-driven CSS, not
    // conditional rendering — so userEvent.click finds and triggers
    // it.
    await userEvent.click(
      screen.getByRole("button", { name: /Download Available VOD/i })
    );
    await waitFor(() => expect(pickVodMock).toHaveBeenCalledTimes(1));
    expect(pickVodMock).toHaveBeenCalledWith({ vodId: "v_avail" });
  });

  test("ipc error surfaces in an alert", async () => {
    vi.mocked(commands.listVods).mockRejectedValue(new Error("boom"));
    renderWith(<LibraryPage />);
    await waitFor(() => expect(screen.getByRole("alert")).toBeInTheDocument());
  });
});
