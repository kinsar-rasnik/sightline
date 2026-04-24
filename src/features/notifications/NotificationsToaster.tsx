import { useEffect, useState } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

import { events, type NotificationPayload } from "@/ipc";

/**
 * In-app mirror of native notifications. The backend emits
 * `notification:show` on the Tauri bus; this component subscribes,
 * renders a transient toast, and auto-dismisses. Rate-limiting lives
 * in the Rust service, so this component trusts each emit to already
 * represent a "show me" decision.
 *
 * Runs fully client-side; the native OS banner is the responsibility
 * of the host shell (Tauri). If `@tauri-apps/plugin-notification` is
 * available and the user has granted permission, we also dispatch a
 * native banner.
 */
export function NotificationsToaster() {
  const [toasts, setToasts] = useState<ToastRow[]>([]);

  useEffect(() => {
    if (!("__TAURI_INTERNALS__" in window)) return;
    let unsub: UnlistenFn | undefined;
    let cancelled = false;
    void (async () => {
      unsub = await listen<NotificationPayload>(events.notificationShow, (ev) => {
        if (cancelled) return;
        const row: ToastRow = { id: crypto.randomUUID(), payload: ev.payload };
        setToasts((list) => [row, ...list].slice(0, 5));
        window.setTimeout(() => {
          setToasts((list) => list.filter((t) => t.id !== row.id));
        }, 5_000);
        fireNativeBanner(ev.payload);
      });
    })();
    return () => {
      cancelled = true;
      unsub?.();
    };
  }, []);

  return (
    <div
      className="fixed z-50 bottom-4 right-4 flex flex-col gap-2 max-w-sm"
      role="region"
      aria-label="Notifications"
      aria-live="polite"
    >
      {toasts.map((t) => (
        <div
          key={t.id}
          className="sightline-slide-in rounded-[var(--radius-md)] border border-[--color-border] bg-[--color-surface-elevated] px-4 py-3 shadow-[var(--shadow-md)]"
          role="status"
        >
          <div className="text-sm font-medium">{t.payload.title}</div>
          <div className="text-xs text-[--color-muted] mt-1">{t.payload.body}</div>
          {t.payload.coalesced > 1 && (
            <div className="text-[10px] uppercase tracking-wider text-[--color-subtle] mt-2">
              Grouped: {t.payload.coalesced}
            </div>
          )}
        </div>
      ))}
    </div>
  );
}

interface ToastRow {
  id: string;
  payload: NotificationPayload;
}

function fireNativeBanner(payload: NotificationPayload) {
  // Only try the Tauri plugin; silently skip if not present (dev
  // browser mode, or a build without the plugin enabled yet).
  const plugin = (window as unknown as {
    __TAURI_PLUGIN_NOTIFICATION__?: {
      sendNotification?: (opts: { title: string; body: string }) => void;
    };
  }).__TAURI_PLUGIN_NOTIFICATION__;
  if (plugin?.sendNotification) {
    plugin.sendNotification({ title: payload.title, body: payload.body });
  }
}
