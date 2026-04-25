# Sightline — Data Model

> **Status.** Phase 1 defines the header fields and the migrations pipeline. Tables marked *Phase N* land in that phase. The schema here is the target; `src-tauri/migrations/*.sql` is the ground truth.

## Storage

- **SQLite** via `sqlx` with the `runtime-tokio` + `sqlite` + `macros` + `migrate` features.
- One database file: `<library_root>/sightline.sqlite`.
- **WAL** journal mode enabled at startup. `synchronous = NORMAL`. `busy_timeout = 5000`.
- All timestamps are `INTEGER` Unix seconds **UTC**. No local-time columns.
- All IDs are stable across restarts. Streamer and VOD IDs come from Twitch; internal rows use `INTEGER PRIMARY KEY`.

## Invariants

1. **Append-only migrations.** Migration files are immutable after merge. To change a schema, add a new migration.
2. **Monotonic schema version.** `PRAGMA user_version` is bumped in the migration that last touched the schema.
3. **No orphans.** FK constraints are ON. Cascades are explicit in the schema.
4. **Time zone safety.** Any column representing a moment in wall-clock time ends with `_at` and holds UTC seconds. Durations use `_ms` or `_seconds` suffixes.
5. **Soft delete vs hard delete.** Rows a user might want back (followed streamers, watch progress) are soft-deleted with a `deleted_at` column. Derived caches are hard-deleted.

---

## Phase 1 — Meta and health

```sql
-- 0001_init.sql
CREATE TABLE schema_meta (
  key   TEXT PRIMARY KEY,
  value TEXT NOT NULL
);

INSERT INTO schema_meta (key, value) VALUES
  ('app_name',     'sightline'),
  ('created_at',   CAST(strftime('%s','now') AS TEXT)),
  ('schema_notes', 'See docs/data-model.md; migrations are append-only.');

PRAGMA user_version = 1;
```

The `schema_meta` table gives the `health` command a read target and lets future migrations assert invariants.

---

## Phase 2 — Streamers, VODs, chapters, poll log, settings, credentials meta

Phase 2 lands schema version **3** via migrations `0002_streamers_vods_chapters.sql` and `0003_poll_log.sql`. The shipping shape:

