-- 0001_init.sql
-- Phase: 1 (foundation)
-- Purpose: seed schema_meta so the `health` command has something to read,
--          and lock the initial PRAGMA user_version.
-- Rollback: forward-only. Any fix ships as a new migration.

CREATE TABLE IF NOT EXISTS schema_meta (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

INSERT OR IGNORE INTO schema_meta (key, value) VALUES
    ('app_name',     'sightline'),
    ('created_at',   CAST(strftime('%s','now') AS TEXT)),
    ('schema_notes', 'See docs/data-model.md; migrations are append-only.');

-- Schema version is authoritative via PRAGMA user_version.
-- Bump here whenever a migration changes the observable schema.
PRAGMA user_version = 1;
