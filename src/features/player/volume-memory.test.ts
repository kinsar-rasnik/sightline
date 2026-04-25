import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { readVolume, writeVolume } from "./volume-memory";

const GLOBAL_KEY = "sightline:player:volume:global";
const VOD_KEY = (vodId: string) => `sightline:player:volume:vod:${vodId}`;

beforeEach(() => {
  window.localStorage.clear();
});

afterEach(() => {
  window.localStorage.clear();
});

describe("readVolume", () => {
  it("session memory ignores localStorage and returns default", () => {
    window.localStorage.setItem(GLOBAL_KEY, "0.3");
    expect(readVolume("session", "v1")).toBe(1);
  });

  it("global memory reads the global key", () => {
    window.localStorage.setItem(GLOBAL_KEY, "0.4");
    expect(readVolume("global", "v1")).toBe(0.4);
  });

  it("global memory falls back to default when key is unset", () => {
    expect(readVolume("global", "v1")).toBe(1);
  });

  it("vod memory reads the per-VOD key when present", () => {
    window.localStorage.setItem(VOD_KEY("v1"), "0.6");
    window.localStorage.setItem(GLOBAL_KEY, "0.2");
    expect(readVolume("vod", "v1")).toBe(0.6);
  });

  it("vod memory falls back to global when per-VOD key is missing", () => {
    window.localStorage.setItem(GLOBAL_KEY, "0.3");
    expect(readVolume("vod", "v1")).toBe(0.3);
  });

  it("vod memory returns default when neither key is set", () => {
    expect(readVolume("vod", "v1")).toBe(1);
  });

  it("clamps out-of-range stored values into [0, 1]", () => {
    window.localStorage.setItem(GLOBAL_KEY, "5");
    expect(readVolume("global", "v1")).toBe(1);
    window.localStorage.setItem(GLOBAL_KEY, "-1");
    expect(readVolume("global", "v1")).toBe(0);
  });

  it("ignores garbage values and returns default", () => {
    window.localStorage.setItem(GLOBAL_KEY, "not a number");
    expect(readVolume("global", "v1")).toBe(1);
  });
});

describe("writeVolume", () => {
  it("session memory writes nothing", () => {
    writeVolume("session", "v1", 0.5);
    expect(window.localStorage.getItem(GLOBAL_KEY)).toBeNull();
    expect(window.localStorage.getItem(VOD_KEY("v1"))).toBeNull();
  });

  it("global memory writes only the global key", () => {
    writeVolume("global", "v1", 0.5);
    expect(window.localStorage.getItem(GLOBAL_KEY)).toBe("0.5");
    expect(window.localStorage.getItem(VOD_KEY("v1"))).toBeNull();
  });

  it("vod memory writes both per-VOD and global keys", () => {
    writeVolume("vod", "v1", 0.7);
    expect(window.localStorage.getItem(VOD_KEY("v1"))).toBe("0.7");
    expect(window.localStorage.getItem(GLOBAL_KEY)).toBe("0.7");
  });

  it("clamps writes into [0, 1]", () => {
    writeVolume("global", "v1", 1.5);
    expect(window.localStorage.getItem(GLOBAL_KEY)).toBe("1");
    writeVolume("global", "v1", -0.4);
    expect(window.localStorage.getItem(GLOBAL_KEY)).toBe("0");
  });

  it("per-VOD writes are independent across vodIds", () => {
    writeVolume("vod", "v1", 0.2);
    writeVolume("vod", "v2", 0.8);
    expect(window.localStorage.getItem(VOD_KEY("v1"))).toBe("0.2");
    expect(window.localStorage.getItem(VOD_KEY("v2"))).toBe("0.8");
  });
});

describe("volume memory round-trip", () => {
  it("vod write → vod read returns the written value", () => {
    writeVolume("vod", "v42", 0.33);
    expect(readVolume("vod", "v42")).toBe(0.33);
  });

  it("vod read on a different vodId falls through to the global write", () => {
    writeVolume("vod", "v42", 0.33);
    expect(readVolume("vod", "v99")).toBe(0.33); // came via global fallback
  });

  it("policy switch from vod to global picks up the global key", () => {
    writeVolume("vod", "v42", 0.4);
    expect(readVolume("global", "v42")).toBe(0.4);
  });
});
