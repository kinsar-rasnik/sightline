# ADR-0033 — Library UI re-conception (available + downloaded)

- **Status.** Accepted
- **Date.** 2026-04-26 (Phase 8)
- **Related.**
  [ADR-0030](0030-pull-distribution-model.md) (the state machine the
  Library UI now visualises) ·
  [ADR-0011](0011-library-layout-pluggability.md) (file-system layout
  that backs the underlying assets — unchanged).

## Context

In v1.0, the Library page showed *downloaded* VODs only.  An
"available but not downloaded" VOD didn't appear; the user had to go
to a separate Downloads page and enqueue it explicitly.

ADR-0030's pull model makes "available" a first-class state.  The
Library page must now answer two questions equally well:

1. "What of this streamer's VODs do I have on disk right now?"
2. "What's available to pick from?"

Showing only the first answer (v1.0) hides the system from the user.
Showing only the second (a "Twitch listing") loses the immediate
"what can I play?" affordance.  The new design unifies them.

## Decision

A single chronologically-sorted Library page that shows every
non-deleted VOD for the selected streamer (or all streamers, if no
filter is set), with visual differentiation by status.

### Visual states

| Status        | Treatment                                                |
|---------------|----------------------------------------------------------|
| `available`   | Card shown at 60 % opacity, no thumbnail flash.  Hover   |
|               | reveals a "Download" button.                             |
| `queued`      | 70 % opacity, blue "Queued" badge, in-flight icon.       |
| `downloading` | 80 % opacity, progress bar overlay on thumbnail.         |
| `ready`       | Full opacity, "Watch" CTA prominently visible.           |
| `archived`    | Full opacity, watched-checkmark indicator, "Re-watch"    |
|               | CTA.                                                     |
| `deleted`     | 50 % opacity, "Pick again" CTA.                          |

The "watching" state (player open) is conveyed by a subtle blue
border on the active card, leveraged from the existing player
context.

### Filter chips

A horizontal chip row above the grid:

- **All** (default) — every status.
- **Available** — `available` + `deleted` (un-picked content).
- **Downloaded** — `queued` + `downloading` + `ready`.
- **Watched** — `archived`.

The filter is persisted in URL state (`?filter=available`) so that
deep links and browser back/forward work as expected.

### Per-VOD quick actions

A hover-revealed action row on each card:

- **Available / Deleted:** "Download now" (calls `pickVod`).
- **Queued:** "Cancel" (calls `unpickVod`, returns to available).
- **Downloading:** "Cancel" (existing `cancelDownload`).
- **Ready:** "Watch" + "Remove" (`removeVod` flips to `deleted`).
- **Archived:** "Re-watch" + "Re-download" if file still on disk;
  "Pick again" if `deleted`.

### Empty states

Per-filter empty state copy:

- **All / no streamers:** "Add a streamer to get started" with a CTA
  to the Settings → Streamers section.
- **All / streamers exist, no VODs:** "Polling for VODs… check back
  in a few minutes."
- **Available / nothing available:** "All caught up.  When this
  streamer publishes a new VOD it'll appear here."
- **Downloaded / nothing downloaded:** "Pick a VOD from Available to
  download it."
- **Watched / nothing watched:** "Watched VODs appear here."

### Sort order

Default: `stream_started_at DESC` (most recent first), matching v1.0.
Sort options stay the same as v1.0; we don't remove any axes.

### Accessibility

- Status badges include a screen-reader-only text label
  (`<span class="sr-only">Available</span>`).
- Quick-action buttons have explicit `aria-label`s describing the
  target VOD ("Download Episode 47 — 2026-04-25").
- Keyboard tab order: card → primary CTA → secondary CTA → next
  card.

## Alternatives considered

### A. Two separate pages (Available, Library)

Considered, rejected.  Forcing the user to context-switch between
two pages to "see what's available and pick" reintroduces the
v1.0 awkwardness ADR-0030 was trying to remove.

### B. Tab-based filter at the page top instead of chips

Tabs imply mutual exclusivity at a stronger level than chips.  The
"all" view is the default and most-used; chips de-emphasise the
filtering compared to tabs.  Subjective design call.

### C. Show downloaded VODs only by default; a "Show available"
       toggle to expand

Rejected.  Defeats the goal of putting "available" on equal footing.

### D. Card-level status colours (red/yellow/green) instead of opacity

Considered.  Colour-blind users would rely on the badge text anyway,
and the opacity treatment is more universally readable.  We do use
status-themed badges for the inline label, but the primary discriminator
is opacity + secondary label, not colour.

### E. Drag-to-pick from a sidebar listing

Out of scope for v2.0.  The "Download now" button is sufficient.

## Consequences

**Positive.**
- The user understands the system state at a glance — no hidden
  "available somewhere else" content.
- The Library page becomes the single source of truth for "what can
  I do with this streamer's content".
- Power users still have the Downloads page for queue diagnostics;
  the Library page doesn't try to replace that.

**Costs accepted.**
- A user with 200+ VODs per streamer sees 200 cards mostly at 60 %
  opacity.  The filter chips are a quick mitigation, but the default
  "All" view is dense by design.  Pagination (Phase-3 already
  paginates) keeps render performance reasonable.
- Frontend testing surface roughly doubles — every quick action
  needs a unit test.

**Risks.**
- A user who toggles `distribution_mode = 'auto'` after upgrading
  sees mostly `ready` VODs and few `available` ones; the visual
  variety of the new design is muted in their case.  Acceptable —
  they're in legacy mode for a reason.
- The hover-reveal pattern is desktop-friendly; touch input on a
  Surface tablet won't get the action row at all.  Documented as
  a v2.1 follow-up (always-visible action row on touch).

## Follow-ups

- Touch-input handling: persistent action row when a touch device is
  detected.  v2.1.
- Bulk-pick UI: shift-click to select a range, "Download N selected"
  action.  v2.1 if user demand.
- "Schedule download" — let the user pick a VOD to start downloading
  at a specific time (e.g., overnight).  v2.2.
- Per-streamer "Always pull next 5" preset that pre-populates the
  sliding-window for a chosen streamer.  Considered post-v2.0.

## References

- `src/features/library/LibraryPage.tsx`
- `src/features/library/VodCard.tsx`
- `src/features/library/FilterChips.tsx` (new)
- ADR-0030 (state machine driving the visual states)
- ADR-0011 (file system layout, unchanged)
