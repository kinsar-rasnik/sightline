# Watch progress

Sightline remembers where you paused a VOD, which VODs you've
completed, and how many hours you've watched — all on your machine,
no server involved.

## The four states

Every VOD Sightline knows about is in one of four states:

| State | What it means |
|-------|---------------|
| **Unwatched** | You've never opened the VOD in the player. |
| **In progress** | You've played some of it but not reached the completion threshold. |
| **Completed** | You watched beyond the completion threshold (default 90%). The VOD is automatically marked done. |
| **Manually watched** | You explicitly clicked "Mark as watched" from the player's `⋯` menu. |

"Completed" and "Manually watched" are distinct because they mean
different things: completed says you actually watched 90%+;
manually-watched is a user choice that doesn't imply anything about
whether the video played. Phase 7's auto-cleanup policies will
treat them differently — see ADR-0018.

## Continue Watching

The library's **Continue Watching** row is a horizontal strip
above the grid that surfaces up to 12 VODs in the "In progress"
state, sorted by most recently watched. Clicking a card resumes
playback with a 5-second pre-roll so you pick up with a beat of
context rather than dropped straight into the middle of a line of
dialogue.

Completing or manually-marking a VOD drops it from the row
immediately.

## Customising the completion threshold

Settings → **Playback** → **Completion threshold** — slide anywhere
from 70% to 100%.

- Lower values (70%) are useful if you often skip the end credits
  and want those VODs to auto-complete.
- Higher values (100%) mean Sightline only treats a VOD as
  completed if you played it all the way through.

The threshold applies to both new playback and re-opened VODs;
scrubbing past the threshold is enough to trigger the transition,
even if you don't sit through the intervening minutes.

## Pre-roll on resume

Settings → **Playback** → **Pre-roll seconds on resume** — anywhere
from 0 to 30 seconds. When you reopen a VOD, the player seeks
`pre-roll` seconds before your stored position so you ease back
into the watch. 5 s is the default and matches the convention
Netflix / Plex use.

If your stored position is within the last 30 seconds of the VOD,
Sightline starts from the beginning instead of dropping you into
the closing credits. That threshold isn't configurable today; file
a bug if it's a problem for you.

## Total watched seconds

Under the hood, Sightline tracks the cumulative *unique* playback
time per VOD in a field called `total_watch_seconds`. Scrubbing
back over territory you've already watched doesn't double-count —
the backend's interval merger collapses overlapping observations
into a single coverage range.

This powers the "Hours watched this streamer" stat in Phase 7's
per-streamer summary view. Today the value is persisted but not
surfaced — a future Settings page will aggregate it.

## How often does Sightline save?

The player writes to the SQLite database:

- Every 5 seconds while you're playing.
- Immediately on pause.
- Immediately when you click another app (the window loses focus).
- Immediately when you close the player.

If Sightline is force-quit mid-playback, the next launch picks up
from the last write — worst case you lose a few seconds of progress.
Position writes round to 0.5-second precision to keep the DB small
even during long watch sessions.

## Per-streamer stats

The backend ships a `cmd_get_watch_stats({ streamerId? })` command
that returns:

- `totalWatchSeconds` — sum of all `total_watch_seconds` rows
- `completedCount` — number of VODs in `completed` or
  `manually_watched`
- `inProgressCount` — number of VODs in `in_progress`

A per-streamer summary UI on top of this ships with Phase 7 polish.

## Clearing watch progress

"Mark as unwatched" from the player's `⋯` menu resets position to 0
and state to `Unwatched`. There's no bulk "forget everything"
button today — if you need one, delete the `watch_progress` table
row manually (it's just a SQLite file) and the next app launch
will treat those VODs as unwatched. Do this on a copy of the DB
first — the `vod_id` foreign keys will cascade so deletions are
safe.

## What we don't do

- **Cross-device sync.** Watch progress is per-install. If you
  run Sightline on a desktop and a laptop, each remembers
  independently.
- **Partial-completion grading.** We don't distinguish "watched
  69%" from "watched 71%" — either you're under the threshold
  (in progress) or over (completed).
- **Skip-intro / skip-outro.** Out of scope; a future feature
  could lean on the chapter timestamps to offer these.
