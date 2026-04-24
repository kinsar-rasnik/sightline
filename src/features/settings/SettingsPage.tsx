import { useEffect, useState } from "react";

import { Button } from "@/components/primitives/Button";
import { ErrorBanner } from "@/components/primitives/ErrorBanner";
import { CredentialsForm } from "@/features/credentials/CredentialsForm";
import { useSettings, useUpdateSettings } from "@/features/settings/use-settings";
import type { SettingsPatch } from "@/ipc";

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
    </div>
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
