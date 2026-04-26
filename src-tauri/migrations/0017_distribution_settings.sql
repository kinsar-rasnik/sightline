-- 0017_distribution_settings.sql
-- Phase 8 — Distribution mode + sliding-window settings.
-- Author-phase: Phase 8.
-- Rollback: forward-only.
--
-- Adds the user-configurable distribution-mode toggle, sliding-
-- window size, and pre-fetch enable flag.  See ADR-0030 §Migration
-- path: existing v1.0 installs (any non-empty downloads table) are
-- pinned to 'auto' to preserve their current behaviour; new
-- installs (empty downloads table) accept the column DEFAULT of
-- 'pull'.

ALTER TABLE app_settings
    ADD COLUMN distribution_mode TEXT NOT NULL DEFAULT 'pull'
        CHECK (distribution_mode IN ('auto','pull'));

ALTER TABLE app_settings
    ADD COLUMN sliding_window_size INTEGER NOT NULL DEFAULT 2
        CHECK (sliding_window_size >= 1
               AND sliding_window_size <= 20);

ALTER TABLE app_settings
    ADD COLUMN prefetch_enabled INTEGER NOT NULL DEFAULT 1
        CHECK (prefetch_enabled IN (0, 1));

-- Backwards-compat detection.  Any download row with a
-- non-recoverable state ('completed', 'queued', 'downloading')
-- pins the install on 'auto' mode so v1.0 users keep their
-- accustomed auto-download behaviour.  See R-RC-01 finding on
-- ADR-0030 — 'downloading' included to cover crash-recovery rows.
UPDATE app_settings
   SET distribution_mode = 'auto'
 WHERE EXISTS (
     SELECT 1 FROM downloads
      WHERE state IN ('completed','queued','downloading')
 )
   AND id = 1;

UPDATE schema_meta
   SET value = 'See docs/data-model.md; migrations are append-only. Phase 8: distribution mode.'
 WHERE key = 'schema_notes';

PRAGMA user_version = 17;
