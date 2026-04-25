import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";

import { Button } from "@/components/primitives/Button";
import {
  useSkipUpdateVersion,
  useUpdateStatus,
} from "@/features/updater/use-update-status";
import {
  commands,
  events,
  type UpdaterUpdateAvailableEvent,
} from "@/ipc";

/**
 * Top-of-shell banner that surfaces an available update.  Reads
 * `update-status` to know whether anything is available; subscribes
 * to `updater:update_available` so a fresh check (manual or
 * scheduled) triggers an immediate render.
 *
 * Three actions:
 *   - "View release" — opens the release page via the backend
 *     `open_release_url` command (https-only, defence-in-depth).
 *   - "Skip this version" — persists into AppSettings.
 *   - "Remind me later" — session-only dismiss.
 */
export function UpdateBanner() {
  const status = useUpdateStatus();
  const skip = useSkipUpdateVersion();
  const [dismissed, setDismissed] = useState(false);

  useEffect(() => {
    if (!("__TAURI_INTERNALS__" in window)) return;
    let unsub: (() => void) | undefined;
    void (async () => {
      unsub = await listen<UpdaterUpdateAvailableEvent>(
        events.updaterUpdateAvailable,
        () => {
          // A fresh tick found a new release — un-dismiss so the
          // banner re-appears even if the user dismissed an older one.
          setDismissed(false);
          status.refetch();
        },
      );
    })();
    return () => unsub?.();
  }, [status]);

  if (dismissed) return null;
  if (!status.data) return null;
  if (!status.data.available) return null;
  if (status.data.skipVersion === status.data.available.version) return null;

  const info = status.data.available;
  return (
    <div
      role="status"
      aria-label="Update available"
      className="border-b border-amber-500/40 bg-amber-500/10 text-sm px-6 py-2 flex flex-wrap items-center gap-3"
    >
      <span className="font-medium">
        Sightline v{info.version} is available.
      </span>
      <span className="text-[--color-muted]">
        You're running v{status.data.currentVersion}.
      </span>
      <div className="ml-auto flex items-center gap-2">
        <Button
          type="button"
          variant="secondary"
          onClick={() => {
            void commands.openReleaseUrl({ url: info.releaseUrl });
          }}
        >
          View release
        </Button>
        <Button
          type="button"
          variant="ghost"
          onClick={() => skip.mutate(info.version)}
          disabled={skip.isPending}
        >
          Skip this version
        </Button>
        <Button
          type="button"
          variant="ghost"
          onClick={() => setDismissed(true)}
        >
          Remind me later
        </Button>
      </div>
    </div>
  );
}
