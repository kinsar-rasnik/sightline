import { act, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type { Chapter, WatchState } from "@/ipc";

import { PlayerChrome } from "./PlayerChrome";
import { CONTROLS_HIDE_AFTER_MS } from "./player-constants";

const baseProps = {
  title: "Test VOD",
  currentSeconds: 30,
  durationSeconds: 600,
  chapters: [] as Chapter[],
  volume: 0.8,
  muted: false,
  playbackRate: 1,
  showRemaining: false,
  isFullscreen: false,
  pipActive: false,
  completionThreshold: 0.9,
  resumeSeconds: null as number | null,
  watchedState: "in_progress" as WatchState,
  onTogglePlay: vi.fn(),
  onSeekTo: vi.fn(),
  onSeekBy: vi.fn(),
  onToggleRemaining: vi.fn(),
  onVolumeChange: vi.fn(),
  onToggleMute: vi.fn(),
  onSetSpeed: vi.fn(),
  onToggleFullscreen: vi.fn(),
  onTogglePip: vi.fn(),
  onJumpChapter: vi.fn(),
  onMarkWatched: vi.fn(),
  onMarkUnwatched: vi.fn(),
};

beforeEach(() => {
  vi.useFakeTimers();
});

afterEach(() => {
  vi.useRealTimers();
  vi.clearAllMocks();
});

function getOverlay(): HTMLElement {
  return screen.getByLabelText("Player controls");
}

describe("PlayerChrome — overlay auto-hide", () => {
  it("renders visible on initial mount", () => {
    render(<PlayerChrome {...baseProps} isPlaying={true} />);
    expect(getOverlay().className).toContain("opacity-100");
  });

  it("hides after CONTROLS_HIDE_AFTER_MS while playing", () => {
    render(<PlayerChrome {...baseProps} isPlaying={true} />);
    expect(getOverlay().className).toContain("opacity-100");
    act(() => {
      vi.advanceTimersByTime(CONTROLS_HIDE_AFTER_MS - 1);
    });
    expect(getOverlay().className).toContain("opacity-100");
    act(() => {
      vi.advanceTimersByTime(2);
    });
    expect(getOverlay().className).toContain("opacity-0");
  });

  it("stays visible while paused", () => {
    render(<PlayerChrome {...baseProps} isPlaying={false} />);
    act(() => {
      vi.advanceTimersByTime(CONTROLS_HIDE_AFTER_MS * 2);
    });
    expect(getOverlay().className).toContain("opacity-100");
  });

  it("re-shows when isPlaying flips from true to false", () => {
    const { rerender } = render(
      <PlayerChrome {...baseProps} isPlaying={true} />
    );
    act(() => {
      vi.advanceTimersByTime(CONTROLS_HIDE_AFTER_MS + 50);
    });
    expect(getOverlay().className).toContain("opacity-0");
    rerender(<PlayerChrome {...baseProps} isPlaying={false} />);
    expect(getOverlay().className).toContain("opacity-100");
  });

  it("mouse move resets the hide timer", () => {
    render(<PlayerChrome {...baseProps} isPlaying={true} />);
    act(() => {
      vi.advanceTimersByTime(CONTROLS_HIDE_AFTER_MS - 100);
    });
    // Bump the timer back up to full via mousemove. Overlay must
    // remain visible past the original deadline.
    act(() => {
      const overlay = getOverlay();
      const ev = new MouseEvent("mousemove", { bubbles: true });
      overlay.dispatchEvent(ev);
    });
    act(() => {
      vi.advanceTimersByTime(200);
    });
    expect(getOverlay().className).toContain("opacity-100");
  });
});
