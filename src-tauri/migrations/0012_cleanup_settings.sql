-- 0012_cleanup_settings.sql
-- Phase 7 — Auto-cleanup service settings.
-- Author-phase: Phase 7.
-- Rollback: forward-only. SQLite ALTER ADD COLUMN is irreversible
--           without a table rewrite.  Defaults below mirror ADR-0024,
--           so existing installs see the documented behaviour after
--           the migration runs.
--
-- Adds the user-configurable knobs for the auto-cleanup service to
-- `app_settings`.  See ADR-0024 §Watermarks for the bound rationale
-- and §Scheduler integration for `cleanup_schedule_hour`.

ALTER TABLE app_settings
    ADD COLUMN cleanup_enabled INTEGER NOT NULL DEFAULT 0
        CHECK (cleanup_enabled IN (0, 1));

-- Trigger threshold: when used / total >= cleanup_high_watermark,
-- the next scheduled tick (or a manual run) executes a plan.
-- Lower bound 0.5 prevents a misconfiguration that would fire
-- every tick on a half-empty disk; upper bound 0.99 keeps a
-- minimum useful free-space buffer.
ALTER TABLE app_settings
    ADD COLUMN cleanup_high_watermark REAL NOT NULL DEFAULT 0.9
        CHECK (cleanup_high_watermark >= 0.5
               AND cleanup_high_watermark <= 0.99);

-- Stop threshold: cleanup keeps deleting until used / total drops
-- to or below this fraction.  Bounds avoid an inverted
-- (low > high) configuration; the service layer additionally
-- enforces low < high at write time.
ALTER TABLE app_settings
    ADD COLUMN cleanup_low_watermark REAL NOT NULL DEFAULT 0.75
        CHECK (cleanup_low_watermark >= 0.4
               AND cleanup_low_watermark <= 0.95);

-- Local hour of day at which the scheduled tick fires (0..=23).
-- The tray daemon checks every 5 minutes whether the wall clock has
-- crossed this hour since the last scheduled run; if so, plan +
-- execute.
ALTER TABLE app_settings
    ADD COLUMN cleanup_schedule_hour INTEGER NOT NULL DEFAULT 3
        CHECK (cleanup_schedule_hour >= 0
               AND cleanup_schedule_hour <= 23);

UPDATE schema_meta
   SET value = 'See docs/data-model.md; migrations are append-only. Phase 7: cleanup settings.'
 WHERE key = 'schema_notes';

PRAGMA user_version = 12;
