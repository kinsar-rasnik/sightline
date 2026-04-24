import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, test, vi } from "vitest";

import { LibraryPage } from "@/features/vods/LibraryPage";
import type { VodWithChapters } from "@/ipc";

import type * as IpcModule from "@/ipc";

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

  test("status filter chip narrows the query", async () => {
    vi.mocked(commands.listVods).mockResolvedValue([]);
    renderWith(<LibraryPage />);
    await userEvent.click(screen.getByRole("button", { name: /sub-only/i }));
    await waitFor(() => {
      const last = vi.mocked(commands.listVods).mock.calls.at(-1)?.[0];
      expect(last?.filters.statuses).toEqual(["skipped_sub_only"]);
    });
  });

  test("ipc error surfaces in an alert", async () => {
    vi.mocked(commands.listVods).mockRejectedValue(new Error("boom"));
    renderWith(<LibraryPage />);
    await waitFor(() => expect(screen.getByRole("alert")).toBeInTheDocument());
  });
});
