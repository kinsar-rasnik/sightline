import { useHealth } from "@/hooks/use-health";
import { useNavStore, type NavPage } from "@/stores/nav-store";
import { LibraryPage } from "@/features/vods/LibraryPage";
import { SettingsPage } from "@/features/settings/SettingsPage";
import { StreamersPage } from "@/features/streamers/StreamersPage";

const NAV_ITEMS: Array<{ id: NavPage; label: string }> = [
  { id: "library", label: "Library" },
  { id: "streamers", label: "Streamers" },
  { id: "settings", label: "Settings" },
];

export function AppShell() {
  const page = useNavStore((s) => s.page);
  const setPage = useNavStore((s) => s.setPage);
  const health = useHealth();

  return (
    <div className="flex min-h-screen flex-col bg-[--color-bg] text-[--color-fg]">
      <header className="border-b border-[--color-border] px-6 py-3 flex items-center gap-6">
        <h1 className="text-base font-semibold tracking-tight">Sightline</h1>
        <nav aria-label="Main navigation" className="flex gap-1">
          {NAV_ITEMS.map((item) => (
            <button
              key={item.id}
              type="button"
              aria-current={page === item.id ? "page" : undefined}
              onClick={() => setPage(item.id)}
              className={`text-sm px-3 py-1.5 rounded transition-colors focus:outline focus:outline-2 focus:outline-[--color-accent] ${
                page === item.id
                  ? "bg-[--color-surface] text-[--color-fg]"
                  : "text-[--color-muted] hover:text-[--color-fg]"
              }`}
            >
              {item.label}
            </button>
          ))}
        </nav>
        <div className="ml-auto text-xs text-[--color-muted]">
          {health.data
            ? `v${health.data.appVersion} · schema v${health.data.schemaVersion}`
            : "…"}
        </div>
      </header>

      <main className="flex-1 px-6 py-6 min-h-0">
        {page === "library" && <LibraryPage />}
        {page === "streamers" && <StreamersPage />}
        {page === "settings" && <SettingsPage />}
      </main>
    </div>
  );
}
