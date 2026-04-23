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

## Phase 2 — Streamers and VODs

```sql
-- 0002_streamers_and_vods.sql (draft — lands in Phase 2)

CREATE TABLE streamers (
  twitch_user_id   TEXT    PRIMARY KEY,        -- Helix user.id
  login            TEXT    NOT NULL UNIQUE,    -- lowercase login
  display_name     TEXT    NOT NULL,
  profile_image_url TEXT,
  followed_at      INTEGER NOT NULL,
  deleted_at       INTEGER
);

CREATE INDEX idx_streamers_login ON streamers(login) WHERE deleted_at IS NULL;

CREATE TABLE vods (
  twitch_video_id   TEXT    PRIMARY KEY,
  twitch_user_id    TEXT    NOT NULL REFERENCES streamers(twitch_user_id) ON DELETE CASCADE,
  title             TEXT    NOT NULL,
  duration_seconds  INTEGER NOT NULL,
  stream_started_at INTEGER NOT NULL,      -- canonical chronological key
  published_at      INTEGER NOT NULL,
  game_name         TEXT,                  -- from chapters: first chapter, or null
  thumbnail_url     TEXT,
  state             TEXT    NOT NULL CHECK (state IN ('discovered','eligible','ignored','queued','downloading','downloaded','watched','failed')),
  discovered_at     INTEGER NOT NULL,
  last_seen_at      INTEGER NOT NULL,
  ignored_reason    TEXT                   -- 'not_gta', 'live', 'sub_only', ...
);

CREATE INDEX idx_vods_user ON vods(twitch_user_id);
CREATE INDEX idx_vods_chronological ON vods(stream_started_at DESC);
CREATE INDEX idx_vods_state ON vods(state);
```

### State machine for `vods.state`

```
         discovered
           │
           │ game whitelist match + stream ended
           ▼
         eligible ──► ignored (not_gta, sub_only, live, ...)
           │
           │ user or auto-queue
           ▼
         queued
           │
           ▼
      downloading ──► failed (after retry budget)
           │
           ▼
       downloaded
           │
           ▼
         watched
```

---

## Phase 3 — Downloads

```sql
-- 0003_downloads.sql (draft — lands in Phase 3)

CREATE TABLE download_tasks (
  id               INTEGER PRIMARY KEY AUTOINCREMENT,
  twitch_video_id  TEXT    NOT NULL UNIQUE REFERENCES vods(twitch_video_id) ON DELETE CASCADE,
  file_path        TEXT,
  quality_preset   TEXT    NOT NULL,
  requested_at     INTEGER NOT NULL,
  started_at       INTEGER,
  completed_at     INTEGER,
  failed_at        INTEGER,
  failure_reason   TEXT,
  bytes_total      INTEGER,
  bytes_downloaded INTEGER NOT NULL DEFAULT 0,
  attempt_count    INTEGER NOT NULL DEFAULT 0
);
```

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
