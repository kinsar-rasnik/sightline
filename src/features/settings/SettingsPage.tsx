import { useEffect, useState } from "react";

import { Button } from "@/components/primitives/Button";
import { ErrorBanner } from "@/components/primitives/ErrorBanner";
import { CredentialsForm } from "@/features/credentials/CredentialsForm";
import {
  useLibraryInfo,
  useStagingInfo,
} from "@/features/downloads/use-downloads";
import { useSettings, useUpdateSettings } from "@/features/settings/use-settings";
import { commands } from "@/ipc";
import type {
  AppSettings,
  LibraryLayoutKind,
  QualityPreset,
  SettingsPatch,
} from "@/ipc";
import { formatBytes } from "@/lib/format";

const KNOWN_GAMES: Array<{ id: string; name: string }> = [
  { id: "32982", name: "Grand Theft Auto V" },
  { id: "515025", name: "Overwatch 2" },
  { id: "509658", name: "Just Chatting" },
  { id: "516575", name: "VALORANT" },
];

export function SettingsPage() {
  const { data, isLoading, isError, error } = useSettings();
  const update = useUpdateSettings();

  if (isLoading) return <LoadingSection />;
  if (isError || !data) return <ErrorBanner error={error} />;

  return (
    <div className="space-y-10">
      <CredentialsForm status={data.credentials} />
      <GameFilterSection
        selectedIds={data.enabledGameIds}
        onChange={(next) => update.mutate({ enabledGameIds: next })}
        pending={update.isPending}
        error={update.error}
      />
      <PollIntervalsSection
        floor={data.pollFloorSeconds}
        recent={data.pollRecentSeconds}
        ceiling={data.pollCeilingSeconds}
        concurrency={data.concurrencyCap}
        onSubmit={(patch) => update.mutate(patch)}
        pending={update.isPending}
      />
      <DownloadsAndStorageSection
        settings={data}
        onUpdate={(patch) => update.mutate(patch)}
        pending={update.isPending}
      />
      <AppearanceSection />
      <NotificationsSection
        settings={data}
        onUpdate={(patch) => update.mutate(patch)}
      />
      <AdvancedSection
        settings={data}
        onUpdate={(patch) => update.mutate(patch)}
      />
    </div>
  );
}

function AppearanceSection() {
  return (
    <section
      aria-labelledby="appearance-heading"
      className="space-y-2 border-t border-[--color-border] pt-6"
    >
      <h3 id="appearance-heading" className="text-base font-medium">
        Appearance
      </h3>
      <p className="text-xs text-[--color-muted] max-w-prose">
        Sightline follows your OS light/dark preference. See{" "}
        <a
          className="underline text-[--color-accent]"
          href="../../docs/design-tokens.md"
          target="_blank"
          rel="noreferrer"
        >
          design tokens
        </a>{" "}
        for the palette and motion variables.
      </p>
    </section>
  );
}

function NotificationsSection({
  settings,
  onUpdate,
}: {
  settings: AppSettings;
  onUpdate: (patch: SettingsPatch) => void;
}) {
  return (
    <section
      aria-labelledby="notifications-heading"
      className="space-y-3 border-t border-[--color-border] pt-6"
    >
      <h3 id="notifications-heading" className="text-base font-medium">
        Notifications
      </h3>
      <p className="text-xs text-[--color-muted] max-w-prose">
        Download and ingest alerts. Rate-limited: a burst of new VODs
        from favorites collapses to a single banner.
      </p>
      <div className="grid grid-cols-1 gap-2 max-w-md">
        <Toggle
          label="Enable notifications (master)"
          checked={settings.notificationsEnabled}
          onChange={(v) => onUpdate({ notificationsEnabled: v })}
        />
        <Toggle
          label="Download completed"
          checked={settings.notifyDownloadComplete}
          onChange={(v) => onUpdate({ notifyDownloadComplete: v })}
          disabled={!settings.notificationsEnabled}
        />
        <Toggle
          label="Download failed (always recommended)"
          checked={settings.notifyDownloadFailed}
          onChange={(v) => onUpdate({ notifyDownloadFailed: v })}
          disabled={!settings.notificationsEnabled}
        />
        <Toggle
          label="New VODs from favorite streamers"
          checked={settings.notifyFavoritesIngest}
          onChange={(v) => onUpdate({ notifyFavoritesIngest: v })}
          disabled={!settings.notificationsEnabled}
        />
        <Toggle
          label="Storage low warning"
          checked={settings.notifyStorageLow}
          onChange={(v) => onUpdate({ notifyStorageLow: v })}
          disabled={!settings.notificationsEnabled}
        />
      </div>
    </section>
  );
}

