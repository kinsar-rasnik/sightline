import { useEffect, useMemo, useRef } from "react";

import { useNavStore, type NavPage } from "@/stores/nav-store";

/**
 * Canonical list of keyboard shortcut action IDs. Kept in one place so
 * the customization UI can enumerate them.
 *
 * Window-scoped IDs (navigation + global utilities) are dispatched by
 * `useShortcuts` below. The `player_*` IDs are dispatched by
 * `usePlayerShortcuts` (sibling hook in `features/player`) — both
 * groups share the same `app_settings.shortcuts_json` table because
 * the backend is opaque about which scope an ID belongs to.
 */
export type ActionId =
  | "library"
  | "timeline"
  | "downloads"
  | "streamers"
  | "settings"
  | "focus_search"
  | "pause_all"
  | "add_streamer"
  | "shortcut_help"
  // --- Phase 6: player container scope ---
  | "player_play_pause"
  | "player_seek_back_5"
  | "player_seek_forward_5"
  | "player_seek_back_10"
  | "player_seek_forward_10"
  | "player_volume_up"
  | "player_volume_down"
  | "player_mute"
  | "player_fullscreen"
  | "player_pip"
  | "player_chapter_next"
  | "player_chapter_prev"
  | "player_speed_down"
  | "player_speed_up"
  | "player_close";

export const DEFAULT_SHORTCUTS: Record<ActionId, string> = {
  library: "g l",
  timeline: "g t",
  downloads: "g d",
  streamers: "g s",
  settings: "g ,",
  focus_search: "/",
  pause_all: "p",
  add_streamer: "n",
  shortcut_help: "?",
  // Player shortcuts — defaults mirror docs/user-guide/player.md and
  // the help-overlay strings in `player-constants.PLAYER_SHORTCUTS`.
  // Bindings use lowercased `e.key` form; `shift+` prefix only for
  // arrow / letter keys where the shifted character isn't a separate
  // glyph (e.g. `shift+c`, but `<` already encodes its shift).
  player_play_pause: "k",
  player_seek_back_5: "arrowleft",
  player_seek_forward_5: "arrowright",
  player_seek_back_10: "j",
  player_seek_forward_10: "l",
  player_volume_up: "arrowup",
  player_volume_down: "arrowdown",
  player_mute: "m",
  player_fullscreen: "f",
  player_pip: "p",
  player_chapter_next: "c",
  player_chapter_prev: "shift+c",
  player_speed_down: "<",
  player_speed_up: ">",
  player_close: "escape",
};

export const SHORTCUT_LABELS: Record<ActionId, string> = {
  library: "Go to Library",
  timeline: "Go to Timeline",
  downloads: "Go to Downloads",
  streamers: "Go to Streamers",
  settings: "Go to Settings",
  focus_search: "Focus search",
  pause_all: "Pause all downloads",
  add_streamer: "Add streamer",
  shortcut_help: "Show shortcut help",
  player_play_pause: "Play / pause",
  player_seek_back_5: "Seek back 5 s",
  player_seek_forward_5: "Seek forward 5 s",
  player_seek_back_10: "Seek back 10 s",
  player_seek_forward_10: "Seek forward 10 s",
  player_volume_up: "Volume up",
  player_volume_down: "Volume down",
  player_mute: "Mute toggle",
  player_fullscreen: "Toggle fullscreen",
  player_pip: "Toggle picture-in-picture",
  player_chapter_next: "Next chapter",
  player_chapter_prev: "Previous chapter",
  player_speed_down: "Slow down playback",
  player_speed_up: "Speed up playback",
  player_close: "Close player / exit fullscreen",
};

/**
 * Action IDs the window-scoped `useShortcuts` hook dispatches. Player
 * IDs are filtered out so we don't accidentally fire player actions
 * while the user is on the library page.
 */
export const GLOBAL_ACTION_IDS: ReadonlySet<ActionId> = new Set<ActionId>([
  "library",
  "timeline",
  "downloads",
  "streamers",
  "settings",
  "focus_search",
  "pause_all",
  "add_streamer",
  "shortcut_help",
]);

