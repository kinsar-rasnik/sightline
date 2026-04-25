import { useEffect, useMemo, useRef, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { listen } from "@tauri-apps/api/event";

import { useHealth } from "@/hooks/use-health";
import { useNavStore, type NavPage } from "@/stores/nav-store";
import { LibraryPage } from "@/features/vods/LibraryPage";
import { DownloadsPage } from "@/features/downloads/DownloadsPage";
import { MultiViewPage } from "@/features/multiview/MultiViewPage";
import { PlayerPage } from "@/features/player/PlayerPage";
import { SettingsPage } from "@/features/settings/SettingsPage";
import { StreamersPage } from "@/features/streamers/StreamersPage";
import { TimelinePage } from "@/features/timeline/TimelinePage";
import { NotificationsToaster } from "@/features/notifications/NotificationsToaster";
import { UpdateBanner } from "@/features/updater/UpdateBanner";
import {
  DEFAULT_SHORTCUTS,
  SHORTCUT_LABELS,
  useShortcuts,
  type ActionId,
} from "@/features/shortcuts/use-shortcuts";
import { commands, events, type AppTrayActionEvent } from "@/ipc";

const NAV_ITEMS: Array<{ id: NavPage; label: string }> = [
  { id: "library", label: "Library" },
  { id: "timeline", label: "Timeline" },
  { id: "streamers", label: "Streamers" },
  { id: "downloads", label: "Downloads" },
  { id: "settings", label: "Settings" },
];

export function AppShell() {
  const page = useNavStore((s) => s.page);
  const watchContext = useNavStore((s) => s.watch);
  const multiViewContext = useNavStore((s) => s.multiView);
  const setPage = useNavStore((s) => s.setPage);
  const health = useHealth();
  const [helpOpen, setHelpOpen] = useState(false);
  const searchInputRef = useRef<HTMLInputElement>(null);

  const shortcutOverrides = useQuery({
    queryKey: ["shortcuts"],
    queryFn: () => commands.listShortcuts(),
    staleTime: 60_000,
  });

  const appSummary = useQuery({
    queryKey: ["app-summary"],
    queryFn: () => commands.getAppSummary(),
    refetchInterval: 10_000,
  });

  const shortcuts = useMemo(() => {
    const base: Record<ActionId, string> = { ...DEFAULT_SHORTCUTS };
    for (const row of shortcutOverrides.data ?? []) {
      if (row.actionId in base) {
        base[row.actionId as ActionId] = row.keys;
      }
    }
    return base;
  }, [shortcutOverrides.data]);

  useShortcuts(shortcuts, {
    focusSearch: () => searchInputRef.current?.focus(),
    pauseAll: () => void commands.pauseAllDownloads(),
    addStreamer: () => setPage("streamers"),
    showHelp: () => setHelpOpen(true),
  });

  // Tray → webview bus. The tray menu's clicks call
  // `emit_tray_action`; we narrow by `kind`.
  useEffect(() => {
    if (!("__TAURI_INTERNALS__" in window)) return;
    let unsub: (() => void) | undefined;
    void (async () => {
      unsub = await listen<AppTrayActionEvent>(events.appTrayAction, (ev) => {
        switch (ev.payload.kind) {
          case "open_sightline":
            break;
          case "open_downloads":
            setPage("downloads");
            break;
          case "open_library":
            setPage("library");
            break;
          case "open_timeline":
            setPage("timeline");
            break;
          case "pause_all":
            void commands.pauseAllDownloads();
            break;
          case "resume_all":
            void commands.resumeAllDownloads();
            break;
        }
      });
    })();
    return () => unsub?.();
  }, [setPage]);

  return (
    <div className="flex min-h-screen flex-col bg-[--color-bg] text-[--color-fg]">
      <a
        href="#main-content"
        className="sr-only focus:not-sr-only absolute top-2 left-2 z-50 bg-[--color-accent] text-[--color-accent-fg] px-3 py-1.5 rounded"
      >
        Skip to main content
      </a>
      <header
        className="border-b border-[--color-border] px-6 py-3 flex items-center gap-6"
        role="banner"
      >
        <h1 className="text-base font-semibold tracking-tight">Sightline</h1>
        <nav aria-label="Main navigation" className="flex gap-1">
          {NAV_ITEMS.map((item) => (
            <button
              key={item.id}
              type="button"
              aria-current={page === item.id ? "page" : undefined}
              onClick={() => setPage(item.id)}
              className={`text-sm px-3 py-1.5 rounded transition-colors ${
                page === item.id
                  ? "bg-[--color-surface] text-[--color-fg]"
                  : "text-[--color-muted] hover:text-[--color-fg]"
              }`}
            >
              {item.label}
            </button>
          ))}
        </nav>
        <div className="ml-auto flex items-center gap-3">
          <input
            ref={searchInputRef}
            type="search"
            placeholder="Search"
            aria-label="Search library"
            className="text-xs bg-[--color-surface] border border-[--color-border] rounded-full px-3 py-1.5 w-40 focus:w-60 transition-[width] duration-[var(--motion-fast)]"
          />
          {appSummary.data && appSummary.data.activeDownloads > 0 && (
            <span
              className="text-xs text-[--color-muted] font-mono"
              aria-live="polite"
              title="Active downloads"
            >
              ↓ {appSummary.data.activeDownloads}
            </span>
          )}
          <button
            type="button"
            onClick={() => setHelpOpen(true)}
            aria-label="Show keyboard shortcuts"
            className="text-xs text-[--color-muted] hover:text-[--color-fg]"
          >
            ?
          </button>
          <div className="text-xs text-[--color-muted]">
            {health.data
              ? `v${health.data.appVersion} · schema v${health.data.schemaVersion}`
              : "…"}
          </div>
        </div>
      </header>

      <UpdateBanner />

      <main
        id="main-content"
        className="flex-1 px-6 py-6 min-h-0"
        role="main"
        tabIndex={-1}
      >
        {page === "library" && <LibraryPage />}
        {page === "timeline" && <TimelinePage />}
        {page === "streamers" && <StreamersPage />}
        {page === "downloads" && <DownloadsPage />}
        {page === "settings" && <SettingsPage />}
        {page === "watch" && watchContext && (
          <PlayerPage
            vodId={watchContext.vodId}
            initialPositionSeconds={watchContext.initialPositionSeconds}
            autoplay={watchContext.autoplay}
          />
        )}
        {page === "multiview" && multiViewContext && (
          <MultiViewPage vodIds={multiViewContext.vodIds} />
        )}
      </main>

      <NotificationsToaster />

      {helpOpen && (
        <ShortcutHelp
          shortcuts={shortcuts}
          onClose={() => setHelpOpen(false)}
        />
      )}
    </div>
  );
}

function ShortcutHelp({
  shortcuts,
  onClose,
}: {
  shortcuts: Record<ActionId, string>;
  onClose: () => void;
}) {
  useEffect(() => {
    function onKey(e: KeyboardEvent) {
      if (e.key === "Escape") onClose();
    }
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose]);

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center p-4"
      role="dialog"
      aria-modal="true"
      aria-labelledby="shortcut-help-title"
    >
      <button
        type="button"
        aria-label="Close shortcut help"
        onClick={onClose}
        className="absolute inset-0 bg-black/50"
      />
      <div className="relative max-w-md w-full bg-[--color-surface-elevated] rounded-[var(--radius-lg)] border border-[--color-border] shadow-[var(--shadow-lg)] p-6">
        <h2 id="shortcut-help-title" className="text-base font-semibold mb-4">
          Keyboard shortcuts
        </h2>
        <dl className="space-y-2 text-sm">
          {(Object.keys(SHORTCUT_LABELS) as ActionId[]).map((id) => (
            <div key={id} className="flex justify-between gap-4">
              <dt className="text-[--color-muted]">{SHORTCUT_LABELS[id]}</dt>
              <dd>
                <kbd className="font-mono text-xs bg-[--color-surface] border border-[--color-border] rounded px-1.5 py-0.5">
                  {shortcuts[id]}
                </kbd>
              </dd>
            </div>
          ))}
        </dl>
        <button
          type="button"
          onClick={onClose}
          className="mt-6 text-xs text-[--color-muted] hover:text-[--color-fg]"
        >
          Close (Esc)
        </button>
      </div>
    </div>
  );
}