function AdvancedSection({
  settings,
  onUpdate,
}: {
  settings: AppSettings;
  onUpdate: (patch: SettingsPatch) => void;
}) {
  return (
    <section
      aria-labelledby="advanced-heading"
      className="space-y-3 border-t border-[--color-border] pt-6"
    >
      <h3 id="advanced-heading" className="text-base font-medium">
        Advanced
      </h3>

      <fieldset className="space-y-2">
        <legend className="text-xs text-[--color-muted]">
          When the window close button is clicked
        </legend>
        <div className="flex flex-wrap gap-2">
          {(
            [
              ["hide", "Hide to tray (keep polling)"],
              ["quit", "Quit"],
            ] as const
          ).map(([value, label]) => (
            <button
              key={value}
              type="button"
              aria-pressed={settings.windowCloseBehavior === value}
              onClick={() => onUpdate({ windowCloseBehavior: value })}
              className={`text-xs px-3 py-1.5 rounded-full border transition-colors ${
                settings.windowCloseBehavior === value
                  ? "bg-[--color-accent] text-white border-transparent"
                  : "bg-transparent text-[--color-fg] border-[--color-border] hover:bg-[--color-surface]"
              }`}
            >
              {label}
            </button>
          ))}
        </div>
      </fieldset>

      <Toggle
        label="Start Sightline at login (opt-in)"
        checked={settings.startAtLogin}
        onChange={(v) => onUpdate({ startAtLogin: v })}
      />
      <Toggle
        label="Show dock icon on macOS (if hidden, use the menu bar)"
        checked={settings.showDockIcon}
        onChange={(v) => onUpdate({ showDockIcon: v })}
      />

      <div className="pt-2 text-xs text-[--color-muted] max-w-prose">
        <p>
          Keyboard shortcuts can be customised — press <kbd>?</kbd> from
          anywhere to see the current bindings. A settings UI for custom
          keys lands alongside the Phase 4 polish follow-up.
        </p>
      </div>
    </section>
  );
}

function Toggle({
  label,
  checked,
  onChange,
  disabled,
}: {
  label: string;
  checked: boolean;
  onChange: (v: boolean) => void;
  disabled?: boolean;
}) {
  return (
    <label className="flex items-center gap-2 text-sm">
      <input
        type="checkbox"
        checked={checked}
        onChange={(e) => onChange(e.target.checked)}
        disabled={disabled}
        className="accent-[--color-accent]"
      />
      <span className={disabled ? "text-[--color-subtle]" : undefined}>
        {label}
      </span>
    </label>
  );
}

function LoadingSection() {
  return (
    <p className="text-sm text-[--color-muted]" role="status">
      Loading settings…
    </p>
  );
}

