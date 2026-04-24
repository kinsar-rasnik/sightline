-- 0003_poll_log.sql
-- Phase: 2 (Twitch ingest).
-- Purpose: append-only audit of every poll cycle for a streamer. Drives the
--          Streamers page's "last poll" badge and the diagnostic panel.
-- Rollback: forward-only.

CREATE TABLE IF NOT EXISTS poll_log (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    twitch_user_id  TEXT    NOT NULL REFERENCES streamers(twitch_user_id) ON DELETE CASCADE,
    started_at      INTEGER NOT NULL,
    finished_at     INTEGER,                                  -- NULL while in progress
    vods_seen       INTEGER NOT NULL DEFAULT 0,
    vods_new        INTEGER NOT NULL DEFAULT 0,
    vods_updated    INTEGER NOT NULL DEFAULT 0,
    chapters_fetched INTEGER NOT NULL DEFAULT 0,
    errors_json     TEXT    NOT NULL DEFAULT '[]',            -- JSON array of { stage, detail }
    status          TEXT    NOT NULL                          -- see allowed states below
                    CHECK (status IN ('running', 'ok', 'partial', 'error', 'rate_limited', 'skipped'))
);

CREATE INDEX IF NOT EXISTS idx_poll_log_user_time
    ON poll_log(twitch_user_id, started_at DESC);

PRAGMA user_version = 3;
