-- 0004_downloads.sql
-- Phase: 3 (Download engine).
-- Purpose: the persistent download queue. One row per VOD enqueued for
--          download. State machine lives in domain/download_state.rs;
--          this table is append-or-update, never purged directly (a
--          completed row can be removed via cmd_cancel_download).
-- Also extends app_settings with the download / storage / library
-- columns so the frontend Settings page can round-trip them.
-- Rollback: forward-only. Any fix ships as a new migration.

CREATE TABLE IF NOT EXISTS downloads (
    vod_id           TEXT    PRIMARY KEY
                             REFERENCES vods(twitch_video_id) ON DELETE CASCADE,
    state            TEXT    NOT NULL                    -- see state machine
                             CHECK (state IN (
                                 'queued',
                                 'downloading',
                                 'paused',
                                 'completed',
                                 'failed_retryable',
                                 'failed_permanent'
                             )),
    priority         INTEGER NOT NULL DEFAULT 100,       -- higher = runs first
    quality_preset   TEXT    NOT NULL                    -- user selection at enqueue time
                             CHECK (quality_preset IN ('source', '1080p60', '720p60', '480p')),
    quality_resolved TEXT,                                -- actual format after negotiation; NULL until first yt-dlp pass
    staging_path     TEXT,                                -- absolute path during download
    final_path       TEXT,                                -- absolute path after atomic move (completed only)
    bytes_total      INTEGER,                             -- from yt-dlp filesize / filesize_approx
    bytes_done       INTEGER NOT NULL DEFAULT 0,
    speed_bps        INTEGER,                             -- last-reported instantaneous speed
    eta_seconds      INTEGER,                             -- last-reported ETA
    attempts         INTEGER NOT NULL DEFAULT 0,          -- cumulative failure count
    last_error       TEXT,                                -- short classification string
    last_error_at    INTEGER,                             -- unix seconds UTC
    queued_at        INTEGER NOT NULL,                    -- unix seconds UTC
    started_at       INTEGER,                             -- unix seconds UTC
    finished_at      INTEGER,                             -- unix seconds UTC
    pause_requested  INTEGER NOT NULL DEFAULT 0           -- 0/1 — set by cmd_pause_download; cleared on transition
);

CREATE INDEX IF NOT EXISTS idx_downloads_state    ON downloads(state);
CREATE INDEX IF NOT EXISTS idx_downloads_priority ON downloads(priority DESC, queued_at ASC);

-- Extend app_settings with the Phase 3 knobs. ALTER TABLE ADD COLUMN is
-- safe in SQLite as long as we provide a default. Semantics:
--   library_root            — absolute path, set when the user picks a folder
--   library_layout          — 'plex' | 'flat'
--   staging_path            — nullable override; NULL means "use the OS default"
--   max_concurrent_downloads — bounded 1..5 in the service; stored raw here
--   bandwidth_limit_bps     — NULL means unlimited; otherwise bytes/second
--   quality_preset          — default for new enqueues
--   auto_update_yt_dlp      — 0/1
ALTER TABLE app_settings ADD COLUMN library_root            TEXT;
ALTER TABLE app_settings ADD COLUMN library_layout          TEXT    NOT NULL DEFAULT 'plex';
ALTER TABLE app_settings ADD COLUMN staging_path            TEXT;
ALTER TABLE app_settings ADD COLUMN max_concurrent_downloads INTEGER NOT NULL DEFAULT 2;
ALTER TABLE app_settings ADD COLUMN bandwidth_limit_bps     INTEGER;
ALTER TABLE app_settings ADD COLUMN quality_preset          TEXT    NOT NULL DEFAULT 'source';
ALTER TABLE app_settings ADD COLUMN auto_update_yt_dlp      INTEGER NOT NULL DEFAULT 1;

UPDATE schema_meta
   SET value = 'See docs/data-model.md; migrations are append-only. Phase 3: downloads queue, library layout + staging settings.'
 WHERE key = 'schema_notes';

PRAGMA user_version = 4;
