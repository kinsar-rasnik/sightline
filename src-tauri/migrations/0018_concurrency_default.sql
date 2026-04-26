-- 0018_concurrency_default.sql
-- v2.0.3 hotfix — clamp app_settings.max_concurrent_downloads to the
-- new safe range 1..=3.  See ADR-0035 (download engine settings
-- wiring) for the why: up to v2.0.2 the worker layer ignored the
-- slider value entirely and the service-layer clamp accepted up to 5.
-- The new code (services::downloads) enforces the cap live and the
-- service-layer clamp matches the slider's new 1..=3 range.
--
-- Existing values 1, 2, or 3 are preserved — those are explicit user
-- choices.  Anything NULL (corrupt) or above 3 is flattened to the
-- safest default (1).  CEO has 2 today; this migration leaves it
-- alone.
--
-- The column-level DEFAULT (set in migration 0004 to 2) cannot be
-- changed without a table rewrite — accepted limitation for v2.0.3,
-- since fresh installs land on 2 which is still inside the new safe
-- range and the worker enforces the cap.

UPDATE app_settings
   SET max_concurrent_downloads = 1
 WHERE max_concurrent_downloads IS NULL
    OR max_concurrent_downloads > 3;

UPDATE schema_meta
   SET value = 'See docs/data-model.md; migrations are append-only. v2.0.3: download concurrency cap.'
 WHERE key = 'schema_notes';

PRAGMA user_version = 18;
