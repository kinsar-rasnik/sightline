-- 0008_watch_progress.sql
-- Phase: 5 (Player + watch progress).
-- Purpose: persist resume-from-position, watched fraction, and
--          per-VOD total-watch-seconds stats so the Continue Watching
--          row and the Phase-7 auto-cleanup policies can be built on
--          real data. See ADR-0018.
-- Rollback: forward-only. `watched_fraction` is a STORED generated
--           column — a future migration that shrinks the schema would
--           have to drop+reseed the table, which is acceptable given
--           this data is a derived view of playback history.

CREATE TABLE IF NOT EXISTS watch_progress (
    vod_id                         TEXT    PRIMARY KEY
                                           REFERENCES vods(twitch_video_id) ON DELETE CASCADE,
    -- Last reported playback head, in seconds. REAL so the service
    -- can round to 0.5 s resolution without precision loss.
    position_seconds               REAL    NOT NULL DEFAULT 0
                                           CHECK (position_seconds >= 0),
    duration_seconds               REAL    NOT NULL
                                           CHECK (duration_seconds >= 0),
    -- Generated so the indexer always produces a consistent value
    -- and the UI can sort by "% watched" without reading two columns.
    -- STORED (not VIRTUAL) so we can index on it if needed later.
    watched_fraction               REAL    GENERATED ALWAYS AS (
        CASE WHEN duration_seconds > 0
             THEN position_seconds / duration_seconds
             ELSE 0
        END
    ) STORED,
    state                          TEXT    NOT NULL
                                           CHECK (state IN (
                                               'unwatched',
                                               'in_progress',
                                               'completed',
                                               'manually_watched'
                                           )),
    first_watched_at               INTEGER,
    last_watched_at                INTEGER NOT NULL,
    last_session_duration_seconds  REAL    NOT NULL DEFAULT 0
                                           CHECK (last_session_duration_seconds >= 0),
    -- Accumulates real playback time (not wall-clock) using the
    -- interval-merger in `domain::interval_merger` so scrubbing over
    -- already-seen territory doesn't double-count.
    total_watch_seconds            REAL    NOT NULL DEFAULT 0
                                           CHECK (total_watch_seconds >= 0)
);

CREATE INDEX IF NOT EXISTS idx_watch_progress_last_watched
    ON watch_progress(last_watched_at DESC);
CREATE INDEX IF NOT EXISTS idx_watch_progress_state
    ON watch_progress(state);

UPDATE schema_meta
   SET value = 'See docs/data-model.md; migrations are append-only. Phase 5: watch_progress.'
 WHERE key = 'schema_notes';

PRAGMA user_version = 8;