function GameFilterSection({
  selectedIds,
  onChange,
  pending,
  error,
}: {
  selectedIds: string[];
  onChange: (ids: string[]) => void;
  pending: boolean;
  error: unknown;
}) {
  const toggle = (id: string) => {
    if (selectedIds.includes(id)) {
      onChange(selectedIds.filter((g) => g !== id));
    } else {
      onChange([...selectedIds, id]);
    }
  };
  return (
    <section
      aria-labelledby="game-filter-heading"
      className="space-y-3 border-t border-[--color-border] pt-6"
    >
      <h3 id="game-filter-heading" className="text-base font-medium">
        Game filter
      </h3>
      <p className="text-xs text-[--color-muted] max-w-prose">
        A VOD becomes <em>eligible</em> when at least one chapter matches an
        enabled game. Unrecognized-game VODs are surfaced for review rather
        than being filtered silently.
      </p>
      <div className="flex flex-wrap gap-2">
        {KNOWN_GAMES.map((g) => {
          const active = selectedIds.includes(g.id);
          return (
            <button
              key={g.id}
              type="button"
              onClick={() => toggle(g.id)}
              aria-pressed={active}
              className={`text-xs px-3 py-1.5 rounded-full border transition-colors ${
                active
                  ? "bg-[--color-accent] text-white border-transparent"
                  : "bg-transparent text-[--color-fg] border-[--color-border] hover:bg-[--color-surface]"
              }`}
            >
              {g.name}
            </button>
          );
        })}
      </div>
      {pending && <p className="text-xs text-[--color-muted]">Saving…</p>}
      <ErrorBanner error={error} />
    </section>
  );
}

function PollIntervalsSection({
  floor,
  recent,
  ceiling,
  concurrency,
  onSubmit,
  pending,
}: {
  floor: number;
  recent: number;
  ceiling: number;
  concurrency: number;
  onSubmit: (patch: SettingsPatch) => void;
  pending: boolean;
}) {
  const [localFloor, setFloor] = useState(floor);
  const [localRecent, setRecent] = useState(recent);
  const [localCeiling, setCeiling] = useState(ceiling);
  const [localConcurrency, setConcurrency] = useState(concurrency);

  useEffect(() => setFloor(floor), [floor]);
  useEffect(() => setRecent(recent), [recent]);
  useEffect(() => setCeiling(ceiling), [ceiling]);
  useEffect(() => setConcurrency(concurrency), [concurrency]);

  const dirty =
    localFloor !== floor ||
    localRecent !== recent ||
    localCeiling !== ceiling ||
    localConcurrency !== concurrency;

  return (
    <section
      aria-labelledby="poll-intervals-heading"
      className="space-y-3 border-t border-[--color-border] pt-6"
    >
      <h3 id="poll-intervals-heading" className="text-base font-medium">
        Polling intervals
      </h3>
      <p className="text-xs text-[--color-muted] max-w-prose">
        Sightline polls a streamer more often when they are live or recently
        streamed, and rarely when dormant. Jitter (±10%) is applied to avoid
        thundering-herd effects.
      </p>
      <div className="grid grid-cols-2 gap-4 max-w-md">
        <IntervalField
          label="Live (floor)"
          value={localFloor}
          setValue={setFloor}
          min={60}
          max={1800}
          step={60}
        />
        <IntervalField
          label="Recent (last 24h)"
          value={localRecent}
          setValue={setRecent}
          min={120}
          max={7200}
          step={60}
        />
        <IntervalField
          label="Dormant (ceiling)"
          value={localCeiling}
          setValue={setCeiling}
          min={600}
          max={86400}
          step={300}
        />
        <IntervalField
          label="Concurrency"
          value={localConcurrency}
          setValue={setConcurrency}
          min={1}
          max={16}
          step={1}
          unit=""
        />
      </div>
      <Button
        variant="primary"
        disabled={!dirty || pending}
        onClick={() =>
          onSubmit({
            pollFloorSeconds: localFloor,
            pollRecentSeconds: localRecent,
            pollCeilingSeconds: localCeiling,
            concurrencyCap: localConcurrency,
          })
        }
      >
        {pending ? "Saving…" : "Save intervals"}
      </Button>
    </section>
  );
}

