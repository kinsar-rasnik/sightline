-- 0009_completion_threshold.sql
-- Phase 6 — housekeeping for Phase 5 deferral #1.
-- Purpose: persist the watch-progress completion threshold so
--          `cmd_update_watch_progress` can derive `ProgressSettings`
--          from the live `AppSettings` row instead of falling back to
--          the compiled-in default of 0.9. Mirrors ADR-0018's state
--          machine boundary; the 70–100 % range is enforced via the
--          column-level CHECK so a malicious or buggy frontend can't
--          push a value that would skip the `in_progress → completed`
--          transition altogether.
-- Rollback: forward-only. SQLite ALTER ADD COLUMN is unreversible
--           without a table rewrite; a future migration would clone the
--           table to drop the column. Default of 0.9 mirrors the
--           pre-Phase-6 hardcoded value, so existing installs see
--           identical behaviour after running this migration.

ALTER TABLE app_settings
    ADD COLUMN completion_threshold REAL NOT NULL DEFAULT 0.9
        CHECK (completion_threshold >= 0.7 AND completion_threshold <= 1.0);

UPDATE schema_meta
   SET value = 'See docs/data-model.md; migrations are append-only. Phase 6: completion_threshold.'
 WHERE key = 'schema_notes';

PRAGMA user_version = 9;
