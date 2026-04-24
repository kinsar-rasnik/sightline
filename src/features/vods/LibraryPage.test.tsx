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
    },
  };
});

import { commands } from "@/ipc";

function renderWith(ui: React.ReactElement) {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  return render(<QueryClientProvider client={client}>{ui}</QueryClientProvider>);
}

function stubVod(overrides: Partial<VodWithChapters["vod"]> = {}): VodWithChapters {
  return {
    vod: {
      twitch_video_id: "v1",
      twitch_user_id: "100",
      stream_id: null,
      title: "Fake VOD",
      description: "",
      stream_started_at: 1_700_000_000,
      published_at: 1_700_000_000,
      url: "https://twitch.tv/videos/v1",
      thumbnail_url: null,
      duration_seconds: 3600,
      view_count: 10,
      language: "en",
      muted_segments: [],
      is_sub_only: false,
      helix_game_id: null,
      helix_game_name: null,
      ingest_status: "eligible",
      status_reason: "",
      first_seen_at: 1_700_000_000,
      last_seen_at: 1_700_000_000,
      ...overrides,
    },
    chapters: [
      {
        position_ms: 0,
        duration_ms: 1_800_000,
        game_id: "32982",
        game_name: "Grand Theft Auto V",
        chapter_type: "GAME_CHANGE",
      },
    ],
    streamer_display_name: "Sampler",
    streamer_login: "sampler",
  };
}

describe("LibraryPage", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(commands.listStreamers).mockResolvedValue([]);
  });

  test("shows empty state when no VODs match", async () => {
    vi.mocked(commands.listVods).mockResolvedValue([]);
    renderWith(<LibraryPage />);
    await waitFor(() =>
      expect(screen.getByText(/no vods match these filters/i)).toBeInTheDocument()
    );
  });

  test("renders VOD rows with title + streamer + chapter count", async () => {
    vi.mocked(commands.listVods).mockResolvedValue([stubVod()]);
    renderWith(<LibraryPage />);
    await waitFor(() =>
      expect(screen.getByText("Fake VOD")).toBeInTheDocument()
    );
    expect(screen.getAllByText(/Sampler/i).length).toBeGreaterThan(0);
    expect(screen.getByText(/1 chapter/i)).toBeInTheDocument();
  });

  test("clicking a row opens the detail drawer", async () => {
    vi.mocked(commands.listVods).mockResolvedValue([stubVod()]);
    renderWith(<LibraryPage />);
    await waitFor(() =>
      expect(screen.getByText("Fake VOD")).toBeInTheDocument()
    );
    await userEvent.click(screen.getByRole("button", { name: /fake vod/i }));
    await waitFor(() =>
      expect(screen.getByText(/Grand Theft Auto V/i)).toBeInTheDocument()
    );
    expect(screen.getByRole("link", { name: /open on twitch/i })).toBeInTheDocument();
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
