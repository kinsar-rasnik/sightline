import { renderHook } from "@testing-library/react";
import { useRef } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";

import {
  DEFAULT_PLAYER_SHORTCUTS,
  type PlayerShortcutHandlers,
  usePlayerShortcuts,
} from "./use-player-shortcuts";

function makeHandlers(): PlayerShortcutHandlers {
  return {
    playPause: vi.fn(),
    seekBy: vi.fn(),
    adjustVolume: vi.fn(),
    toggleMute: vi.fn(),
    toggleFullscreen: vi.fn(),
    togglePip: vi.fn(),
    jumpChapter: vi.fn(),
    stepSpeed: vi.fn(),
    close: vi.fn(),
    frameStep: vi.fn(),
    seekToFraction: vi.fn(),
  };
}

interface Setup {
  container: HTMLDivElement;
  handlers: PlayerShortcutHandlers;
}

function setup(): Setup {
  const container = document.createElement("div");
  container.tabIndex = -1;
  document.body.appendChild(container);
  const handlers = makeHandlers();
  renderHook(() => {
    const ref = useRef<HTMLDivElement | null>(container);
    usePlayerShortcuts(ref, DEFAULT_PLAYER_SHORTCUTS, handlers);
  });
  return { container, handlers };
}

afterEach(() => {
  document.body.innerHTML = "";
});

function press(
  container: HTMLElement,
  init: KeyboardEventInit & { key: string }
) {
  const ev = new KeyboardEvent("keydown", { bubbles: true, ...init });
  container.dispatchEvent(ev);
  return ev;
}

describe("usePlayerShortcuts — named bindings", () => {
  it("k fires play_pause", () => {
    const { container, handlers } = setup();
    press(container, { key: "k" });
    expect(handlers.playPause).toHaveBeenCalledTimes(1);
  });

  it("ArrowLeft / ArrowRight fire ±5 seek", () => {
    const { container, handlers } = setup();
    press(container, { key: "ArrowLeft" });
    press(container, { key: "ArrowRight" });
    expect(handlers.seekBy).toHaveBeenNthCalledWith(1, -5);
    expect(handlers.seekBy).toHaveBeenNthCalledWith(2, 5);
  });

  it("j / l fire ±10 seek", () => {
    const { container, handlers } = setup();
    press(container, { key: "j" });
    press(container, { key: "l" });
    expect(handlers.seekBy).toHaveBeenNthCalledWith(1, -10);
    expect(handlers.seekBy).toHaveBeenNthCalledWith(2, 10);
  });

  it("m / f / p fire mute / fullscreen / pip", () => {
    const { container, handlers } = setup();
    press(container, { key: "m" });
    press(container, { key: "f" });
    press(container, { key: "p" });
    expect(handlers.toggleMute).toHaveBeenCalledOnce();
    expect(handlers.toggleFullscreen).toHaveBeenCalledOnce();
    expect(handlers.togglePip).toHaveBeenCalledOnce();
  });

  it("c fires next chapter; shift+C fires previous chapter", () => {
    const { container, handlers } = setup();
    press(container, { key: "c" });
    press(container, { key: "C", shiftKey: true });
    expect(handlers.jumpChapter).toHaveBeenNthCalledWith(1, "next");
    expect(handlers.jumpChapter).toHaveBeenNthCalledWith(2, "prev");
  });

  it("< / > fire speed step down / up", () => {
    const { container, handlers } = setup();
    press(container, { key: "<" });
    press(container, { key: ">" });
    expect(handlers.stepSpeed).toHaveBeenNthCalledWith(1, "down");
    expect(handlers.stepSpeed).toHaveBeenNthCalledWith(2, "up");
  });

  it("Escape fires close", () => {
    const { container, handlers } = setup();
    press(container, { key: "Escape" });
    expect(handlers.close).toHaveBeenCalledOnce();
  });
});

describe("usePlayerShortcuts — structural keys", () => {
  it("Space aliases play_pause", () => {
    const { container, handlers } = setup();
    press(container, { key: " " });
    expect(handlers.playPause).toHaveBeenCalledOnce();
  });

  it("digits 0–9 fire seek to fraction", () => {
    const { container, handlers } = setup();
    press(container, { key: "0" });
    press(container, { key: "5" });
    press(container, { key: "9" });
    expect(handlers.seekToFraction).toHaveBeenNthCalledWith(1, 0);
    expect(handlers.seekToFraction).toHaveBeenNthCalledWith(2, 0.5);
    expect(handlers.seekToFraction).toHaveBeenNthCalledWith(3, 0.9);
  });

  it("Shift+ArrowLeft / Shift+ArrowRight fire frame step", () => {
    const { container, handlers } = setup();
    press(container, { key: "ArrowLeft", shiftKey: true });
    press(container, { key: "ArrowRight", shiftKey: true });
    expect(handlers.frameStep).toHaveBeenNthCalledWith(1, "back");
    expect(handlers.frameStep).toHaveBeenNthCalledWith(2, "forward");
  });

  it(", / . fire frame step", () => {
    const { container, handlers } = setup();
    press(container, { key: "," });
    press(container, { key: "." });
    expect(handlers.frameStep).toHaveBeenNthCalledWith(1, "back");
    expect(handlers.frameStep).toHaveBeenNthCalledWith(2, "forward");
  });
});

describe("usePlayerShortcuts — input scope", () => {
  it("ignores keystrokes inside an INPUT element", () => {
    const { container, handlers } = setup();
    const input = document.createElement("input");
    container.appendChild(input);
    const ev = new KeyboardEvent("keydown", {
      key: "k",
      bubbles: true,
    });
    input.dispatchEvent(ev);
    expect(handlers.playPause).not.toHaveBeenCalled();
  });
});
