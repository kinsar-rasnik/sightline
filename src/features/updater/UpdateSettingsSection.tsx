import { Button } from "@/components/primitives/Button";
import {
  useCheckForUpdate,
  useSkipUpdateVersion,
  useUpdateStatus,
} from "@/features/updater/use-update-status";
import { commands, type AppSettings, type SettingsPatch } from "@/ipc";
import { formatUnixSeconds } from "@/lib/format";

interface Props {
  settings: AppSettings;
  onUpdate: (patch: SettingsPatch) => void;
  pending: boolean;
}

/**
 * Settings → Updates section.  Owns the master toggle, the
 * last-checked timestamp display, the manual "Check now" button,
 * and the inverse "Don't skip" affordance for restoring the banner.
 */
export function UpdateSettingsSection({ settings, pending, onUpdate }: Props) {
  const status = useUpdateStatus();
  const check = useCheckForUpdate();
  const skip = useSkipUpdateVersion();

  return (
    <section
      aria-labelledby="updates-heading"
      className="space-y-3 border-t border-[--color-border] pt-6"
    >
      <h3 id="updates-heading" className="text-base font-medium">
        Updates
      </h3>

      <Toggle
        label="Check for updates"
        description="Once per day, query GitHub Releases for a newer Sightline version. No telemetry — only the latest tag is fetched."
        checked={settings.updateCheckEnabled}
        disabled={pending}
        onChange={(v) => onUpdate({ updateCheckEnabled: v })}
      />

      <p className="text-xs text-[--color-muted]">
        Currently running <span className="font-mono">v{status.data?.currentVersion ?? "—"}</span>.
        Last checked: {formatUnixSeconds(status.data?.lastCheckedAt)}.
      </p>

      {status.data?.available && (
        <p className="text-xs">
          v{status.data.available.version} is available.{" "}
          <a
            className="underline text-[--color-accent]"
            href={status.data.available.releaseUrl}
            onClick={(e) => {
              e.preventDefault();
              const url = status.data?.available?.releaseUrl;
              if (url) {
                void commands.openReleaseUrl({ url });
              }
            }}
          >
            View release notes
          </a>
        </p>
      )}

      <div className="flex flex-wrap gap-2 items-center">
        <Button
          type="button"
          variant="secondary"
          onClick={() => check.mutate({ force: true })}
          disabled={check.isPending}
        >
          {check.isPending ? "Checking…" : "Check now"}
        </Button>
        {status.data?.skipVersion && (
          <Button
            type="button"
            variant="ghost"
            onClick={() => skip.mutate("")}
            disabled={skip.isPending}
          >
            Don't skip v{status.data.skipVersion}
          </Button>
        )}
      </div>

      {check.error && (
        <p
          role="alert"
          className="text-xs text-red-500 max-w-prose"
        >
          Couldn't reach GitHub: {String(check.error)}
        </p>
      )}
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