export interface ShortcutHandlers {
  focusSearch?: () => void;
  pauseAll?: () => void;
  addStreamer?: () => void;
  showHelp?: () => void;
}

/**
 * Install a global keydown listener that honours the current shortcut
 * table. Handles single-key and two-key "chord" shortcuts (e.g.
 * `g l` to jump to the library). Skips when focus is inside an input /
 * textarea / contenteditable to avoid hijacking a user's typing.
 */
export function useShortcuts(
  shortcuts: Record<ActionId, string>,
  handlers: ShortcutHandlers
) {
  const setPage = useNavStore((s) => s.setPage);
  const handlersRef = useRef(handlers);
  handlersRef.current = handlers;

  // Filter to global IDs only — `usePlayerShortcuts` owns the
  // `player_*` actions and is scoped to its container, so dispatching
  // them from a window-scoped listener would fire player actions while
  // the user is anywhere else in the app.
  const bindings = useMemo(
    () =>
      buildBindings(
        Object.fromEntries(
          Object.entries(shortcuts).filter(([id]) =>
            GLOBAL_ACTION_IDS.has(id as ActionId)
          )
        ) as Record<ActionId, string>
      ),
    [shortcuts]
  );

  useEffect(() => {
    let chordBuffer: string | null = null;
    let chordTimer: ReturnType<typeof setTimeout> | undefined;

    function clearChord() {
      chordBuffer = null;
      if (chordTimer) clearTimeout(chordTimer);
    }

    function dispatchAction(id: ActionId) {
      switch (id) {
        case "library":
        case "timeline":
        case "downloads":
        case "streamers":
        case "settings":
          setPage(id satisfies NavPage);
          break;
        case "focus_search":
          handlersRef.current.focusSearch?.();
          break;
        case "pause_all":
          handlersRef.current.pauseAll?.();
          break;
        case "add_streamer":
          handlersRef.current.addStreamer?.();
          break;
        case "shortcut_help":
          handlersRef.current.showHelp?.();
          break;
      }
    }

    function handler(e: KeyboardEvent) {
      if (e.defaultPrevented) return;
      if (e.metaKey || e.ctrlKey || e.altKey) return; // reserve for browser / menu
      const target = e.target as HTMLElement | null;
      if (target) {
        const tag = target.tagName;
        if (tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT") return;
        if (target.isContentEditable) return;
      }
      const key = e.key.toLowerCase();
      if (chordBuffer) {
        const combined = `${chordBuffer} ${key}`;
        const hit = bindings.get(combined);
        clearChord();
        if (hit) {
          e.preventDefault();
          dispatchAction(hit);
        }
        return;
      }
      // Single-key hit wins.
      const hit = bindings.get(key);
      if (hit) {
        e.preventDefault();
        dispatchAction(hit);
        return;
      }
      // Chord start?
      if (bindings.has(`${key} chord`)) {
        chordBuffer = key;
        chordTimer = setTimeout(clearChord, 1200);
      }
    }

    window.addEventListener("keydown", handler);
    return () => {
      window.removeEventListener("keydown", handler);
      clearChord();
    };
  }, [bindings, setPage]);
}

function buildBindings(shortcuts: Record<ActionId, string>): Map<string, ActionId> {
  // Two spellings per chord shortcut: the literal "g l" key, and a
  // marker "g chord" so we know whether the first key starts a chord.
  const out = new Map<string, ActionId>();
  for (const [action, combo] of Object.entries(shortcuts)) {
    const id = action as ActionId;
    const trimmed = combo.trim().toLowerCase();
    if (!trimmed) continue;
    if (trimmed.includes(" ")) {
      out.set(trimmed, id);
      const first = trimmed.split(" ")[0];
      if (first) {
        out.set(`${first} chord`, id);
      }
    } else {
      out.set(trimmed, id);
    }
  }
  return out;
}