function DownloadsAndStorageSection({
  settings,
  onUpdate,
  pending,
}: {
  settings: AppSettings;
  onUpdate: (patch: SettingsPatch) => void;
  pending: boolean;
}) {
  const staging = useStagingInfo();
  const library = useLibraryInfo();
  const [libraryRoot, setLibraryRoot] = useState(settings.libraryRoot ?? "");

  useEffect(() => setLibraryRoot(settings.libraryRoot ?? ""), [settings.libraryRoot]);

  const onChangeLayout = async (next: LibraryLayoutKind) => {
    if (next === settings.libraryLayout) return;
    const confirmed = window.confirm(
      `Switch library layout to "${next}"? Any completed downloads will be reorganised in the background; new downloads will use the new layout immediately.`
    );
    if (!confirmed) return;
    try {
      await commands.migrateLibrary({ targetLayout: next });
    } catch (e) {
      console.warn("migrate_library failed", e);
    }
  };

  return (
    <section
      aria-labelledby="downloads-storage-heading"
      className="space-y-4 border-t border-[--color-border] pt-6"
    >
      <h3 id="downloads-storage-heading" className="text-base font-medium">
        Downloads &amp; Storage
      </h3>

      <div className="grid grid-cols-2 gap-6 max-w-2xl">
        <IntervalField
          label="Max concurrent downloads"
          value={settings.maxConcurrentDownloads}
          setValue={(n) => onUpdate({ maxConcurrentDownloads: n })}
          min={1}
          max={5}
          step={1}
          unit=""
        />
        <BandwidthLimit
          currentBps={settings.bandwidthLimitBps}
          onUpdate={(bps) => onUpdate({ bandwidthLimitBps: bps })}
          pending={pending}
        />
      </div>

      <fieldset className="space-y-2">
        <legend className="text-xs text-[--color-muted]">Quality preset</legend>
        <div className="flex flex-wrap gap-2">
          {(["source", "1080p60", "720p60", "480p"] as QualityPreset[]).map(
            (preset) => (
              <button
                key={preset}
                type="button"
                aria-pressed={settings.qualityPreset === preset}
                onClick={() => onUpdate({ qualityPreset: preset })}
                className={`text-xs px-3 py-1.5 rounded-full border transition-colors ${
                  settings.qualityPreset === preset
                    ? "bg-[--color-accent] text-white border-transparent"
                    : "bg-transparent text-[--color-fg] border-[--color-border] hover:bg-[--color-surface]"
                }`}
              >
                {preset}
              </button>
            )
          )}
        </div>
      </fieldset>

      <div className="space-y-1 max-w-2xl">
        <label className="block text-xs">
          <span className="text-[--color-muted]">Library root</span>
          <input
            type="text"
            value={libraryRoot}
            onChange={(e) => setLibraryRoot(e.target.value)}
            onBlur={() => {
              if (libraryRoot !== (settings.libraryRoot ?? "")) {
                onUpdate({ libraryRoot });
              }
            }}
            placeholder="/absolute/path/to/library"
            className="mt-1 w-full rounded border border-[--color-border] bg-[--color-surface] px-3 py-2 text-sm font-mono focus:outline focus:outline-2 focus:outline-[--color-accent]"
          />
        </label>
        <p className="text-[10px] text-[--color-muted]">
          {library.data?.freeBytes != null
            ? `${formatBytes(library.data.freeBytes)} free · ${library.data.fileCount} files`
            : "Library root not yet configured."}
        </p>
      </div>

      <fieldset className="space-y-2">
        <legend className="text-xs text-[--color-muted]">Library layout</legend>
        <div className="flex flex-wrap gap-2">
          {(["plex", "flat"] as LibraryLayoutKind[]).map((kind) => (
            <button
              key={kind}
              type="button"
              aria-pressed={settings.libraryLayout === kind}
              onClick={() => onChangeLayout(kind)}
              className={`text-xs px-3 py-1.5 rounded-full border transition-colors ${
                settings.libraryLayout === kind
                  ? "bg-[--color-accent] text-white border-transparent"
                  : "bg-transparent text-[--color-fg] border-[--color-border] hover:bg-[--color-surface]"
              }`}
            >
              {kind === "plex" ? "Plex / Jellyfin" : "Flat"}
            </button>
          ))}
        </div>
        <LayoutPreview kind={settings.libraryLayout} />
      </fieldset>

      <div className="text-xs text-[--color-muted] max-w-2xl space-y-1">
        <div>
          Staging: <span className="font-mono">{staging.data?.path ?? "…"}</span>
          {staging.data?.freeBytes != null && (
            <span> — {formatBytes(staging.data.freeBytes)} free</span>
          )}
        </div>
        {(staging.data?.staleFileCount ?? 0) > 0 && (
          <div>
            {staging.data?.staleFileCount} stale file(s) from prior downloads
            will be cleaned on next startup.
          </div>
        )}
      </div>

      <label className="flex items-center gap-2 text-xs">
        <input
          type="checkbox"
          checked={settings.autoUpdateYtDlp}
          onChange={(e) => onUpdate({ autoUpdateYtDlp: e.target.checked })}
          className="accent-[--color-accent]"
        />
        Auto-update yt-dlp on startup
      </label>
    </section>
  );
}

