-- 0002_streamers_vods_chapters.sql
-- Phase: 2 (Twitch ingest, metadata, chapters).
-- Purpose: land the first real content tables — streamers (soft-deletable),
--          vods (the MASTER chronology), chapters (per-VOD game segments),
--          app_settings (single-row JSON blob), credentials_meta (a tiny
--          status row the frontend reads; the actual Client ID + Secret live
--          in the OS keyring and never in this file).
-- Rollback: forward-only. Any fix ships as a new migration.

CREATE TABLE IF NOT EXISTS streamers (
    twitch_user_id      TEXT    PRIMARY KEY,
    login               TEXT    NOT NULL,                      -- stored lowercase for case-insensitive lookup
    display_name        TEXT    NOT NULL,
    profile_image_url   TEXT,
    broadcaster_type    TEXT    NOT NULL DEFAULT '',           -- 'partner' | 'affiliate' | ''
    twitch_created_at   INTEGER NOT NULL,                      -- unix seconds UTC
    added_at            INTEGER NOT NULL,                      -- unix seconds UTC, when the user added them
    deleted_at          INTEGER,                               -- soft delete tombstone
    last_polled_at      INTEGER,                               -- unix seconds UTC
    next_poll_at        INTEGER,                               -- unix seconds UTC, adaptive schedule
    last_live_at        INTEGER                                -- unix seconds UTC, drives adaptive interval
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_streamers_login_active
    ON streamers(login) WHERE deleted_at IS NULL;
CREATE INDEX IF NOT EXISTS idx_streamers_next_poll
    ON streamers(next_poll_at) WHERE deleted_at IS NULL;

CREATE TABLE IF NOT EXISTS vods (
    twitch_video_id     TEXT    PRIMARY KEY,
    twitch_user_id      TEXT    NOT NULL REFERENCES streamers(twitch_user_id) ON DELETE CASCADE,
    stream_id           TEXT,                                  -- nullable; populated when we can match
    title               TEXT    NOT NULL,
    description         TEXT    NOT NULL DEFAULT '',
    stream_started_at   INTEGER NOT NULL,                      -- canonical chronological key (unix seconds UTC)
    published_at        INTEGER NOT NULL,
    url                 TEXT    NOT NULL,
    thumbnail_url       TEXT,
    duration_seconds    INTEGER NOT NULL,
    view_count          INTEGER NOT NULL DEFAULT 0,
    language            TEXT    NOT NULL DEFAULT '',
    muted_segments_json TEXT    NOT NULL DEFAULT '[]',         -- JSON array of { duration, offset } objects
    is_sub_only         INTEGER NOT NULL DEFAULT 0,            -- 0/1
    helix_game_id       TEXT,                                  -- top-level Helix game, fallback for single-game streams
    helix_game_name     TEXT,
    ingest_status       TEXT    NOT NULL DEFAULT 'pending'     -- see state machine below
                        CHECK (ingest_status IN (
                            'pending',
                            'chapters_fetched',
                            'eligible',
                            'skipped_game',
                            'skipped_sub_only',
                            'skipped_live',
                            'error'
                        )),
    status_reason       TEXT    NOT NULL DEFAULT '',           -- human-readable (e.g. 'vod type != archive')
    first_seen_at       INTEGER NOT NULL,                      -- unix seconds UTC, when ingest first discovered it
    last_seen_at        INTEGER NOT NULL                       -- unix seconds UTC, refreshed on every poll
);

CREATE INDEX IF NOT EXISTS idx_vods_user ON vods(twitch_user_id);
CREATE INDEX IF NOT EXISTS idx_vods_chronological ON vods(stream_started_at DESC);
CREATE INDEX IF NOT EXISTS idx_vods_status ON vods(ingest_status);
CREATE INDEX IF NOT EXISTS idx_vods_user_chronological ON vods(twitch_user_id, stream_started_at DESC);

CREATE TABLE IF NOT EXISTS chapters (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    twitch_video_id TEXT    NOT NULL REFERENCES vods(twitch_video_id) ON DELETE CASCADE,
    position_ms     INTEGER NOT NULL CHECK (position_ms >= 0),
    duration_ms     INTEGER NOT NULL CHECK (duration_ms >= 0),
    game_id         TEXT,                                       -- nullable for 'unknown game' fallback
    game_name       TEXT    NOT NULL DEFAULT '',
    chapter_type    TEXT    NOT NULL DEFAULT 'GAME_CHANGE'      -- GAME_CHANGE | SYNTHETIC | OTHER
                    CHECK (chapter_type IN ('GAME_CHANGE', 'SYNTHETIC', 'OTHER'))
);

CREATE INDEX IF NOT EXISTS idx_chapters_vod ON chapters(twitch_video_id, position_ms);
CREATE INDEX IF NOT EXISTS idx_chapters_game ON chapters(game_id) WHERE game_id IS NOT NULL;

-- Single-row settings table. Using INSERT OR REPLACE semantics keyed on id=1.
CREATE TABLE IF NOT EXISTS app_settings (
    id                    INTEGER PRIMARY KEY CHECK (id = 1),
    enabled_game_ids_json TEXT    NOT NULL DEFAULT '["32982"]',  -- GTA V
    poll_floor_seconds    INTEGER NOT NULL DEFAULT 600,           -- 10 min (live)
    poll_recent_seconds   INTEGER NOT NULL DEFAULT 1800,          -- 30 min
    poll_ceiling_seconds  INTEGER NOT NULL DEFAULT 7200,          -- 2 h (dormant)
    concurrency_cap       INTEGER NOT NULL DEFAULT 4,
    first_backfill_limit  INTEGER NOT NULL DEFAULT 100,
    updated_at            INTEGER NOT NULL
);

INSERT OR IGNORE INTO app_settings (id, updated_at)
    VALUES (1, CAST(strftime('%s','now') AS INTEGER));

-- Credentials metadata. Safe-to-persist summary the frontend can render:
-- "configured", a masked Client ID hint, and the last-token-acquired timestamp.
-- The actual Client ID + Secret are stored in the OS keyring and never on disk.
CREATE TABLE IF NOT EXISTS credentials_meta (
    id                        INTEGER PRIMARY KEY CHECK (id = 1),
    configured                INTEGER NOT NULL DEFAULT 0,      -- 0/1
    client_id_masked          TEXT,
    last_token_acquired_at    INTEGER,
    updated_at                INTEGER NOT NULL
);

INSERT OR IGNORE INTO credentials_meta (id, updated_at)
    VALUES (1, CAST(strftime('%s','now') AS INTEGER));

-- Touch meta so the schema_notes reflects Phase 2 content.
UPDATE schema_meta SET value = 'See docs/data-model.md; migrations are append-only. Phase 2: streamers, vods, chapters, app_settings, credentials_meta.' WHERE key = 'schema_notes';

PRAGMA user_version = 2;