```sql
-- streamers: soft-deletable roster.
CREATE TABLE streamers (
    twitch_user_id     TEXT    PRIMARY KEY,         -- Helix user.id
    login              TEXT    NOT NULL,            -- stored lowercase (case-insensitive unique active below)
    display_name       TEXT    NOT NULL,
    profile_image_url  TEXT,
    broadcaster_type   TEXT    NOT NULL DEFAULT '', -- 'partner' | 'affiliate' | ''
    twitch_created_at  INTEGER NOT NULL,
    added_at           INTEGER NOT NULL,
    deleted_at         INTEGER,                     -- soft delete
    last_polled_at     INTEGER,
    next_poll_at       INTEGER,                     -- adaptive schedule target
    last_live_at       INTEGER                      -- drives adaptive interval
);
CREATE UNIQUE INDEX idx_streamers_login_active ON streamers(login) WHERE deleted_at IS NULL;
CREATE INDEX idx_streamers_next_poll ON streamers(next_poll_at) WHERE deleted_at IS NULL;

-- vods: the MASTER chronology, ordered by stream_started_at.
CREATE TABLE vods (
    twitch_video_id     TEXT    PRIMARY KEY,
    twitch_user_id      TEXT    NOT NULL REFERENCES streamers(twitch_user_id) ON DELETE CASCADE,
    stream_id           TEXT,
    title               TEXT    NOT NULL,
    description         TEXT    NOT NULL DEFAULT '',
    stream_started_at   INTEGER NOT NULL,           -- canonical chronological key (UTC seconds)
    published_at        INTEGER NOT NULL,
    url                 TEXT    NOT NULL,
    thumbnail_url       TEXT,
    duration_seconds    INTEGER NOT NULL,
    view_count          INTEGER NOT NULL DEFAULT 0,
    language            TEXT    NOT NULL DEFAULT '',
    muted_segments_json TEXT    NOT NULL DEFAULT '[]',  -- JSON array of { duration, offset }
    is_sub_only         INTEGER NOT NULL DEFAULT 0,
    helix_game_id       TEXT,                        -- fallback for single-game streams
    helix_game_name     TEXT,
    ingest_status       TEXT    NOT NULL DEFAULT 'pending' CHECK (ingest_status IN (
        'pending','chapters_fetched','eligible','skipped_game','skipped_sub_only','skipped_live','error'
    )),
    status_reason       TEXT    NOT NULL DEFAULT '',
    first_seen_at       INTEGER NOT NULL,
    last_seen_at        INTEGER NOT NULL
);

-- chapters: GAME_CHANGE moments from the public Twitch GQL endpoint (ADR-0008).
CREATE TABLE chapters (
    id               INTEGER PRIMARY KEY AUTOINCREMENT,
    twitch_video_id  TEXT    NOT NULL REFERENCES vods(twitch_video_id) ON DELETE CASCADE,
    position_ms      INTEGER NOT NULL CHECK (position_ms >= 0),
    duration_ms      INTEGER NOT NULL CHECK (duration_ms >= 0),
    game_id          TEXT,                                     -- null for unknown (review state)
    game_name        TEXT    NOT NULL DEFAULT '',
    chapter_type     TEXT    NOT NULL DEFAULT 'GAME_CHANGE' CHECK (chapter_type IN (
        'GAME_CHANGE','SYNTHETIC','OTHER'
    ))
);

-- app_settings: single-row. Holds the game filter, poll intervals, concurrency cap.
CREATE TABLE app_settings (
    id                    INTEGER PRIMARY KEY CHECK (id = 1),
    enabled_game_ids_json TEXT    NOT NULL DEFAULT '["32982"]',  -- GTA V
    poll_floor_seconds    INTEGER NOT NULL DEFAULT 600,
    poll_recent_seconds   INTEGER NOT NULL DEFAULT 1800,
    poll_ceiling_seconds  INTEGER NOT NULL DEFAULT 7200,
    concurrency_cap       INTEGER NOT NULL DEFAULT 4,
    first_backfill_limit  INTEGER NOT NULL DEFAULT 100,
    updated_at            INTEGER NOT NULL
);

-- credentials_meta: safe-to-persist summary. The actual Client ID + Secret
-- live in the OS keyring and never land in this file.
CREATE TABLE credentials_meta (
    id                      INTEGER PRIMARY KEY CHECK (id = 1),
    configured              INTEGER NOT NULL DEFAULT 0,
    client_id_masked        TEXT,
    last_token_acquired_at  INTEGER,
    updated_at              INTEGER NOT NULL
);

-- poll_log: append-only audit, one row per poll cycle per streamer.
CREATE TABLE poll_log (
    id                INTEGER PRIMARY KEY AUTOINCREMENT,
    twitch_user_id    TEXT    NOT NULL REFERENCES streamers(twitch_user_id) ON DELETE CASCADE,
    started_at        INTEGER NOT NULL,
    finished_at       INTEGER,
    vods_seen         INTEGER NOT NULL DEFAULT 0,
    vods_new          INTEGER NOT NULL DEFAULT 0,
    vods_updated      INTEGER NOT NULL DEFAULT 0,
    chapters_fetched  INTEGER NOT NULL DEFAULT 0,
    errors_json       TEXT    NOT NULL DEFAULT '[]',
    status            TEXT    NOT NULL CHECK (status IN (
        'running','ok','partial','error','rate_limited','skipped'
    ))
);
```

### State machine for `vods.ingest_status`

```
    pending
      │  Helix fetch + optional GQL chapters
      ▼
    chapters_fetched ───► error (persisted, retried next poll)
      │
      │ game filter / live gate / sub-only detection
      ▼
    eligible ──► skipped_game | skipped_sub_only | skipped_live
```

