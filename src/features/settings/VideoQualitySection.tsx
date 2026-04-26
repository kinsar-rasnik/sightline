import { useState } from "react";

import { Button } from "@/components/primitives/Button";
import { ErrorBanner } from "@/components/primitives/ErrorBanner";
import {
  useEncoderCapability,
  useRedetectEncoders,
  useSetVideoQualityProfile,
} from "@/features/settings/use-quality";
import type {
  AppSettings,
  EncoderKind,
  SettingsPatch,
  VideoQualityProfile,
} from "@/ipc";

/**
 * Catalog of profile metadata used by the picker.  Mirrors the
 * domain `VideoQualityProfile` enum; ADR-0028 + ADR-0032 are the
 * source of truth for the GB/hour numbers.
 */
const PROFILES: Array<{
  value: VideoQualityProfile;
  label: string;
  gbPerHour: number;
  warning?: "low-resolution" | "high-bandwidth";
}> = [
  { value: "480p30", label: "480p · 30 fps · H.265", gbPerHour: 0.30, warning: "low-resolution" },
  { value: "480p60", label: "480p · 60 fps · H.265", gbPerHour: 0.45, warning: "low-resolution" },
  { value: "720p30", label: "720p · 30 fps · H.265 (recommended)", gbPerHour: 0.70 },
  { value: "720p60", label: "720p · 60 fps · H.265", gbPerHour: 1.10 },
  { value: "1080p30", label: "1080p · 30 fps · H.265", gbPerHour: 1.50, warning: "high-bandwidth" },
  { value: "1080p60", label: "1080p · 60 fps · H.265", gbPerHour: 2.20, warning: "high-bandwidth" },
  { value: "source", label: "Source · no re-encode (passthrough)", gbPerHour: 4.00, warning: "high-bandwidth" },
];

function exampleMath(gbPerHour: number, downloadMbps = 50): {
  sixHourGb: string;
  downloadMinutes: string;
} {
  const sixHourGb = (gbPerHour * 6).toFixed(1);
  // 6 h × gb/h × 8 bits/byte × 1024 MB/GB / Mbps / 60 = minutes.
  const minutes = (gbPerHour * 6 * 1024 * 8) / downloadMbps / 60;
  return {
    sixHourGb,
    downloadMinutes: minutes >= 60
      ? `${(minutes / 60).toFixed(1)} h`
      : `${Math.round(minutes)} min`,
  };
}

const ENCODER_LABELS: Record<EncoderKind, string> = {
  video_toolbox: "VideoToolbox (Apple Silicon / Intel Mac)",
  nvenc: "NVENC (NVIDIA)",
  amf: "AMF (AMD)",
  quick_sync: "QuickSync (Intel)",
  vaapi: "VAAPI (Linux hardware)",
  software: "Software (libx265 / libx264)",
};

