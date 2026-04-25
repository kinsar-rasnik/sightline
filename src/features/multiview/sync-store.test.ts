import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { useMultiViewStore } from "./sync-store";

vi.mock("@/ipc", () => ({
  commands: {
    getOverlap: vi.fn(),
    openSyncGroup: vi.fn(),
    closeSyncGroup: vi.fn(),
    setSyncLeader: vi.fn(),
    getVod: vi.fn(),
  },
}));

import { commands } from "@/ipc";

const mocked = commands as unknown as {
  getOverlap: ReturnType<typeof vi.fn>;
  openSyncGroup: ReturnType<typeof vi.fn>;
  closeSyncGroup: ReturnType<typeof vi.fn>;
  setSyncLeader: ReturnType<typeof vi.fn>;
  getVod: ReturnType<typeof vi.fn>;
};

beforeEach(() => {
  useMultiViewStore.getState().reset();
  mocked.getOverlap.mockReset();
  mocked.openSyncGroup.mockReset();
  mocked.closeSyncGroup.mockReset();
  mocked.setSyncLeader.mockReset();
  mocked.getVod.mockReset();
});

afterEach(() => {
  useMultiViewStore.getState().reset();
});

function vodFixture(id: string, start: number, dur: number) {
  return {
    vod: {
      twitchVideoId: id,
      streamStartedAt: start,
      durationSeconds: dur,
    },
  };
}

describe("multiview store — open/close", () => {
  it("populates panes + overlap on successful open", async () => {
    mocked.getOverlap.mockResolvedValue({
      vodIds: ["v1", "v2"],
      window: { startAt: 100, endAt: 1100 },
    });
    mocked.openSyncGroup.mockResolvedValue({
      id: 7,
      createdAt: 1_000,
      closedAt: null,
      layout: "split-50-50",
      leaderPaneIndex: 0,
      status: "active",
      panes: [
        {
          paneIndex: 0,
          vodId: "v1",
          volume: 1,
          muted: false,
          joinedAt: 1_000,
        },
        {
          paneIndex: 1,
          vodId: "v2",
          volume: 1,
          muted: false,
          joinedAt: 1_000,
        },
      ],
    });
    mocked.getVod.mockImplementation(({ twitchVideoId }) => {
      if (twitchVideoId === "v1") return Promise.resolve(vodFixture("v1", 0, 1200));
      return Promise.resolve(vodFixture("v2", 100, 1200));
    });

    await useMultiViewStore.getState().openSession(["v1", "v2"]);

    const s = useMultiViewStore.getState();
    expect(s.phase).toBe("active");
    expect(s.sessionId).toBe(7);
    expect(s.leaderPaneIndex).toBe(0);
    expect(s.panes).toHaveLength(2);
    expect(s.panes[0]).toMatchObject({
      paneIndex: 0,
      vodId: "v1",
      streamStartedAt: 0,
      durationSeconds: 1200,
    });
    expect(s.panes[1]).toMatchObject({
      paneIndex: 1,
      vodId: "v2",
      streamStartedAt: 100,
      durationSeconds: 1200,
    });
    expect(s.overlap).toEqual({ startAt: 100, endAt: 1100 });
  });

  it("rejects an open with the wrong number of vodIds", async () => {
    await useMultiViewStore.getState().openSession(["only"]);
    const s = useMultiViewStore.getState();
    expect(s.phase).toBe("idle");
    expect(s.errorDetail).toMatch(/two/i);
    expect(mocked.openSyncGroup).not.toHaveBeenCalled();
  });

  it("close calls the IPC and resets state", async () => {
    mocked.getOverlap.mockResolvedValue({
      vodIds: ["v1", "v2"],
      window: { startAt: 0, endAt: 100 },
    });
    mocked.openSyncGroup.mockResolvedValue({
      id: 11,
      createdAt: 0,
      closedAt: null,
      layout: "split-50-50",
      leaderPaneIndex: 0,
      status: "active",
      panes: [
        {
          paneIndex: 0,
          vodId: "v1",
          volume: 1,
          muted: false,
          joinedAt: 0,
        },
        {
          paneIndex: 1,
          vodId: "v2",
          volume: 1,
          muted: false,
          joinedAt: 0,
        },
      ],
    });
    mocked.getVod.mockResolvedValue(vodFixture("v1", 0, 100));
    mocked.closeSyncGroup.mockResolvedValue(undefined);

    await useMultiViewStore.getState().openSession(["v1", "v2"]);
    await useMultiViewStore.getState().closeSession();

    expect(mocked.closeSyncGroup).toHaveBeenCalledWith({ sessionId: 11 });
    const s = useMultiViewStore.getState();
    expect(s.phase).toBe("idle");
    expect(s.sessionId).toBeNull();
    expect(s.panes).toEqual([]);
  });
});

describe("multiview store — leader + audio mix", () => {
  beforeEach(async () => {
    mocked.getOverlap.mockResolvedValue({
      vodIds: ["v1", "v2"],
      window: { startAt: 0, endAt: 100 },
    });
    mocked.openSyncGroup.mockResolvedValue({
      id: 1,
      createdAt: 0,
      closedAt: null,
      layout: "split-50-50",
      leaderPaneIndex: 0,
      status: "active",
      panes: [
        {
          paneIndex: 0,
          vodId: "v1",
          volume: 1,
          muted: false,
          joinedAt: 0,
        },
        {
          paneIndex: 1,
          vodId: "v2",
          volume: 1,
          muted: false,
          joinedAt: 0,
        },
      ],
    });
    mocked.getVod.mockResolvedValue(vodFixture("v1", 0, 100));
    await useMultiViewStore.getState().openSession(["v1", "v2"]);
  });

  it("setLeader fires the IPC and updates the store", async () => {
    mocked.setSyncLeader.mockResolvedValue({
      id: 1,
      createdAt: 0,
      closedAt: null,
      layout: "split-50-50",
      leaderPaneIndex: 1,
      status: "active",
      panes: [],
    });
    await useMultiViewStore.getState().setLeader(1);
    expect(mocked.setSyncLeader).toHaveBeenCalledWith({
      sessionId: 1,
      paneIndex: 1,
    });
    expect(useMultiViewStore.getState().leaderPaneIndex).toBe(1);
  });

  it("setVolume clamps to [0, 1]", () => {
    useMultiViewStore.getState().setVolume(0, 1.5);
    useMultiViewStore.getState().setVolume(1, -0.2);
    const panes = useMultiViewStore.getState().panes;
    expect(panes[0]?.volume).toBe(1);
    expect(panes[1]?.volume).toBe(0);
  });

  it("toggleMute flips per-pane mute independently", () => {
    useMultiViewStore.getState().toggleMute(0);
    let panes = useMultiViewStore.getState().panes;
    expect(panes[0]?.muted).toBe(true);
    expect(panes[1]?.muted).toBe(false);
    useMultiViewStore.getState().toggleMute(0);
    panes = useMultiViewStore.getState().panes;
    expect(panes[0]?.muted).toBe(false);
  });

  it("setOutOfRange flips the per-pane flag", () => {
    useMultiViewStore.getState().setOutOfRange(1, true);
    expect(useMultiViewStore.getState().panes[1]?.outOfRange).toBe(true);
    useMultiViewStore.getState().setOutOfRange(1, false);
    expect(useMultiViewStore.getState().panes[1]?.outOfRange).toBe(false);
  });

  it("recordDrift stamps the per-pane lastDriftMs", () => {
    useMultiViewStore.getState().recordDrift(0, 420);
    expect(useMultiViewStore.getState().panes[0]?.lastDriftMs).toBe(420);
  });
});