A VOD can oscillate between `skipped_live` and `eligible` as the streamer finishes the broadcast. A VOD marked `skipped_sub_only` is re-evaluated on every poll in case the streamer unlocks it. Game filter results (`skipped_game` / `eligible`) flip when the user changes `app_settings.enabled_game_ids_json`.

The Phase 3 downstream states (`queued → downloading → downloaded → watched`) live on a separate `download_tasks` table (see §Phase 3) rather than stacking more enum values onto `ingest_status` — the ingest pipeline and the download pipeline have different retry policies and different failure domains.

---

## Phase 3 — Downloads + library migration

Phase 3 lands schema version **5** via migrations `0004_downloads.sql`
and `0005_library_migrations.sql`. The shipping shape:

```sql
-- 0004_downloads.sql — persistent download queue.
CREATE TABLE downloads (
    vod_id           TEXT    PRIMARY KEY REFERENCES vods(twitch_video_id) ON DELETE CASCADE,
    state            TEXT    NOT NULL CHECK (state IN (
        'queued','downloading','paused','completed','failed_retryable','failed_permanent'
    )),
    priority         INTEGER NOT NULL DEFAULT 100,
    quality_preset   TEXT    NOT NULL CHECK (quality_preset IN ('source','1080p60','720p60','480p')),
    quality_resolved TEXT,
    staging_path     TEXT,
    final_path       TEXT,
    bytes_total      INTEGER,
    bytes_done       INTEGER NOT NULL DEFAULT 0,
    speed_bps        INTEGER,
    eta_seconds      INTEGER,
    attempts         INTEGER NOT NULL DEFAULT 0,
    last_error       TEXT,
    last_error_at    INTEGER,
    queued_at        INTEGER NOT NULL,
    started_at       INTEGER,
    finished_at      INTEGER,
    pause_requested  INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX idx_downloads_state    ON downloads(state);
CREATE INDEX idx_downloads_priority ON downloads(priority DESC, queued_at ASC);

-- app_settings gains the library + download knobs:
ALTER TABLE app_settings ADD COLUMN library_root             TEXT;
ALTER TABLE app_settings ADD COLUMN library_layout           TEXT    NOT NULL DEFAULT 'plex';
ALTER TABLE app_settings ADD COLUMN staging_path             TEXT;
ALTER TABLE app_settings ADD COLUMN max_concurrent_downloads INTEGER NOT NULL DEFAULT 2;
ALTER TABLE app_settings ADD COLUMN bandwidth_limit_bps      INTEGER;        -- NULL = unlimited
ALTER TABLE app_settings ADD COLUMN quality_preset           TEXT    NOT NULL DEFAULT 'source';
ALTER TABLE app_settings ADD COLUMN auto_update_yt_dlp       INTEGER NOT NULL DEFAULT 1;
```

```sql
-- 0005_library_migrations.sql — audit trail for layout switches.
CREATE TABLE library_migrations (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    started_at  INTEGER NOT NULL,
    finished_at INTEGER,
    from_layout TEXT    NOT NULL,
    to_layout   TEXT    NOT NULL,
    moved       INTEGER NOT NULL DEFAULT 0,
    errors      INTEGER NOT NULL DEFAULT 0,
    status      TEXT    NOT NULL CHECK (status IN ('running','completed','failed','cancelled'))
);
-- At most one running migration at a time.
CREATE UNIQUE INDEX idx_library_migrations_one_running
    ON library_migrations(status) WHERE status = 'running';
```

### State machine for `downloads.state`

```
  queued
    │  worker picks row → yt-dlp download into staging_path
    ▼
  downloading ──────────► paused  (cmd_pause_download sets pause_requested)
    │  │                    │
    │  │                    └──► downloading  (cmd_resume_download)
    │  ▼
    │ failed_retryable ─► queued (attempts < 5, exponential backoff)
    │                 └─► failed_permanent (attempts == 5 OR reason is DISK_FULL / SUB_ONLY)
    ▼
  completed (terminal; final_path set; NFO / thumbnail written for plex layout)
```

