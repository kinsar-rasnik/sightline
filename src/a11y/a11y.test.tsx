// Phase 5 accessibility gate — per docs/a11y-exceptions.md.
//
// Renders every route in a jsdom harness and runs axe-core on the
// result. The gate fails on any WCAG 2.1 AA violation unless that
// specific rule is listed as a documented exception.
//
// Runs under the `pnpm a11y` script (which is just vitest pointed at
// src/a11y/**). CI includes that script in the `checks` job, so any
// violation that lands in a PR fails the build.

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, cleanup } from "@testing-library/react";
import axe, { type AxeResults, type Result as AxeResult } from "axe-core";
import { afterEach, beforeEach, describe, expect, test, vi } from "vitest";

import { LibraryPage } from "@/features/vods/LibraryPage";
import { TimelinePage } from "@/features/timeline/TimelinePage";
import { StreamersPage } from "@/features/streamers/StreamersPage";
import { DownloadsPage } from "@/features/downloads/DownloadsPage";
import { SettingsPage } from "@/features/settings/SettingsPage";

import type * as IpcModule from "@/ipc";

// LibraryPage subscribes to Tauri events for cache invalidation.
// jsdom doesn't have a Tauri runtime — short-circuit `listen` to a
// no-op unsubscribe so the route renders without throwing.
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));

// Mock every backend call axe-touched routes reach for. jsdom never
// actually boots a Tauri runtime, so each `commands.*` is a direct
// promise — no timeouts, no loading states blocking the initial render.
vi.mock("@/ipc", async () => {
  const actual = await vi.importActual<typeof IpcModule>("@/ipc");
  return {
    ...actual,
    commands: {
      ...actual.commands,
      health: vi.fn().mockResolvedValue({
        appName: "sightline",
        appVersion: "0.1.0",
        schemaVersion: 8,
        startedAt: 0,
        checkedAt: 0,
      }),
      listVods: vi.fn().mockResolvedValue([]),
      listStreamers: vi.fn().mockResolvedValue([]),
      listDownloads: vi.fn().mockResolvedValue([]),
      listTimeline: vi.fn().mockResolvedValue([]),
      getTimelineStats: vi.fn().mockResolvedValue({
        totalIntervals: 0,
        streamerLanes: 0,
        earliestAt: null,
        latestAt: null,
      }),
      getPollStatus: vi.fn().mockResolvedValue([]),
      getSettings: vi.fn().mockResolvedValue({
        enabledGameIds: ["32982"],
        pollFloorSeconds: 600,
        pollRecentSeconds: 1800,
        pollCeilingSeconds: 7200,
        concurrencyCap: 4,
        firstBackfillLimit: 100,
        credentials: {
          configured: false,
          clientIdMasked: null,
          lastTokenAcquiredAt: null,
        },
        libraryRoot: null,
        libraryLayout: "plex",
        stagingPath: null,
        maxConcurrentDownloads: 2,
        bandwidthLimitBps: null,
        qualityPreset: "source",
        autoUpdateYtDlp: true,
        windowCloseBehavior: "hide",
        startAtLogin: false,
        showDockIcon: false,
        notificationsEnabled: true,
        notifyDownloadComplete: false,
        notifyDownloadFailed: true,
        notifyFavoritesIngest: true,
        notifyStorageLow: true,
      }),
      getStagingInfo: vi.fn().mockResolvedValue({
        path: "/tmp/staging",
        freeBytes: 1024 * 1024 * 1024,
        staleFileCount: 0,
      }),
      getLibraryInfo: vi
        .fn()
        .mockResolvedValue({ path: null, freeBytes: null, fileCount: 0 }),
      getTwitchCredentialsStatus: vi.fn().mockResolvedValue({
        configured: false,
        clientIdMasked: null,
        lastTokenAcquiredAt: null,
      }),
      listShortcuts: vi.fn().mockResolvedValue([]),
      getVodAssets: vi.fn().mockResolvedValue({
        vodId: "",
        videoPath: null,
        thumbnailPath: null,
        previewFramePaths: [],
      }),
    },
  };
});

