# The Sightline player

Once a VOD finishes downloading, a **Play** button appears on its
library card and in the detail drawer. Click it to open the player.

## Controls

Move your mouse over the player to bring up the controls overlay.
After three seconds of inactivity (while playback is active) the
overlay fades out so the video has the screen to itself.

- **Play / pause** — click the video or press `Space` / `k`.
- **Seek bar** — click anywhere on the bar to jump. The amber fill
  shows the currently-played position. Short ticks mark chapters:
  thin white for a regular chapter, amber for GTA V.
- **Elapsed / remaining** — click the time readout to toggle
  between showing total duration and "time left".
- **Volume** — drag the slider, click the speaker icon to mute, or
  press `m`. Scroll wheel over the video changes volume by ±5%.
- **Playback speed** — the `1×` button in the bottom-right opens a
  menu. Pick 0.5× up to 2×.
- **Chapter menu** — the three-line icon opens a list of chapter
  timestamps; click one to jump.
- **Picture-in-picture** — the `⧉` icon pops the video into a
  floating window so you can keep watching while using other apps.
- **Fullscreen** — the `⤢` icon or `f`. Double-clicking the video
  also toggles fullscreen.
- **More actions** — the `⋯` icon is where "Mark as watched" /
  "Mark as unwatched" lives.

## Keyboard shortcuts

| Key | Action |
|-----|--------|
| `Space` / `k` | Play / pause |
| `← / →` | Seek ±5 s |
| `Shift + ← / →` | Frame step (pauses if playing) |
| `J / L` | Seek ±10 s |
| `↑ / ↓` | Volume ±10% |
| `M` | Mute toggle |
| `F` | Fullscreen toggle |
| `P` | Picture-in-picture |
| `C` / `Shift + C` | Next / previous chapter |
| `0 – 9` | Seek to 0–90% of duration |
| `< / >` | Speed down / up one step |
| `, / .` | Frame step while paused |
| `Esc` | Exit fullscreen or overlay |

Every shortcut is customisable — see Settings → Advanced →
shortcuts.

## Resume behaviour

Sightline remembers where you paused a VOD and picks up there next
time. A small white tick on the seek bar shows the stored position.
Two conventions matter:

- **Pre-roll.** On resume, the player seeks a few seconds *back*
  from where you paused so you get a few seconds of context. The
  default is 5 s; Settings → Playback lets you pick anywhere from
  0 s to 30 s.
- **Restart near the end.** If you paused in the last 30 s of a
  VOD, reopening the player starts from the beginning — landing
  in the final seconds is almost never what you want.

If you want to explicitly reset: "Mark as unwatched" from the
player's `⋯` menu sets position to 0 and clears the watched state.

## When to use "Mark as watched"

Two reasons to reach for it:

- You watched the VOD outside Sightline (in a browser, on another
  device) and want Sightline to stop resurfacing it in the Continue
  Watching row.
- You started a VOD, realised it was a short talking-head segment
  you don't need to finish, and want Sightline to drop it from
  Continue Watching without actually watching the rest.

"Mark as watched" is distinct from "completed". Completed happens
automatically when you cross the threshold (default 90%); manually-
watched is a deliberate choice. Phase 7's auto-cleanup policies
treat the two differently (see ADR-0018).

## Missing file

Sometimes you delete or move a VOD file outside Sightline, and the
player can't find it on disk. You'll see a clear error state with a
**Re-download** button — clicking it enqueues a fresh download at
normal priority.

## "Can't play this file"

Some VODs produce files the browser's video decoder rejects (rare —
most Twitch VODs decode cleanly). When that happens the player
offers a **Remux file** action: Sightline runs ffmpeg to repackage
the file into a decoder-friendly container. The original is backed
up to a `.backup` file while the remux runs, and restored if
anything goes wrong.

If neither re-downloading nor remuxing works, the likely cause is
an unusual codec; file a bug with the VOD id and we'll look at
adding a format fallback.

## Picture-in-picture on window blur

Settings → Playback → **Enter picture-in-picture when the window
loses focus** toggles an experimental convenience: when you click
away from Sightline while a VOD is playing, the video pops into a
floating window that stays on top. Off by default — some OS
windowing stacks handle PiP better than others.

## What we don't do yet

- **Subtitles / captions** — Phase 7.
- **Per-VOD playback defaults** (this VOD always opens at 1.5×) —
  on the backlog; today playback speed is either global or session-
  scoped depending on Settings → Playback → Volume memory.
- **Network-streamed playback** — everything is local disk. The
  player can't open a Twitch URL directly; you download first, then
  play.
