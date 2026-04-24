import { useEffect, useMemo, useRef } from "react";

import { useNavStore, type NavPage } from "@/stores/nav-store";

/**
 * Canonical list of keyboard shortcut action IDs. Kept in one place so
 * the customization UI can enumerate them.
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
  | "shortcut_help";

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
};

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

  const bindings = useMemo(() => buildBindings(shortcuts), [shortcuts]);

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