function wrap(ui: React.ReactElement) {
  const client = new QueryClient({
    defaultOptions: {
      queries: { retry: false, refetchInterval: false },
      mutations: { retry: false },
    },
  });
  return <QueryClientProvider client={client}>{ui}</QueryClientProvider>;
}

// Rules we explicitly skip, with a short rationale and the exceptions
// doc they're recorded in. Keep this list tiny; every entry is a
// regression-risk surface.
const ALLOWLIST = new Set<string>([
  // jsdom doesn't run a rendering engine, so `color-contrast` always
  // flags false-positive `color-contrast-enhanced` even when the real
  // browser resolves the computed styles correctly. Covered by the
  // manual dark/light review recorded in docs/a11y-exceptions.md.
  "color-contrast",
  "color-contrast-enhanced",
]);

function filterViolations(results: AxeResults): AxeResult[] {
  return results.violations.filter((v) => !ALLOWLIST.has(v.id));
}

async function runAxe(container: HTMLElement) {
  const results = await axe.run(container, {
    runOnly: {
      type: "tag",
      values: ["wcag2a", "wcag2aa", "wcag21a", "wcag21aa"],
    },
  });
  return filterViolations(results);
}

// Axe is expensive (~hundreds of ms per run on a large DOM). Bump the
// default 5s timeout so slow CI runners don't flake.
const AXE_TIMEOUT = 15_000;

describe("a11y: axe-core over every route", () => {
  beforeEach(() => {
    // Put the scanned tree under <main> so landmark-one-main is
    // satisfied inside the test harness (AppShell wraps each route
    // with one at runtime).
  });
  afterEach(() => {
    cleanup();
  });

  test(
    "library route has no axe violations",
    async () => {
      const { container } = render(wrap(<main><LibraryPage /></main>));
      const violations = await runAxe(container);
      if (violations.length > 0) {
        console.error("axe violations:", violations.map(formatViolation));
      }
      expect(violations).toEqual([]);
    },
    AXE_TIMEOUT
  );

  test(
    "timeline route has no axe violations",
    async () => {
      const { container } = render(wrap(<main><TimelinePage /></main>));
      const violations = await runAxe(container);
      if (violations.length > 0) {
        console.error("axe violations:", violations.map(formatViolation));
      }
      expect(violations).toEqual([]);
    },
    AXE_TIMEOUT
  );

  test(
    "streamers route has no axe violations",
    async () => {
      const { container } = render(wrap(<main><StreamersPage /></main>));
      const violations = await runAxe(container);
      if (violations.length > 0) {
        console.error("axe violations:", violations.map(formatViolation));
      }
      expect(violations).toEqual([]);
    },
    AXE_TIMEOUT
  );

  test(
    "downloads route has no axe violations",
    async () => {
      const { container } = render(wrap(<main><DownloadsPage /></main>));
      const violations = await runAxe(container);
      if (violations.length > 0) {
        console.error("axe violations:", violations.map(formatViolation));
      }
      expect(violations).toEqual([]);
    },
    AXE_TIMEOUT
  );

  test(
    "settings route has no axe violations",
    async () => {
      const { container } = render(wrap(<main><SettingsPage /></main>));
      const violations = await runAxe(container);
      if (violations.length > 0) {
        console.error("axe violations:", violations.map(formatViolation));
      }
      expect(violations).toEqual([]);
    },
    AXE_TIMEOUT
  );
});

function formatViolation(v: AxeResult) {
  return {
    id: v.id,
    help: v.help,
    impact: v.impact,
    nodes: v.nodes.map((n) => ({
      html: n.html.slice(0, 140),
      failureSummary: n.failureSummary,
    })),
  };
}
