-- 0011_sync_settings.sql
-- Phase 6 — Multi-View Sync Engine settings.
-- Author-phase: Phase 6.
-- Rollback: forward-only. SQLite ALTER ADD COLUMN is irreversible
--           without a table rewrite.  Defaults below mirror the
--           production values from ADR-0022, so existing installs see
--           the documented behaviour after the migration.
--
-- Adds the user-configurable knobs for the multi-view sync engine to
-- `app_settings`.  Kept separate from migration 0010 so the two
-- concerns (schema for the session row vs. configuration for the
-- engine that operates on it) can land + revert independently.
--
-- See ADR-0022 §Drift tolerance for the threshold range justification
-- and ADR-0023 §UI affordances for the leader-default semantics.

ALTER TABLE app_settings
    ADD COLUMN sync_drift_threshold_ms REAL NOT NULL DEFAULT 250.0
        CHECK (sync_drift_threshold_ms >= 50.0
               AND sync_drift_threshold_ms <= 1000.0);

ALTER TABLE app_settings
    ADD COLUMN sync_default_layout TEXT NOT NULL DEFAULT 'split-50-50'
        CHECK (sync_default_layout IN ('split-50-50'));

-- 'first-opened' = use the pane index 0 (the primary VOD the user
-- clicked from the detail drawer) as leader.  v2 may add 'longest'
-- (auto-elect the longest-duration pane) — kept as a TEXT enum so the
-- vocabulary can grow without a column change.
ALTER TABLE app_settings
    ADD COLUMN sync_default_leader TEXT NOT NULL DEFAULT 'first-opened'
        CHECK (sync_default_leader IN ('first-opened'));

UPDATE schema_meta
   SET value = 'See docs/data-model.md; migrations are append-only. Phase 6: sync settings.'
 WHERE key = 'schema_notes';

PRAGMA user_version = 11;