`cmd_cancel_download` transitions from any non-terminal state to
`failed_permanent` with reason `USER_CANCELLED`. On process start-up
any row still in `downloading` is reset to `queued` — the staging
file is treated as garbage and the download is retried from scratch
(yt-dlp's resume flag is not trusted across process restarts on
Proton Drive / network filesystems).

---

## Phase 5 — Watch progress

Phase 5 lands schema version **8** via migration
`0008_watch_progress.sql`. The shipping shape (see
[ADR-0018](adr/0018-watch-progress-model.md)):

```sql
CREATE TABLE watch_progress (
    vod_id                         TEXT    PRIMARY KEY
                                           REFERENCES vods(twitch_video_id) ON DELETE CASCADE,
    position_seconds               REAL    NOT NULL DEFAULT 0
                                           CHECK (position_seconds >= 0),
    duration_seconds               REAL    NOT NULL
                                           CHECK (duration_seconds >= 0),
    -- Generated STORED for "sort by % watched" without a function scan.
    watched_fraction               REAL    GENERATED ALWAYS AS (
        CASE WHEN duration_seconds > 0
             THEN position_seconds / duration_seconds
             ELSE 0
        END
    ) STORED,
    state                          TEXT    NOT NULL CHECK (state IN (
        'unwatched','in_progress','completed','manually_watched'
    )),
    first_watched_at               INTEGER,
    last_watched_at                INTEGER NOT NULL,
    last_session_duration_seconds  REAL    NOT NULL DEFAULT 0,
    total_watch_seconds            REAL    NOT NULL DEFAULT 0
);
CREATE INDEX idx_watch_progress_last_watched ON watch_progress(last_watched_at DESC);
CREATE INDEX idx_watch_progress_state        ON watch_progress(state);
```

### State machine

```
    unwatched
      │  first timeupdate > 0
      ▼
    in_progress ──► completed   (watched_fraction ≥ threshold; default 0.9)
                           │
                           └── mark-as-unwatched ──► unwatched

    any ──► manually_watched   (user clicked "Mark as watched"; position := duration)
                       │
                       └── mark-as-unwatched ──► unwatched
```

`completed` and `manually_watched` are sticky under timeupdate — the
state machine's `transition_on_update` never moves away from them,
only the explicit mark-as-unwatched command does. Phase 7 cleanup
treats the two terminals differently: see ADR-0018.

### Notes

- `watched_fraction` is `STORED`, not `VIRTUAL`, so a future
  "sort library by progress" index is cheap to add without a
  migration.
- `total_watch_seconds` tracks *unique* playback time. The player
  session manager holds an in-memory interval-merger
  (`domain::interval_merger::IntervalSet`) and only forwards the
  cumulative new-coverage delta, so scrubbing back doesn't
  double-count.
- Position writes round to 0.5 s resolution
  (`domain::watch_progress::round_to_half_second`). Cuts write
  amplification from a 4 Hz `timeupdate` to an effective ~2 writes
  per second at most, then further throttled to one DB write every
  5 s wall-clock by the service layer.

---

## Phase 6 — Completion threshold

Phase 6 lands schema version **9** via migration
`0009_completion_threshold.sql`. Closes the Phase-5 deferral that
left `cmd_update_watch_progress` hardcoding
`ProgressSettings::default()` instead of reading the user-configured
threshold.

```sql
ALTER TABLE app_settings
    ADD COLUMN completion_threshold REAL NOT NULL DEFAULT 0.9
        CHECK (completion_threshold >= 0.7 AND completion_threshold <= 1.0);
```

The column-level CHECK mirrors the documented 70–100 %
configurable range from ADR-0018. `cmd_update_watch_progress` calls
`SettingsService::get()` per tick and threads `completion_threshold`
into `ProgressSettings`; the domain-layer `clamp()` is
defence-in-depth.

The frontend Settings UI now writes the threshold via
`update_settings({ completionThreshold })` instead of localStorage,
and `PlayerPage` reads it from `useSettings()`. Single source of
truth: `app_settings.completion_threshold`.

---

## Phase 6 — Multi-View Sync sessions

