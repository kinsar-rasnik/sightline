import "@testing-library/jest-dom/vitest";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { UpdateSettingsSection } from "./UpdateSettingsSection";

const mocks = vi.hoisted(() => ({
  getUpdateStatus: vi.fn(),
  checkForUpdate: vi.fn(),
  skipUpdateVersion: vi.fn(),
  openReleaseUrl: vi.fn(),
}));

vi.mock("@/ipc", async () => {
  const actual = (await vi.importActual("@/ipc")) as {
    commands: Record<string, unknown>;
    [k: string]: unknown;
  };
  return {
    ...actual,
    commands: {
      ...actual.commands,
      getUpdateStatus: mocks.getUpdateStatus,
      checkForUpdate: mocks.checkForUpdate,
      skipUpdateVersion: mocks.skipUpdateVersion,
      openReleaseUrl: mocks.openReleaseUrl,
    },
  };
});

const baseSettings = {
  enabledGameIds: [],
  pollFloorSeconds: 0,
  pollRecentSeconds: 0,
  pollCeilingSeconds: 0,
  concurrencyCap: 1,
  firstBackfillLimit: 1,
  credentials: {
    configured: false,
    clientIdMasked: null,
    lastTokenAcquiredAt: null,
  },
  libraryRoot: null,
  libraryLayout: "plex" as const,
  stagingPath: null,
  maxConcurrentDownloads: 1,
  bandwidthLimitBps: null,
  qualityPreset: "source" as const,
  autoUpdateYtDlp: false,
  windowCloseBehavior: "hide" as const,
  startAtLogin: false,
  showDockIcon: false,
  notificationsEnabled: true,
  notifyDownloadComplete: true,
  notifyDownloadFailed: true,
  notifyFavoritesIngest: true,
  notifyStorageLow: true,
  completionThreshold: 0.9,
  syncDriftThresholdMs: 250,
  syncDefaultLayout: "split-50-50" as const,
  syncDefaultLeader: "first-opened",
  cleanupEnabled: false,
  cleanupHighWatermark: 0.9,
  cleanupLowWatermark: 0.75,
  cleanupScheduleHour: 3,
  updateCheckEnabled: false,
  updateCheckLastRun: null,
  updateCheckSkipVersion: null,
  videoQualityProfile: "720p30" as const,
  softwareEncodeOptIn: false,
  encoderCapability: null,
  maxConcurrentReencodes: 1,
  cpuThrottleHighThreshold: 0.7,
  cpuThrottleLowThreshold: 0.5,
};

function renderSection(props?: Partial<typeof baseSettings>) {
  const onUpdate = vi.fn();
  const qc = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  render(
    <QueryClientProvider client={qc}>
      <UpdateSettingsSection
        settings={{ ...baseSettings, ...props }}
        onUpdate={onUpdate}
        pending={false}
      />
    </QueryClientProvider>,
  );
  return { onUpdate };
}

describe("UpdateSettingsSection", () => {
  beforeEach(() => {
    mocks.getUpdateStatus.mockResolvedValue({
      enabled: false,
      currentVersion: "1.0.0",
      lastCheckedAt: null,
      skipVersion: null,
      available: null,
    });
    mocks.checkForUpdate.mockResolvedValue(null);
    mocks.skipUpdateVersion.mockResolvedValue(undefined);
    mocks.openReleaseUrl.mockResolvedValue(undefined);
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  it("renders the master toggle bound to settings.updateCheckEnabled", async () => {
    renderSection();
    const toggle = await screen.findByLabelText(/Check for updates/i);
    expect(toggle).not.toBeChecked();
  });

  it("shows the running version", () => {
    renderSection();
    expect(screen.getByText(/Currently running/i)).toBeInTheDocument();
  });

  it("invokes onUpdate when the toggle flips", async () => {
    const user = userEvent.setup();
    const { onUpdate } = renderSection();
    const toggle = await screen.findByLabelText(/Check for updates/i);
    await user.click(toggle);
    expect(onUpdate).toHaveBeenCalledWith({ updateCheckEnabled: true });
  });

  it("'Check now' calls checkForUpdate with force: true", async () => {
    const user = userEvent.setup();
    renderSection();
    const button = await screen.findByRole("button", { name: /Check now/i });
    await user.click(button);
    expect(mocks.checkForUpdate).toHaveBeenCalledWith({ force: true });
  });
});
