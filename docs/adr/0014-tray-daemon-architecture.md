# ADR-0014 — Tray/daemon architecture and close-to-tray default

- **Status.** Accepted
- **Date.** 2026-04-24 (Phase 4)
- **Related.** [ADR-0005](0005-background-polling-architecture.md) (Tokio polling
  model, which this ADR relies on) · [ADR-0013](0013-sidecar-bundling.md) (the
  sidecars that keep running headlessly).

## Context

Phases 1–3 built the working Sightline but treated the window as the app: when
the user closed the window, the poller and download queue came down with it.
That's acceptable for the first three phases because no long-running work
actually relied on the process surviving — the fastest poll cycle finishes in
seconds; a download might be mid-flight, but the user explicitly clicked
close.

Phase 4 makes Sightline useful *headlessly*. The user should be able to add a
streamer, queue a download, close the window, and come back in an hour to find
everything in order. This is the same behaviour users expect from Steam,
Discord, qBittorrent, Tailscale — a menu-bar/tray icon that keeps the app
running without a visible window.

## Decision

### Runtime lifecycle

Three distinct lifecycle states instead of the Phase-3 two:

| State | Tokio tasks | Window | Process |
|-------|-------------|--------|---------|
| **Running (visible)** | alive | shown | alive |
| **Running (headless)** | alive | hidden | alive |
| **Quitting** | draining | closing | exiting |

Transitions:

- **Visible → Headless** — window close button. Default for new installs
  (existing users get a one-time toast on first close).
- **Headless → Visible** — user clicks the tray icon, or tray menu "Open
  Sightline", or OS re-activation (macOS `Reopen` event).
- **Any → Quitting** — tray menu "Quit", `Cmd/Ctrl+Q`, or
  `cmd_request_shutdown` from the webview. Broadcasts
  `app:shutdown_requested` with a 10 s deadline; poller stops next cycle;
  download queue signals each worker to pause + flush `bytes_done` /
  `last_error` to DB; then `std::process::exit(0)`.

The close-button behaviour is persisted as `app_settings.window_close_behavior`
(`hide` | `quit`). Users who prefer traditional quit-on-close can flip it
from Settings → Advanced.

### Why "hide by default"

The alternative is "quit by default" with an opt-in tray mode. We rejected it:
once a user understands the app has downloads + polling, headless is the
obviously-correct mode. The one-time toast on first close explains this and
offers "Actually quit on close" for users who prefer the old behaviour — so
the default can be right without surprising anyone.

### Tray surface

Platform-native tray icons via Tauri's built-in tray APIs. Menu items:

1. **Summary row** (disabled) — rendered dynamically from `cmd_get_app_summary`
   (active/queued downloads, next poll ETA, bandwidth). The row itself isn't
   clickable; it just communicates state at a glance.
2. **Open Sightline** — emits `app:tray_action { kind: "open_sightline" }` and
   shows the window.
3. **Open Downloads** / **Open Timeline** — same, routed to webview so the
   frontend's `useNavStore` switches page.
4. **Pause all downloads** → `cmd_pause_all_downloads`. Disabled when no
   active downloads.
5. **Resume all downloads** → `cmd_resume_all_downloads`. Disabled when no
   paused rows.
6. **Settings** → open window + route to Settings.
7. **Quit** → `cmd_request_shutdown` → `std::process::exit` after drain.

The menu is built by `lib.rs::run` when Tauri initializes the tray. The native
click handlers call the Tauri commands directly; the commands handle the
persistence and emit webview events so the UI can reflect state changes.

Tray icon assets: a 16×16 monochrome template image per platform (macOS expects
template images, Linux/Windows accept coloured). Icon bundling is a Phase 7
polish item — Phase 4 ships a workable placeholder.

### Shutdown coordination

`cmd_request_shutdown` emits `app:shutdown_requested { deadline_at }` and then
triggers each long-running service's existing `shutdown()` handle:

- `PollerService::spawn` already owns a `broadcast::Sender<()>` shutdown
  channel. The spawn's `Handle::shutdown` drops the sender, each poll task
  sees the closed channel on its next `tokio::select!` and returns cleanly.
- `DownloadQueueService::spawn` owns an `mpsc::Sender<Command>` and a
  `Semaphore` for worker slots. Shutdown is the same pattern: the
  command-channel close causes the manager loop to exit; active workers are
  signalled to save progress and let `kill_on_drop(true)` on the yt-dlp
  child take care of the subprocess.

Both services flush their last DB write before returning. The lib.rs wrapper
waits up to 10 s in `on_window_event` for the drain to complete before
exiting; beyond that, we force-exit — a user pressing Quit a second time
shouldn't be stuck waiting forever for a hung sidecar.

### Notifications (gated by this ADR because they ride the same event bus)

Native OS notifications are dispatched by a Rust `NotificationService`. The
service coalesces events per category within a 30-second window so a burst
of ingests (e.g. 20 new VODs from favorites) collapses to one "20 new VODs"
banner. Emission is gated by per-category settings plus a master toggle;
each emit writes to both a generic `notification:show` topic (the
`NotificationsToaster` component subscribes for in-app toasts) and a
category-specific topic (for callers that care about one category only).

The actual OS banner is fired via the webview shim that calls
`@tauri-apps/plugin-notification`'s `sendNotification`. This keeps the
plugin dependency frontend-only and avoids pulling the Rust plugin crate
into our non-bundled dev loop.

### Autostart (optional, per-user)

A `start_at_login` toggle in Settings. Rust-side wiring defers to Phase 7's
full release-polish work — registering LaunchAgent/Registry/XDG entries
requires per-OS signing that we don't yet set up. Phase 4 persists the
preference; Phase 7 wires the actual autostart registration.

## Alternatives considered

### A. Runtime process = webview-only (status quo pre-Phase-4)

Rejected. The whole point of Sightline is to aggregate VODs while you're
away from the window; quitting when the user closes defeats that.

### B. Background process spawned independently of the window

Rejected. Two processes means two log streams, two crash modes, two copies
of the DB connection pool. The Tokio-in-the-Tauri-process model is
simpler and already what we have.

### C. Keep the window hidden but also hide the dock icon / taskbar button

Rejected as the *default*. On macOS we can set `LSUIElement = 1` (which hides
the dock icon entirely), but that's a surprising posture for a new user
to opt into blindly. `show_dock_icon` is a per-user preference — off by
default on macOS so the tray icon is the canonical surface, but users can
flip it on if they prefer the window to be always dock-reachable.

## Consequences

**Positive.**
- Users can trust that closing the window doesn't cancel their downloads.
- The tray gives a glanceable status without opening the window.
- Shutdown is graceful — no partial DB state on the next boot.
- Future "Start at login" is a setting flip, not a new architecture.

**Costs accepted.**
- One more runtime state means one more thing to reason about in tests.
  The integration test suite now exercises:
    1. Close-to-hide keeps services alive + DB writes landing.
    2. Quit drains cleanly — a yt-dlp child is terminated, the DB row is
       reset to `queued`, the next boot picks up where we left off.
- Tray menu is platform-specific code; we test it behind a feature-flag
  on macOS (the development host) and rely on CI smoke tests on Linux +
  Windows.

## Follow-ups

1. Wire the actual tray icon assets (Phase 7 release polish — the icon set
   lives under `design/` and is generated by `cargo tauri icon`).
2. Autostart registration via the Tauri autostart plugin (Phase 7).
3. Phase 6 (multi-view sync) inherits the tray surface — no rewiring; the
   sync controller just adds a "Sync paused" entry when it's running.
