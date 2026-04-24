-- 0006_stream_intervals.sql
-- Phase 4 — timeline foundation.
-- Author-phase: Phase 4.
-- Rollback: forward-only. Migration 0007 can safely run on top.
--
-- Pre-computed overlap intervals for fast timeline rendering. Rebuilt
-- incrementally as new VODs are ingested; fully re-derivable from
-- `vods`, so a rebuild command exists for schema-upgrade cases.

CREATE TABLE IF NOT EXISTS stream_intervals (
  id          INTEGER PRIMARY KEY AUTOINCREMENT,
  vod_id      TEXT    NOT NULL UNIQUE REFERENCES vods(twitch_video_id) ON DELETE CASCADE,
  streamer_id TEXT    NOT NULL REFERENCES streamers(twitch_user_id) ON DELETE CASCADE,
  start_at    INTEGER NOT NULL,
  end_at      INTEGER NOT NULL,
  created_at  INTEGER NOT NULL,
  CHECK (end_at >= start_at)
);

CREATE INDEX IF NOT EXISTS idx_intervals_start    ON stream_intervals(start_at);
CREATE INDEX IF NOT EXISTS idx_intervals_range    ON stream_intervals(start_at, end_at);
CREATE INDEX IF NOT EXISTS idx_intervals_streamer ON stream_intervals(streamer_id, start_at);

PRAGMA user_version = 6;
