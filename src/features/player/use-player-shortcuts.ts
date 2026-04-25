import { useEffect, useMemo, useRef, type RefObject } from "react";

import {
  DEFAULT_SHORTCUTS,
  type ActionId,
} from "@/features/shortcuts/use-shortcuts";

/**
 * Subset of `ActionId` dispatched by the player-scoped hook. Lives in
 * the same enum so the customization UI + `app_settings.shortcuts_json`
 * round-trips can treat all bindings uniformly, but the engine here
 * only ever fires `player_*` actions.
 */
export type PlayerActionId = Extract<ActionId, `player_${string}`>;

const PLAYER_ACTION_IDS: ReadonlyArray<PlayerActionId> = [
  "player_play_pause",
  "player_seek_back_5",
  "player_seek_forward_5",
  "player_seek_back_10",
  "player_seek_forward_10",
  "player_volume_up",
  "player_volume_down",
  "player_mute",
  "player_fullscreen",
  "player_pip",
  "player_chapter_next",
  "player_chapter_prev",
  "player_speed_down",
  "player_speed_up",
  "player_close",
];

/**
 * Defaults filtered out of `DEFAULT_SHORTCUTS` so callers (PlayerPage,
 * tests) can read just the player slice without re-typing them.
 */
export const DEFAULT_PLAYER_SHORTCUTS: Record<PlayerActionId, string> =
  Object.fromEntries(
    PLAYER_ACTION_IDS.map((id) => [id, DEFAULT_SHORTCUTS[id]])
  ) as Record<PlayerActionId, string>;

export interface PlayerShortcutHandlers {
  /** Toggle play/pause. Also bound to Space alongside the named action. */
  playPause: () => void;
  seekBy: (seconds: number) => void;
  /** ±0.1 by default; the hook passes a positive or negative delta. */
  adjustVolume: (delta: number) => void;
  toggleMute: () => void;
  toggleFullscreen: () => void;
  togglePip: () => void;
  jumpChapter: (direction: "next" | "prev") => void;
  /** Bump playback speed up or down one step. Caller owns the table. */
  stepSpeed: (direction: "up" | "down") => void;
  /** Esc — exits fullscreen if active, otherwise closes the player. */
  close: () => void;
  /** Frame step (Shift+Arrow or `,` / `.`). Pauses if currently playing. */
  frameStep: (direction: "back" | "forward") => void;
  /** Seek to fraction of duration (digits 0–9 → 0–90 %). */
  seekToFraction: (fraction: number) => void;
}

/**
 * Container-scoped sibling of `useShortcuts`. Attaches the keydown
 * listener to the supplied container ref so player actions only fire
 * when the player has focus — navigation shortcuts on Library /
 * Settings keep working independently. Falls back through the named
 * binding table first; structural keys (Space, digits, `,`/`.`,
 * Shift+Arrow) are dispatched via dedicated handlers because they
 * either alias a named action (Space → play/pause) or describe a
 * range that doesn't fit the single-key binding table (digit seek).
 */
export function usePlayerShortcuts(
  containerRef: RefObject<HTMLElement | null>,
  shortcuts: Record<PlayerActionId, string>,
  handlers: PlayerShortcutHandlers
) {
  const handlersRef = useRef(handlers);
  handlersRef.current = handlers;

  const bindings = useMemo(() => buildBindings(shortcuts), [shortcuts]);

  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;

    function dispatch(id: PlayerActionId) {
      const h = handlersRef.current;
      switch (id) {
        case "player_play_pause":
          h.playPause();
          break;
        case "player_seek_back_5":
          h.seekBy(-5);
          break;
        case "player_seek_forward_5":
          h.seekBy(5);
          break;
        case "player_seek_back_10":
          h.seekBy(-10);
          break;
        case "player_seek_forward_10":
          h.seekBy(10);
          break;
        case "player_volume_up":
          h.adjustVolume(0.1);
          break;
        case "player_volume_down":
          h.adjustVolume(-0.1);
          break;
        case "player_mute":
          h.toggleMute();
          break;
        case "player_fullscreen":
          h.toggleFullscreen();
          break;
        case "player_pip":
          h.togglePip();
          break;
        case "player_chapter_next":
          h.jumpChapter("next");
          break;
        case "player_chapter_prev":
          h.jumpChapter("prev");
          break;
        case "player_speed_down":
          h.stepSpeed("down");
          break;
        case "player_speed_up":
          h.stepSpeed("up");
          break;
        case "player_close":
          h.close();
          break;
      }
    }

    function onKey(e: KeyboardEvent) {
      if (e.defaultPrevented) return;
      const target = e.target as HTMLElement | null;
      if (target) {
        const tag = target.tagName;
        if (tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT") return;
        if (target.isContentEditable) return;
      }

      // Frame step: Shift+Arrow takes priority over the seek binding.
      if (e.shiftKey && (e.key === "ArrowLeft" || e.key === "ArrowRight")) {
        e.preventDefault();
        handlersRef.current.frameStep(
          e.key === "ArrowLeft" ? "back" : "forward"
        );
        return;
      }
      // Frame step via , / . — distinct from the named bindings.
      if (e.key === "," || e.key === ".") {
        e.preventDefault();
        handlersRef.current.frameStep(e.key === "," ? "back" : "forward");
        return;
      }
      // Space aliases the named play_pause action.
      if (e.key === " ") {
        e.preventDefault();
        handlersRef.current.playPause();
        return;
      }
      // Digits 0–9 → 0–90 % seek. Range-based, not customisable.
      if (/^[0-9]$/.test(e.key)) {
        e.preventDefault();
        handlersRef.current.seekToFraction(Number(e.key) / 10);
        return;
      }

      const stroke = eventToKeystroke(e);
      const hit = bindings.get(stroke);
      if (hit) {
        e.preventDefault();
        dispatch(hit);
      }
    }

    container.addEventListener("keydown", onKey);
    return () => container.removeEventListener("keydown", onKey);
  }, [bindings, containerRef]);
}

function buildBindings(
  shortcuts: Record<PlayerActionId, string>
): Map<string, PlayerActionId> {
  const out = new Map<string, PlayerActionId>();
  for (const [action, combo] of Object.entries(shortcuts)) {
    const id = action as PlayerActionId;
    const trimmed = combo.trim().toLowerCase();
    if (!trimmed) continue;
    out.set(trimmed, id);
  }
  return out;
}

/**
 * Translate a keydown event into a binding-table key. `shift+` is
 * emitted only for arrow / letter keys, where the shifted character
 * is identical-cased to its unshifted form (e.g. `shift+c` vs `c`).
 * Symbols like `<` already encode shift in the printed character, so
 * adding a `shift+` prefix would mismatch the binding string.
 */
function eventToKeystroke(e: KeyboardEvent): string {
  const lower = e.key.toLowerCase();
  if (e.shiftKey && (lower.startsWith("arrow") || /^[a-z]$/.test(lower))) {
    return `shift+${lower}`;
  }
  return lower;
}
