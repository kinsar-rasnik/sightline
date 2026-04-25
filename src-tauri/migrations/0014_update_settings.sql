-- 0014_update_settings.sql
-- Phase 7 — Update-checker settings.
-- Author-phase: Phase 7.
-- Rollback: forward-only. SQLite ALTER ADD COLUMN is irreversible
--           without a table rewrite.
--
-- Adds the user-configurable knobs for the GitHub Releases update
-- checker.  See ADR-0026.

-- Master toggle.  Default off — privacy-aware (no network calls
-- without explicit opt-in).
ALTER TABLE app_settings
    ADD COLUMN update_check_enabled INTEGER NOT NULL DEFAULT 0
        CHECK (update_check_enabled IN (0, 1));

-- Wall-clock seconds (UTC) at which the daily check most recently
-- ran (any outcome — success, no-update, or error).  Nullable so a
-- brand-new install starts at "never checked".
ALTER TABLE app_settings
    ADD COLUMN update_check_last_run INTEGER;

-- Tag the user explicitly suppressed via the banner's "Skip this
-- version" action (e.g. "v1.2.3").  Nullable; cleared by passing
-- an empty string through `update_settings`.
ALTER TABLE app_settings
    ADD COLUMN update_check_skip_version TEXT;

UPDATE schema_meta
   SET value = 'See docs/data-model.md; migrations are append-only. Phase 7: update settings.'
 WHERE key = 'schema_notes';

PRAGMA user_version = 14;
