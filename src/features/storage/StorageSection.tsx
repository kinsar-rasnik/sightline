import { useState } from "react";

import { Button } from "@/components/primitives/Button";
import {
  useCleanupHistory,
  useDiskUsage,
} from "@/features/storage/use-cleanup";
import { CleanupHistoryView } from "@/features/storage/CleanupHistoryView";
import { CleanupPlanDrawer } from "@/features/storage/CleanupPlanDrawer";
import type { AppSettings, SettingsPatch } from "@/ipc";
import { formatBytes } from "@/lib/format";

interface Props {
  settings: AppSettings;
  onUpdate: (patch: SettingsPatch) => void;
  pending: boolean;
}

/**
 * Settings → Storage section.  Reports disk usage, owns the
 * cleanup-toggle + watermark sliders, and triggers the plan-preview
 * drawer.
 */
export function StorageSection({ settings, onUpdate, pending }: Props) {
  const [planOpen, setPlanOpen] = useState(false);
  const usage = useDiskUsage();
  const history = useCleanupHistory(10);

  const usedPct = usage.data ? Math.round(usage.data.usedFraction * 100) : 0;
  const aboveHigh = usage.data?.aboveHighWatermark ?? false;

  return (
    <section
      aria-labelledby="storage-heading"
      className="space-y-4 border-t border-[--color-border] pt-6"
    >
      <h3 id="storage-heading" className="text-base font-medium">
        Storage
      </h3>

      <div className="rounded border border-[--color-border] p-3 text-xs space-y-2">
        <div className="flex items-center justify-between">
          <div>
            <div className="text-sm font-medium">
              {usage.data?.libraryPath
                ? `Library disk: ${usedPct}% used`
                : "No library root configured"}
            </div>
            {usage.data?.libraryPath && (
              <div className="text-[--color-muted]">
                {formatBytes(
                  usage.data.totalBytes - usage.data.freeBytes
                )}{" "}
                of {formatBytes(usage.data.totalBytes)} ·{" "}
                {formatBytes(usage.data.freeBytes)} free
              </div>
            )}
          </div>
          {aboveHigh && (
            <span
              role="status"
              aria-label="Disk pressure above high watermark"
              className="rounded bg-amber-500/20 text-amber-600 px-2 py-1 text-[11px]"
            >
              Above high watermark
            </span>
          )}
        </div>

        {usage.data?.libraryPath && (
          <div
            role="progressbar"
            aria-valuenow={usedPct}
            aria-valuemin={0}
            aria-valuemax={100}
            aria-label="Library disk used"
            className="h-2 w-full rounded bg-[--color-surface] overflow-hidden"
          >
            <div
              className={`h-full ${aboveHigh ? "bg-amber-500" : "bg-[--color-accent]"}`}
              style={{ width: `${Math.min(100, usedPct)}%` }}
            />
          </div>
        )}
      </div>

      <Toggle
        label="Enable auto-cleanup"
        description="Delete watched VODs to keep the library disk under the high watermark."
        checked={settings.cleanupEnabled}
        disabled={pending}
        onChange={(v) => onUpdate({ cleanupEnabled: v })}
      />

      <RangeField
        id="cleanup-high-watermark"
        label="High watermark"
        min={0.5}
        max={0.99}
        step={0.01}
        value={settings.cleanupHighWatermark}
        format={(v) => `${Math.round(v * 100)}%`}
        onChange={(v) => onUpdate({ cleanupHighWatermark: v })}
        description="When the library disk crosses this, cleanup fires."
      />

      <RangeField
        id="cleanup-low-watermark"
        label="Low watermark"
        min={0.4}
        max={0.95}
        step={0.01}
        value={settings.cleanupLowWatermark}
        format={(v) => `${Math.round(v * 100)}%`}
        onChange={(v) => onUpdate({ cleanupLowWatermark: v })}
        description="Cleanup keeps deleting until used space drops to this fraction."
      />

      <RangeField
        id="cleanup-schedule-hour"
        label="Run at hour (local)"
        min={0}
        max={23}
        step={1}
        value={settings.cleanupScheduleHour}
        format={(v) => `${String(v).padStart(2, "0")}:00`}
        onChange={(v) => onUpdate({ cleanupScheduleHour: Math.round(v) })}
        description="Once per day, at this hour, cleanup considers a new run."
      />

      <div className="flex items-center gap-2">
        <Button
          type="button"
          variant="secondary"
          onClick={() => setPlanOpen(true)}
        >
          Preview cleanup plan…
        </Button>
        <span className="text-xs text-[--color-muted]">
          Lists what would be deleted; no files are touched until you confirm.
        </span>
      </div>

      <CleanupHistoryView entries={history.data} />

      <CleanupPlanDrawer
        open={planOpen}
        onClose={() => setPlanOpen(false)}
      />
    </section>
  );
}

function Toggle({
  label,
  description,
  checked,
  onChange,
  disabled,
}: {
  label: string;
  description?: string;
  checked: boolean;
  onChange: (v: boolean) => void;
  disabled?: boolean;
}) {
  return (
    <label className="flex items-start gap-3 text-sm">
      <input
        type="checkbox"
        checked={checked}
        disabled={disabled}
        onChange={(e) => onChange(e.target.checked)}
        className="mt-1"
      />
      <span className="flex flex-col">
        <span>{label}</span>
        {description && (
          <span className="text-xs text-[--color-muted] max-w-prose">
            {description}
          </span>
        )}
      </span>
    </label>
  );
}

function RangeField({
  id,
  label,
  min,
  max,
  step,
  value,
  format,
  onChange,
  description,
}: {
  id: string;
  label: string;
  min: number;
  max: number;
  step: number;
  value: number;
  format: (v: number) => string;
  onChange: (v: number) => void;
  description?: string;
}) {
  return (
    <div className="flex flex-col gap-1 text-xs">
      <label
        htmlFor={id}
        className="flex items-center justify-between text-[--color-muted]"
      >
        <span>{label}</span>
        <span className="font-mono text-[--color-fg]">{format(value)}</span>
      </label>
      <input
        id={id}
        type="range"
        min={min}
        max={max}
        step={step}
        value={value}
        onChange={(e) => onChange(Number(e.target.value))}
      />
      {description && (
        <span className="text-[--color-muted] max-w-prose">{description}</span>
      )}
    </div>
  );
}