export function VideoQualitySection({
  settings,
  onUpdate,
  pending,
}: {
  settings: AppSettings;
  onUpdate: (patch: SettingsPatch) => void;
  pending: boolean;
}) {
  const cap = useEncoderCapability();
  const redetect = useRedetectEncoders();
  const setProfile = useSetVideoQualityProfile();
  const [showAdvanced, setShowAdvanced] = useState(false);

  const isUsingSoftware = cap.data?.primary === "software";
  const softwareWithoutOptIn =
    isUsingSoftware && !settings.softwareEncodeOptIn;

  return (
    <section
      aria-labelledby="video-quality-heading"
      className="space-y-4 border-t border-[--color-border] pt-6"
    >
      <header>
        <h3
          id="video-quality-heading"
          className="text-base font-medium"
        >
          Video quality
        </h3>
        <p className="text-sm text-[--color-text-muted]">
          Default 720p30 H.265 keeps storage and download sizes
          reasonable for everyday viewing. Higher tiers preserve more
          detail but cost considerably more disk and bandwidth.
        </p>
      </header>

      {/* Profile picker */}
      <fieldset className="space-y-2">
        <legend className="sr-only">Quality profile</legend>
        {PROFILES.map((p) => {
          const ex = exampleMath(p.gbPerHour);
          const checked = settings.videoQualityProfile === p.value;
          return (
            <label
              key={p.value}
              className={`flex cursor-pointer items-start gap-3 rounded-md border p-3 ${
                checked
                  ? "border-[--color-primary] bg-[--color-surface-hover]"
                  : "border-[--color-border] hover:bg-[--color-surface-hover]"
              }`}
            >
              <input
                type="radio"
                name="quality-profile"
                value={p.value}
                checked={checked}
                onChange={() => setProfile.mutate(p.value)}
                disabled={pending || setProfile.isPending}
                className="mt-1"
                aria-label={`${p.label}, approximately ${ex.sixHourGb} GB per 6-hour VOD`}
              />
              <div className="flex-1">
                <div className="font-medium">{p.label}</div>
                <div className="text-xs text-[--color-text-muted]">
                  ≈ {ex.sixHourGb} GB / 6 h VOD · downloads in{" "}
                  {ex.downloadMinutes} at 50 Mbit/s
                </div>
                {p.warning === "low-resolution" && (
                  <div className="mt-1 text-xs text-amber-600 dark:text-amber-400">
                    Game text and Discord overlays will be hard to
                    read at this quality.
                  </div>
                )}
                {p.warning === "high-bandwidth" && (
                  <div className="mt-1 text-xs text-amber-600 dark:text-amber-400">
                    Disk usage and download bandwidth will be
                    significantly higher.
                  </div>
                )}
              </div>
            </label>
          );
        })}
      </fieldset>

      {setProfile.isError && <ErrorBanner error={setProfile.error} />}

      {/* Hardware encoder status */}
      <div className="rounded-md border border-[--color-border] p-3">
        <div className="flex items-center justify-between gap-4">
          <div>
            <div className="text-sm font-medium">Hardware encoder</div>
            <div className="text-xs text-[--color-text-muted]">
              {cap.isLoading
                ? "Detecting…"
                : cap.data
                ? `Auto-detected: ${ENCODER_LABELS[cap.data.primary]}`
                : "Not yet detected"}
            </div>
            {cap.data && !cap.data.h265 && (
              <div className="mt-1 text-xs text-amber-600 dark:text-amber-400">
                The detected encoder does not support H.265 on this
                machine. Re-encodes will fail unless you select the
                Source profile.
              </div>
            )}
            {softwareWithoutOptIn && (
              <div className="mt-1 text-xs text-amber-600 dark:text-amber-400">
                No hardware encoder available. Enable "Allow software
                encoding" below to use libx265 — note that software
                encoding uses 100 % of your CPU and may slow down
                games or streaming during re-encodes.
              </div>
            )}
          </div>
          <Button
            type="button"
            variant="secondary"
            disabled={redetect.isPending}
            onClick={() => redetect.mutate()}
          >
            {redetect.isPending ? "Re-detecting…" : "Re-detect"}
          </Button>
        </div>
        {redetect.isError && <ErrorBanner error={redetect.error} />}
      </div>

      {/* Software fallback opt-in */}
      <div className="flex items-start gap-2">
        <input
          id="quality-software-opt-in"
          type="checkbox"
          checked={settings.softwareEncodeOptIn}
          onChange={(e) =>
            onUpdate({ softwareEncodeOptIn: e.target.checked })
          }
          disabled={pending}
          className="mt-1"
          aria-label="Allow software encoding (libx265 fallback)"
        />
        <label
          htmlFor="quality-software-opt-in"
          className="cursor-pointer text-sm"
        >
          <span className="block">Allow software encoding (libx265 fallback)</span>
          <span className="block text-xs text-[--color-text-muted]">
            Required when no hardware encoder is available.
            Recommended only when no hardware encoder is available —
            software encoding can saturate your CPU during gaming.
          </span>
        </label>
      </div>

      {/* Advanced disclosure */}
      <button
        type="button"
        className="text-xs text-[--color-text-muted] underline"
        onClick={() => setShowAdvanced((v) => !v)}
        aria-expanded={showAdvanced}
      >
        {showAdvanced ? "Hide" : "Show"} advanced (concurrency &
        throttle thresholds)
      </button>

      {showAdvanced && (
        <div className="space-y-3 rounded-md bg-[--color-surface-subtle] p-3">
          <Slider
            label="Maximum concurrent re-encodes"
            value={settings.maxConcurrentReencodes}
            min={1}
            max={2}
            step={1}
            onChange={(v) => onUpdate({ maxConcurrentReencodes: v })}
            disabled={pending}
            description="Hard-capped at 2; the throttle only handles one suspended encode at a time."
          />
          <Slider
            label="CPU throttle: high threshold (suspend above)"
            value={settings.cpuThrottleHighThreshold}
            min={0.5}
            max={0.9}
            step={0.05}
            onChange={(v) =>
              onUpdate({ cpuThrottleHighThreshold: v })
            }
            disabled={pending}
            description="System-wide CPU load above this fraction (sustained 30 s) suspends the running ffmpeg encoder."
            renderValue={(v) => `${Math.round(v * 100)}%`}
          />
          <Slider
            label="CPU throttle: low threshold (resume below)"
            value={settings.cpuThrottleLowThreshold}
            min={0.3}
            max={0.8}
            step={0.05}
            onChange={(v) =>
              onUpdate({ cpuThrottleLowThreshold: v })
            }
            disabled={pending}
            description="System-wide CPU load below this fraction (sustained 30 s) resumes a suspended encoder."
            renderValue={(v) => `${Math.round(v * 100)}%`}
          />
          <p className="text-xs text-[--color-text-muted]">
            On Windows v2.0 the throttle records the decision but
            does not yet suspend ffmpeg. SuspendThread support will
            land in v2.1.
          </p>
        </div>
      )}
    </section>
  );
}

function Slider({
  label,
  value,
  min,
  max,
  step,
  onChange,
  disabled,
  description,
  renderValue,
}: {
  label: string;
  value: number;
  min: number;
  max: number;
  step: number;
  onChange: (next: number) => void;
  disabled?: boolean;
  description?: string;
  renderValue?: (v: number) => string;
}) {
  return (
    <label className="block space-y-1">
      <div className="flex items-center justify-between text-sm">
        <span>{label}</span>
        <span className="font-mono text-xs">
          {renderValue ? renderValue(value) : value}
        </span>
      </div>
      <input
        type="range"
        min={min}
        max={max}
        step={step}
        value={value}
        onChange={(e) => onChange(Number(e.target.value))}
        disabled={disabled}
        className="w-full"
      />
      {description && (
        <div className="text-xs text-[--color-text-muted]">
          {description}
        </div>
      )}
    </label>
  );
}
