-- 0010_sync_sessions.sql
-- Phase 6 — Multi-View Sync Engine (Split-View v1).
-- Author-phase: Phase 6.
-- Rollback: forward-only. The two tables are pure derivable state from
--           a frontend session description; dropping them in a future
--           migration is a clean operation (no foreign data references
--           sync rows from elsewhere).
--
-- Persists the *definition* of a multi-view sync session — which VODs,
-- which pane is leader, the layout choice — so that v2's "resume
-- previous multi-view session" feature has the audit trail it needs
-- (see ADR-0021 §Persistence).  Live frame-by-frame state (the panes'
-- currentTime, drift measurements) stays in frontend memory; this
-- table only describes *what* a session looked like, not *where it
-- was* at a given instant.

CREATE TABLE IF NOT EXISTS sync_sessions (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    created_at      INTEGER NOT NULL,
    closed_at       INTEGER,
    -- Layout vocabulary mirrors `domain::sync::SyncLayout`. v1 only
    -- ships 'split-50-50'; the column is a TEXT so v2 can add 'pip'
    -- and '2x2' without a schema bump.
    layout          TEXT    NOT NULL DEFAULT 'split-50-50' CHECK (layout IN (
        'split-50-50'
    )),
    -- The pane currently driving the wall-clock; matches one of the
    -- `pane_index` values in `sync_session_panes`. NULL during
    -- session construction (between `INSERT INTO sync_sessions` and
    -- the follow-up `UPDATE` that sets the leader).
    leader_pane_index INTEGER CHECK (leader_pane_index IS NULL OR leader_pane_index >= 0),
    status          TEXT    NOT NULL DEFAULT 'active' CHECK (status IN (
        'active', 'closed'
    ))
);

CREATE INDEX IF NOT EXISTS idx_sync_sessions_status_created
    ON sync_sessions(status, created_at DESC);

-- Per-session pane membership. v1 is fixed at 2 panes (pane_index in
-- {0, 1}); v2 may grow the upper bound by relaxing the CHECK.
CREATE TABLE IF NOT EXISTS sync_session_panes (
    session_id  INTEGER NOT NULL REFERENCES sync_sessions(id) ON DELETE CASCADE,
    pane_index  INTEGER NOT NULL CHECK (pane_index >= 0 AND pane_index <= 1),
    vod_id      TEXT    NOT NULL REFERENCES vods(twitch_video_id) ON DELETE CASCADE,
    -- Per-pane audio mix (v1: per-pane volume + mute, no crossfader).
    volume      REAL    NOT NULL DEFAULT 1.0
                        CHECK (volume >= 0.0 AND volume <= 1.0),
    muted       INTEGER NOT NULL DEFAULT 0 CHECK (muted IN (0, 1)),
    joined_at   INTEGER NOT NULL,
    PRIMARY KEY (session_id, pane_index)
);

-- A session shouldn't have two panes pointing at the same VOD — the
-- whole point is multiple perspectives. Enforced as a unique partial
-- index so the DB rejects accidental duplicates.
CREATE UNIQUE INDEX IF NOT EXISTS idx_sync_session_panes_one_vod_per_session
    ON sync_session_panes(session_id, vod_id);

UPDATE schema_meta
   SET value = 'See docs/data-model.md; migrations are append-only. Phase 6: sync_sessions.'
 WHERE key = 'schema_notes';

PRAGMA user_version = 10;
