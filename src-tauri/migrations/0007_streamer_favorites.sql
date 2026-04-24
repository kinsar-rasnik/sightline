-- 0007_streamer_favorites.sql
-- Phase 4 — favorites + per-streamer preferences seed.
-- Author-phase: Phase 4.
-- Rollback: forward-only (column adds are unreversible in SQLite
-- without a table rewrite; a future migration would clone the table).

ALTER TABLE streamers ADD COLUMN favorite INTEGER NOT NULL DEFAULT 0 CHECK (favorite IN (0, 1));

-- Settings bump: window close behavior, start-at-login, notification prefs.
-- All optional; defaults chosen so upgraders see the same behaviour as
-- before (close-to-hide is the new default — hotfix flagged in Phase 4
-- release notes).
ALTER TABLE app_settings ADD COLUMN window_close_behavior    TEXT    NOT NULL DEFAULT 'hide' CHECK (window_close_behavior IN ('hide', 'quit'));
ALTER TABLE app_settings ADD COLUMN start_at_login           INTEGER NOT NULL DEFAULT 0 CHECK (start_at_login IN (0, 1));
ALTER TABLE app_settings ADD COLUMN show_dock_icon           INTEGER NOT NULL DEFAULT 0 CHECK (show_dock_icon IN (0, 1));
ALTER TABLE app_settings ADD COLUMN notifications_enabled    INTEGER NOT NULL DEFAULT 1 CHECK (notifications_enabled IN (0, 1));
ALTER TABLE app_settings ADD COLUMN notify_download_complete INTEGER NOT NULL DEFAULT 0 CHECK (notify_download_complete IN (0, 1));
ALTER TABLE app_settings ADD COLUMN notify_download_failed   INTEGER NOT NULL DEFAULT 1 CHECK (notify_download_failed IN (0, 1));
ALTER TABLE app_settings ADD COLUMN notify_favorites_ingest  INTEGER NOT NULL DEFAULT 1 CHECK (notify_favorites_ingest IN (0, 1));
ALTER TABLE app_settings ADD COLUMN notify_storage_low       INTEGER NOT NULL DEFAULT 1 CHECK (notify_storage_low IN (0, 1));

-- Keyboard shortcuts are stored as a JSON object mapping action_id → key
-- string (e.g. `{"library":"g l","focus_search":"/"}`). Empty object
-- means "use compiled-in defaults"; the frontend's ShortcutsStore
-- merges this over its defaults at startup.
ALTER TABLE app_settings ADD COLUMN shortcuts_json TEXT NOT NULL DEFAULT '{}';

PRAGMA user_version = 7;