Phase 6 lands schema version **11** via migrations
`0010_sync_sessions.sql` and `0011_sync_settings.sql`.  See
[ADR-0021](adr/0021-split-view-layout.md),
[ADR-0022](adr/0022-sync-math-and-drift.md), and
[ADR-0023](adr/0023-group-wide-transport.md) for the design.

```sql
-- 0010_sync_sessions.sql — multi-view session definitions.
CREATE TABLE sync_sessions (
    id                INTEGER PRIMARY KEY AUTOINCREMENT,
    created_at        INTEGER NOT NULL,
    closed_at         INTEGER,
    layout            TEXT    NOT NULL DEFAULT 'split-50-50' CHECK (layout IN ('split-50-50')),
    leader_pane_index INTEGER CHECK (leader_pane_index IS NULL OR leader_pane_index >= 0),
    status            TEXT    NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'closed'))
);
CREATE INDEX idx_sync_sessions_status_created
    ON sync_sessions(status, created_at DESC);

-- Per-pane membership; v1 caps `pane_index` at 0..=1.
CREATE TABLE sync_session_panes (
    session_id  INTEGER NOT NULL REFERENCES sync_sessions(id) ON DELETE CASCADE,
    pane_index  INTEGER NOT NULL CHECK (pane_index >= 0 AND pane_index <= 1),
    vod_id      TEXT    NOT NULL REFERENCES vods(twitch_video_id) ON DELETE CASCADE,
    volume      REAL    NOT NULL DEFAULT 1.0 CHECK (volume >= 0.0 AND volume <= 1.0),
    muted       INTEGER NOT NULL DEFAULT 0 CHECK (muted IN (0, 1)),
    joined_at   INTEGER NOT NULL,
    PRIMARY KEY (session_id, pane_index)
);
CREATE UNIQUE INDEX idx_sync_session_panes_one_vod_per_session
    ON sync_session_panes(session_id, vod_id);
```

```sql
-- 0011_sync_settings.sql — runtime knobs for the sync engine.
ALTER TABLE app_settings
    ADD COLUMN sync_drift_threshold_ms REAL NOT NULL DEFAULT 250.0
        CHECK (sync_drift_threshold_ms >= 50.0 AND sync_drift_threshold_ms <= 1000.0);
ALTER TABLE app_settings
    ADD COLUMN sync_default_layout TEXT NOT NULL DEFAULT 'split-50-50'
        CHECK (sync_default_layout IN ('split-50-50'));
ALTER TABLE app_settings
    ADD COLUMN sync_default_leader TEXT NOT NULL DEFAULT 'first-opened'
        CHECK (sync_default_leader IN ('first-opened'));
```

### Notes

- `leader_pane_index` is `NULL` only during the brief window between
  `INSERT INTO sync_sessions` and the follow-up `UPDATE` that sets the
  leader; the services layer wraps both writes in a single transaction
  so external callers never observe `NULL` mid-session.
- `sync_session_panes(session_id, vod_id)` has a unique partial-style
  index so the DB rejects "two panes pointing at the same VOD".  Live
  v2 PiP layouts that *do* want the same VOD twice would supersede
  this constraint via a new migration.
