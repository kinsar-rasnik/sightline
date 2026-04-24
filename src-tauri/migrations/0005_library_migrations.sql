-- 0005_library_migrations.sql
-- Phase: 3 (Download engine).
-- Purpose: audit trail of library-layout migrations. Only one
--          non-terminal migration can be active at a time (enforced in
--          services/library_migrator.rs; the unique partial index on
--          `status` makes it a hard invariant at the DB layer too).
-- Rollback: forward-only.

CREATE TABLE IF NOT EXISTS library_migrations (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    started_at  INTEGER NOT NULL,                         -- unix seconds UTC
    finished_at INTEGER,                                   -- NULL while in progress
    from_layout TEXT    NOT NULL,
    to_layout   TEXT    NOT NULL,
    moved       INTEGER NOT NULL DEFAULT 0,                -- count of files successfully moved
    errors      INTEGER NOT NULL DEFAULT 0,                -- count of per-file errors
    status      TEXT    NOT NULL                           -- running | completed | failed | cancelled
                        CHECK (status IN ('running', 'completed', 'failed', 'cancelled'))
);

-- At most one running migration at a time.
CREATE UNIQUE INDEX IF NOT EXISTS idx_library_migrations_one_running
    ON library_migrations(status) WHERE status = 'running';
CREATE INDEX IF NOT EXISTS idx_library_migrations_recent
    ON library_migrations(started_at DESC);

PRAGMA user_version = 5;
