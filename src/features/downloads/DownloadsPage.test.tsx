import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, test, vi } from "vitest";

import { DownloadsPage } from "@/features/downloads/DownloadsPage";
import type { DownloadRow, DownloadState } from "@/ipc";
import type * as IpcModule from "@/ipc";

vi.mock("@/ipc", async () => {
  const actual = await vi.importActual<typeof IpcModule>("@/ipc");
  return {
    ...actual,
    commands: {
      ...actual.commands,
      listDownloads: vi.fn(),
      pauseDownload: vi.fn(),
      resumeDownload: vi.fn(),
      cancelDownload: vi.fn(),
      retryDownload: vi.fn(),
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

function stubRow(overrides: Partial<DownloadRow> = {}): DownloadRow {
  return {
    vodId: "v1",
    state: "queued" as DownloadState,
    priority: 100,
    qualityPreset: "source",
    qualityResolved: null,
    stagingPath: null,
    finalPath: null,
    bytesTotal: null,
    bytesDone: 0,
    speedBps: null,
    etaSeconds: null,
    attempts: 0,
    lastError: null,
    lastErrorAt: null,
    queuedAt: 1_700_000_000,
    startedAt: null,
    finishedAt: null,
    pauseRequested: false,
    title: "GTA RP Heist",
    streamerDisplayName: "Sampler",
    streamerLogin: "sampler",
    thumbnailUrl: null,
    ...overrides,
  };
}

describe("DownloadsPage", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  test("shows empty state when nothing queued", async () => {
    vi.mocked(commands.listDownloads).mockResolvedValue([]);
    renderWith(<DownloadsPage />);
    await waitFor(() =>
      expect(screen.getByText(/nothing in the queue yet/i)).toBeInTheDocument()
    );
  });

  test("renders a row per download with its title + streamer", async () => {
    vi.mocked(commands.listDownloads).mockResolvedValue([
      stubRow(),
      stubRow({ vodId: "v2", title: "Second VOD", streamerDisplayName: "Other" }),
    ]);
    renderWith(<DownloadsPage />);
    await waitFor(() =>
      expect(screen.getByText("GTA RP Heist")).toBeInTheDocument()
    );
    expect(screen.getByText("Second VOD")).toBeInTheDocument();
    expect(screen.getAllByText(/Sampler|Other/).length).toBeGreaterThanOrEqual(2);
  });

  test("pause action calls the mutation for a downloading row", async () => {
    vi.mocked(commands.listDownloads).mockResolvedValue([
      stubRow({ state: "downloading", bytesTotal: 1000, bytesDone: 500 }),
    ]);
    vi.mocked(commands.pauseDownload).mockResolvedValue(
      stubRow({ state: "paused" })
    );
    renderWith(<DownloadsPage />);
    const pauseButton = await screen.findByRole("button", { name: /pause/i });
    await userEvent.click(pauseButton);
    await waitFor(() =>
      expect(vi.mocked(commands.pauseDownload)).toHaveBeenCalledWith({
        vodId: "v1",
      })
    );
  });

  test("retry action available for failed rows", async () => {
    vi.mocked(commands.listDownloads).mockResolvedValue([
      stubRow({ state: "failed_permanent", lastError: "DISK_FULL" }),
    ]);
    vi.mocked(commands.retryDownload).mockResolvedValue(
      stubRow({ state: "queued" })
    );
    renderWith(<DownloadsPage />);
    const retryButton = await screen.findByRole("button", { name: /retry/i });
    await userEvent.click(retryButton);
    await waitFor(() =>
      expect(vi.mocked(commands.retryDownload)).toHaveBeenCalledWith({
        vodId: "v1",
      })
    );
  });

  test("state filter chip narrows the listing query", async () => {
    vi.mocked(commands.listDownloads).mockResolvedValue([]);
    renderWith(<DownloadsPage />);
    await userEvent.click(screen.getByRole("button", { name: /paused/i }));
    await waitFor(() => {
      const last = vi.mocked(commands.listDownloads).mock.calls.at(-1)?.[0];
      expect(last?.filters?.state).toBe("paused");
    });
  });
});