- Live frame-by-frame state (each pane's `currentTime`, drift history)
  stays in frontend memory.  The DB row describes the session
  *definition*, not its instantaneous playback state.
- `sync_default_leader = 'first-opened'` selects pane index 0 (the
  primary VOD the user clicked).  Modeled as TEXT so v2 can add
  `'longest'` etc. without a contract bump.

---

## Phase 7 — Auto-cleanup + update checker

Phase 7 lands schema version **14** via three migrations.  See
[ADR-0024](adr/0024-auto-cleanup-service.md) and
[ADR-0026](adr/0026-update-checker.md).

```sql
-- 0012_cleanup_settings.sql — cleanup-service knobs.
ALTER TABLE app_settings
    ADD COLUMN cleanup_enabled INTEGER NOT NULL DEFAULT 0
        CHECK (cleanup_enabled IN (0, 1));
ALTER TABLE app_settings
    ADD COLUMN cleanup_high_watermark REAL NOT NULL DEFAULT 0.9
        CHECK (cleanup_high_watermark >= 0.5
               AND cleanup_high_watermark <= 0.99);
ALTER TABLE app_settings
    ADD COLUMN cleanup_low_watermark REAL NOT NULL DEFAULT 0.75
        CHECK (cleanup_low_watermark >= 0.4
               AND cleanup_low_watermark <= 0.95);
ALTER TABLE app_settings
    ADD COLUMN cleanup_schedule_hour INTEGER NOT NULL DEFAULT 3
        CHECK (cleanup_schedule_hour >= 0
               AND cleanup_schedule_hour <= 23);
```

The service-layer `update` handler additionally rejects
`cleanup_low_watermark >= cleanup_high_watermark` so the History view
never displays an inverted-watermark configuration.

```sql
-- 0013_cleanup_log.sql — audit trail for every cleanup run.
CREATE TABLE cleanup_log (
    id                 INTEGER PRIMARY KEY AUTOINCREMENT,
    ran_at             INTEGER NOT NULL,
    mode               TEXT    NOT NULL CHECK (mode IN (
                           'scheduled', 'manual', 'dry_run'
                       )),
    freed_bytes        INTEGER NOT NULL DEFAULT 0,
    deleted_vod_count  INTEGER NOT NULL DEFAULT 0,
    status             TEXT    NOT NULL CHECK (status IN (
                           'ok', 'partial', 'skipped', 'error'
                       ))
);
CREATE INDEX idx_cleanup_log_ran_at ON cleanup_log(ran_at DESC);
```

```sql
-- 0014_update_settings.sql — update-checker knobs.
ALTER TABLE app_settings
    ADD COLUMN update_check_enabled INTEGER NOT NULL DEFAULT 0
        CHECK (update_check_enabled IN (0, 1));
ALTER TABLE app_settings
    ADD COLUMN update_check_last_run INTEGER;
ALTER TABLE app_settings
    ADD COLUMN update_check_skip_version TEXT;
```

### Notes

- A successful cleanup deletes the file from `<final_path>` and flips
  the `downloads` row to `failed_permanent` with
  `last_error = 'CLEANED_UP'`. The `watch_progress` row is **not**
  touched, so a re-download lands the user at their last position.
- `cleanup_log` is append-only; the Settings UI's History view
  consumes the most recent rows via
  `commands.getCleanupHistory({ limit })`.
- `update_check_last_run` is bumped on every check (success, no-op,
  or error) so the once-per-day gate is honoured even across daemon
  restarts.

---

## Referential integrity and cascading

- Deleting a streamer (hard delete path, rarely used) cascades to their VODs, download tasks, and watch progress.
- Soft-deleting a streamer sets `deleted_at` and hides them from the UI, but their VODs remain in the library so the user can still watch what was already downloaded.
- A VOD is never deleted from the library as a side effect of any operation except auto-cleanup, which has its own gated flow in Phase 7.

## Queries to watch for

- **Library grid.** `SELECT ... FROM vods v JOIN streamers s ON v.twitch_user_id = s.twitch_user_id WHERE v.state IN ('downloaded','watched') ORDER BY v.stream_started_at DESC LIMIT ? OFFSET ?;`
- **Poll discovery.** `INSERT INTO vods (...) ON CONFLICT(twitch_video_id) DO UPDATE SET last_seen_at = excluded.last_seen_at, title = excluded.title, ...;`
- **Chronological join for Sync View.** Range select on `stream_started_at` across two `twitch_user_id`s.

All of these are supported by the indexes declared above.

## Migration workflow

1. Create `src-tauri/migrations/NNNN_<slug>.sql`. Four-digit sequence. Never edit once merged.
2. Include a header comment: purpose, author-phase, rollback note (forward-only is acceptable — say so).
3. `sqlx migrate add` is not used here — we author files by hand to keep them reviewable.
4. The migration pipeline runs at app startup (before any other DB access).
5. If a migration fails, the app exits with a clear error; no partial schema is acceptable.
