import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, test, vi } from "vitest";

import { StreamersPage } from "@/features/streamers/StreamersPage";
import type { PollStatusRow, StreamerSummary } from "@/ipc";
import { useActivePollsStore } from "@/stores/active-polls-store";

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
      twitchUserId: "100",
      login: "sampler",
      displayName: "Sampler",
      profileImageUrl: null,
      broadcasterType: "",
      twitchCreatedAt: 0,
      addedAt: 1_700_000_000,
      deletedAt: null,
      lastPolledAt: 1_700_000_100,
      nextPollAt: 1_700_000_700,
      lastLiveAt: null,
      favorite: false,
      ...overrides,
    },
    vodCount: 10,
    eligibleVodCount: 4,
    liveNow: false,
    nextPollEtaSeconds: 600,
  };
}

describe("StreamersPage", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(commands.getPollStatus).mockResolvedValue([]);
    useActivePollsStore.getState().clear();
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
      lastPoll: {
        startedAt: 1_700_000_000,
        finishedAt: 1_700_000_100,
        vodsNew: 1,
        vodsUpdated: 2,
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

  test("shows a polling indicator when active-polls store has this streamer", async () => {
    vi.mocked(commands.listStreamers).mockResolvedValue([stubStreamer()]);
    renderWith(<StreamersPage />);
    await waitFor(() =>
      expect(screen.getByText("Sampler")).toBeInTheDocument()
    );

    // No indicator initially.
    expect(screen.queryByRole("status", { name: /polling/i })).toBeNull();

    // Flip the store — mimics `poll:started` arriving from the backend.
    act(() => {
      useActivePollsStore.getState().add("100");
    });
    expect(
      await screen.findByRole("status", { name: /polling/i })
    ).toBeInTheDocument();

    // `poll:finished` clears it again.
    act(() => {
      useActivePollsStore.getState().remove("100");
    });
    await waitFor(() =>
      expect(screen.queryByRole("status", { name: /polling/i })).toBeNull()
    );
  });
});
