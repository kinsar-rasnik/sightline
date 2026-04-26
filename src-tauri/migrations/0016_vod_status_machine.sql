-- 0016_vod_status_machine.sql
-- Phase 8 — Pull-on-demand distribution model.
-- Author-phase: Phase 8.
-- Rollback: forward-only.
--
-- Adds the `status` column to `vods` and backfills it from the
-- existing `downloads` + `watch_progress` tables so the pull-model
-- state machine has a fully populated lifecycle from day one.  See
-- ADR-0030 §Migration path.

-- The lifecycle column.  CHECK constraint mirrors the closed enum
-- in domain::distribution::VodStatus.  Default 'available' so any
-- newly-discovered VOD enters the pull model in the unpicked state.
ALTER TABLE vods
    ADD COLUMN status TEXT NOT NULL DEFAULT 'available'
        CHECK (status IN (
            'available','queued','downloading','ready','archived','deleted'
        ));

-- Compound index used by the Library UI's per-streamer filter and
-- the sliding-window enforcer.  `status` is a low-cardinality
-- column so it's the leading key for filter selectivity, and the
-- secondary `stream_started_at DESC` matches the default UI sort.
CREATE INDEX idx_vods_streamer_status_started
    ON vods(twitch_user_id, status, stream_started_at DESC);

-- Backfill from existing rows.  Order matters: the later UPDATEs
-- override earlier ones for any row matching multiple predicates.
--
-- 1. Default 'available' is already in place via the column default.
--
-- 2. A completed download whose file still exists -> 'ready'
--    UNLESS the watch-progress state is completed/manually_watched,
--    in which case it should be archived.
--
-- 3. completed + watch_progress in {completed, manually_watched}
--    -> 'archived' (the user is done with this VOD).
--
-- 4. completed but the cleanup service has already removed the file
--    (state = 'failed_permanent' AND last_error = 'CLEANED_UP')
--    -> 'deleted'.
--
-- 5. queued -> 'queued', downloading -> 'queued' (crash-recovery
--    will reset 'downloading' on next service start anyway).
UPDATE vods
   SET status = 'ready'
 WHERE twitch_video_id IN (
     SELECT vod_id FROM downloads
      WHERE state = 'completed' AND final_path IS NOT NULL
 );

UPDATE vods
   SET status = 'archived'
 WHERE twitch_video_id IN (
     SELECT d.vod_id
       FROM downloads d
       JOIN watch_progress w ON w.vod_id = d.vod_id
      WHERE d.state = 'completed'
        AND w.state IN ('completed', 'manually_watched')
 );

UPDATE vods
   SET status = 'deleted'
 WHERE twitch_video_id IN (
     SELECT vod_id FROM downloads
      WHERE state = 'failed_permanent' AND last_error = 'CLEANED_UP'
 );

UPDATE vods
   SET status = 'queued'
 WHERE twitch_video_id IN (
     SELECT vod_id FROM downloads
      WHERE state IN ('queued','downloading')
 );

UPDATE schema_meta
   SET value = 'See docs/data-model.md; migrations are append-only. Phase 8: vod status machine.'
 WHERE key = 'schema_notes';

PRAGMA user_version = 16;
