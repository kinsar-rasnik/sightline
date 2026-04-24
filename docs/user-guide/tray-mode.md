# Tray mode

Sightline keeps running in the background after you close the window, so
polling and downloads aren't interrupted when the window is out of sight.
A menu-bar (macOS) or system-tray (Windows / Linux) icon is the ambient
surface into the app's state.

## The default behaviour

- **Click the window close button.** The window hides. The app keeps
  running — poller stays on schedule, downloads continue, the tray icon
  stays visible.
- **First time you close.** A one-time toast explains the new behaviour
  and gives you a "Actually quit on close" link so you can switch to the
  traditional quit-on-close mode in one click.
- **Reopen the window.** Click the tray icon, or pick "Open Sightline"
  from the tray menu.

## Tray menu

| Item | Action |
|------|--------|
| *Summary row* | Current state at a glance: active downloads, queued, bandwidth, next poll ETA. Not clickable. |
| Open Sightline | Show the window. |
| Open Downloads | Show the window and jump to the Downloads page. |
| Open Timeline | Show the window and jump to the Timeline page. |
| Pause all downloads | Snapshot every active/queued row into `paused`. Resume later with… |
| Resume all downloads | …the paired command. |
| Settings | Show the window and jump to Settings → Advanced. |
| Quit | Stop the poller, drain the download queue (yt-dlp is terminated; progress is flushed to the DB), then exit. |

## Changing the default

Settings → **Advanced** → "When the window close button is clicked":

- **Hide to tray (keep polling)** — the default described above.
- **Quit** — window close quits the app like a traditional desktop
  program.

## Starting at login (optional)

Settings → Advanced → "Start Sightline at login" registers Sightline as an
auto-start item. The registration is user-scoped (no admin privileges
required) and can be toggled off at any time. On first run the OS may
prompt to confirm; say yes once and you'll never see the prompt again.

## macOS dock icon

By default, Sightline hides its dock icon in tray mode — the menu-bar
icon is the canonical surface. If you prefer the familiar dock icon,
Settings → Advanced → **Show dock icon on macOS** flips it back.

## What survives a crash

The poller and download queue both persist state incrementally to SQLite
(WAL journaling). If Sightline quits unexpectedly — OS restart, crash,
force-quit — the next launch:

1. Replays the migration check (should be a no-op).
2. Picks up any `downloading` row that didn't get a chance to finish,
   resets it to `queued`, and re-runs the download from scratch (yt-dlp's
   resume flag is unreliable across our target filesystems, see
   [ADR-0012](../adr/0012-staging-atomic-move.md)).
3. Resumes the regular poll cycle on the schedule you'd configured.

You never lose ingested data — every streamer, VOD, chapter, and download
history remains on disk.