function LayoutPreview({ kind }: { kind: LibraryLayoutKind }) {
  const samples =
    kind === "plex"
      ? [
          "Sampler/Season 2026-04/Sampler - 2026-04-02 - GTA RP Heist [twitch-v1].mp4",
          "Sampler/Season 2026-04/Sampler - 2026-04-02 - GTA RP Heist [twitch-v1].nfo",
        ]
      : [
          "sampler/2026-04-02_v1_gta-rp-heist.mp4",
          "sampler/.thumbs/2026-04-02_v1_gta-rp-heist.jpg",
        ];
  return (
    <pre className="text-[10px] text-[--color-muted] bg-[--color-surface] border border-[--color-border] rounded px-3 py-2 font-mono overflow-x-auto">
      {samples.join("\n")}
    </pre>
  );
}

function BandwidthLimit({
  currentBps,
  onUpdate,
  pending,
}: {
  currentBps: number | null;
  onUpdate: (bps: number) => void;
  pending: boolean;
}) {
  const isUnlimited = currentBps == null;
  const mbps = isUnlimited ? 5 : Math.max(0, Math.round((currentBps ?? 0) / 1_048_576));
  return (
    <div className="text-xs space-y-1">
      <div className="flex items-center justify-between">
        <span className="text-[--color-muted]">Bandwidth</span>
        <label className="flex items-center gap-1 text-[10px]">
          <input
            type="checkbox"
            checked={isUnlimited}
            onChange={(e) => onUpdate(e.target.checked ? -1 : 5 * 1_048_576)}
            className="accent-[--color-accent]"
            disabled={pending}
          />
          Unlimited
        </label>
      </div>
      <div className="flex items-center gap-2">
        <input
          type="range"
          min={0}
          max={50}
          step={1}
          disabled={isUnlimited || pending}
          value={mbps}
          onChange={(e) => {
            const raw = Math.max(1, Number(e.target.value));
            onUpdate(raw * 1_048_576);
          }}
          className="flex-1 accent-[--color-accent] disabled:opacity-50"
          aria-label="Bandwidth limit (MB/s)"
        />
        <span className="w-20 text-right font-mono">
          {isUnlimited ? "∞" : `${Math.max(1, mbps)} MB/s`}
        </span>
      </div>
    </div>
  );
}

function IntervalField({
  label,
  value,
  setValue,
  min,
  max,
  step,
  unit = "s",
}: {
  label: string;
  value: number;
  setValue: (n: number) => void;
  min: number;
  max: number;
  step: number;
  unit?: string;
}) {
  return (
    <label className="block text-xs">
      <span className="text-[--color-muted]">{label}</span>
      <div className="mt-1 flex items-center gap-2">
        <input
          type="range"
          min={min}
          max={max}
          step={step}
          value={Math.min(Math.max(value, min), max)}
          onChange={(e) => setValue(Number(e.target.value))}
          className="flex-1 accent-[--color-accent]"
          aria-label={label}
        />
        <span className="w-20 text-right font-mono">
          {value}
          {unit}
        </span>
      </div>
    </label>
  );
}
