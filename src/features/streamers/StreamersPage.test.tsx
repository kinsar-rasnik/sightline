import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, test, vi } from "vitest";

import { StreamersPage } from "@/features/streamers/StreamersPage";
import type { PollStatusRow, StreamerSummary } from "@/ipc";

import type * as IpcModule from "@/ipc";

vi.mock("@/ipc", async () => {
  const actual = await vi.importActual<typeof IpcModule>("@/ipc");
  return {
    ...actual,
    commands: {
      ...actual.commands,
      listStreamers: vi.fn(),
      addStreamer: vi.fn(),
      removeStreamer: vi.fn(),
      getPollStatus: vi.fn(),
      triggerPoll: vi.fn(),
    },
  };
});

import { commands } from "@/ipc";

function renderWith(ui: React.ReactElement) {
  const client = new QueryClient({
    defaultOptions: {
      queries: { retry: false },
      mutations: { retry: false },
    },
  });
  return render(<QueryClientProvider client={client}>{ui}</QueryClientProvider>);
}

function stubStreamer(overrides: Partial<StreamerSummary["streamer"]> = {}): StreamerSummary {
  return {
    streamer: {
      twitch_user_id: "100",
      login: "sampler",
      display_name: "Sampler",
      profile_image_url: null,
      broadcaster_type: "",
      twitch_created_at: 0,
      added_at: 1_700_000_000,
      deleted_at: null,
      last_polled_at: 1_700_000_100,
      next_poll_at: 1_700_000_700,
      last_live_at: null,
      ...overrides,
    },
    vod_count: 10,
    eligible_vod_count: 4,
    live_now: false,
    next_poll_eta_seconds: 600,
  };
}

describe("StreamersPage", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(commands.getPollStatus).mockResolvedValue([]);
  });

  test("empty state message when no streamers are registered", async () => {
    vi.mocked(commands.listStreamers).mockResolvedValue([]);
    renderWith(<StreamersPage />);
    await waitFor(() =>
      expect(screen.getByText(/no streamers yet/i)).toBeInTheDocument()
    );
  });

  test("shows the list with per-row counts once loaded", async () => {
    vi.mocked(commands.listStreamers).mockResolvedValue([stubStreamer()]);
    renderWith(<StreamersPage />);
    await waitFor(() =>
      expect(screen.getByText("Sampler")).toBeInTheDocument()
    );
    expect(screen.getByText("10 VODs")).toBeInTheDocument();
    expect(screen.getByText("4 eligible")).toBeInTheDocument();
  });

  test("add form validates empty input locally", async () => {
    vi.mocked(commands.listStreamers).mockResolvedValue([]);
    renderWith(<StreamersPage />);
    await waitFor(() =>
      expect(screen.getByRole("button", { name: /add/i })).toBeInTheDocument()
    );
    await userEvent.click(screen.getByRole("button", { name: /add/i }));
    expect(await screen.findByText(/login is required/i)).toBeInTheDocument();
    expect(vi.mocked(commands.addStreamer)).not.toHaveBeenCalled();
  });

  test("add flow sends the login to the backend", async () => {
    vi.mocked(commands.listStreamers).mockResolvedValue([]);
    vi.mocked(commands.addStreamer).mockResolvedValue(stubStreamer());
    renderWith(<StreamersPage />);
    await userEvent.type(
      screen.getByLabelText(/twitch login/i),
      "sampler"
    );
    await userEvent.click(screen.getByRole("button", { name: /add/i }));
    await waitFor(() =>
      expect(vi.mocked(commands.addStreamer)).toHaveBeenCalledWith({
        login: "sampler",
      })
    );
  });

  test("ipc error surfaces in an alert", async () => {
    vi.mocked(commands.listStreamers).mockRejectedValue(new Error("boom"));
    renderWith(<StreamersPage />);
    await waitFor(() =>
      expect(screen.getByRole("alert")).toBeInTheDocument()
    );
  });

  test("poll status is fetched alongside the list", async () => {
    const row: PollStatusRow = {
      streamer: stubStreamer(),
      last_poll: {
        started_at: 1_700_000_000,
        finished_at: 1_700_000_100,
        vods_new: 1,
        vods_updated: 2,
        status: "ok",
      },
    };
    vi.mocked(commands.listStreamers).mockResolvedValue([stubStreamer()]);
    vi.mocked(commands.getPollStatus).mockResolvedValue([row]);
    renderWith(<StreamersPage />);
    await waitFor(() =>
      expect(vi.mocked(commands.getPollStatus)).toHaveBeenCalled()
    );
  });
});
