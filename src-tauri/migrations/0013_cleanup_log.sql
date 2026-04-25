-- 0013_cleanup_log.sql
-- Phase 7 — Auto-cleanup audit log.
-- Author-phase: Phase 7.
-- Rollback: forward-only. The table is pure audit data; dropping it
--           in a future migration removes history but leaves no
--           foreign keys dangling.
--
-- Records every cleanup invocation (scheduled, manual, or dry-run) so
-- the History view in Settings can show what was freed and so a
-- post-mortem investigation has the trail it needs.  See ADR-0024
-- §Execution.

CREATE TABLE IF NOT EXISTS cleanup_log (
    id                 INTEGER PRIMARY KEY AUTOINCREMENT,
    ran_at             INTEGER NOT NULL,
    -- 'scheduled' = tray-daemon tick
    -- 'manual'    = user clicked "Run cleanup now" in Settings
    -- 'dry_run'   = preview-only; no files were deleted
    mode               TEXT    NOT NULL CHECK (mode IN (
                           'scheduled', 'manual', 'dry_run'
                       )),
    freed_bytes        INTEGER NOT NULL DEFAULT 0,
    deleted_vod_count  INTEGER NOT NULL DEFAULT 0,
    -- 'ok'      = ran cleanly, freed >= 0 bytes
    -- 'partial' = some files failed to delete; the rest succeeded
    -- 'skipped' = scheduled tick determined disk pressure was below
    --             the high watermark (no work needed)
    -- 'error'   = the run aborted before any file was deleted
    status             TEXT    NOT NULL CHECK (status IN (
                           'ok', 'partial', 'skipped', 'error'
                       ))
);

CREATE INDEX IF NOT EXISTS idx_cleanup_log_ran_at
    ON cleanup_log(ran_at DESC);

UPDATE schema_meta
   SET value = 'See docs/data-model.md; migrations are append-only. Phase 7: cleanup log.'
 WHERE key = 'schema_notes';

PRAGMA user_version = 13;
