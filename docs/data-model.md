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

```sql
-- 0005_watch_progress.sql (draft — lands in Phase 5)

CREATE TABLE watch_progress (
  twitch_video_id TEXT    PRIMARY KEY REFERENCES vods(twitch_video_id) ON DELETE CASCADE,
  position_ms     INTEGER NOT NULL DEFAULT 0,
  duration_ms     INTEGER NOT NULL,
  last_watched_at INTEGER NOT NULL,
  marked_watched  INTEGER NOT NULL DEFAULT 0       -- 0/1
);
```

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
