-- 0015_quality_settings.sql
-- Phase 8 — Quality pipeline + background-friendly re-encode.
-- Author-phase: Phase 8.
-- Rollback: forward-only. SQLite ALTER ADD COLUMN is irreversible
--           without a table rewrite.
--
-- Adds the user-configurable quality pipeline settings.  See
-- ADR-0028 (quality pipeline / 720p30 H.265 default) and ADR-0029
-- (background-friendly re-encode / nice + adaptive suspend).

-- The chosen quality profile.  New installs get '720p30' as
-- documented in ADR-0028 §Decision; existing installs preserve their
-- v1.0 `quality_preset` value separately (the legacy column stays
-- untouched).  The two values converge at the service layer, with
-- `video_quality_profile` taking precedence when both are set.
ALTER TABLE app_settings
    ADD COLUMN video_quality_profile TEXT NOT NULL DEFAULT '720p30'
        CHECK (video_quality_profile IN (
            '480p30','480p60','720p30','720p60',
            '1080p30','1080p60','source'
        ));

-- Software-encode opt-in.  Default 0 — the user must explicitly
-- enable libx265 fallback because it can saturate the CPU during
-- gaming (ADR-0028 §Decision).  When 0 and no hardware encoder is
-- available, the service surfaces an error instead of silently
-- using software.
ALTER TABLE app_settings
    ADD COLUMN software_encode_opt_in INTEGER NOT NULL DEFAULT 0
        CHECK (software_encode_opt_in IN (0, 1));

-- Encoder-capability JSON populated by `services::encoder_detection`
-- on first start (or "Re-detect" click).  Format documented in
-- ADR-0028 §Detection.  NULL means "never detected" — the service
-- runs detection on the next get-capability call and persists the
-- result.
ALTER TABLE app_settings
    ADD COLUMN encoder_capability TEXT;

-- Concurrency cap on background re-encodes.  Hard-clamped 1..=2 in
-- the service-layer write path; the column-level CHECK is
-- defence-in-depth so a hand-edit can't escape the bound.
ALTER TABLE app_settings
    ADD COLUMN max_concurrent_reencodes INTEGER NOT NULL DEFAULT 1
        CHECK (max_concurrent_reencodes >= 1
               AND max_concurrent_reencodes <= 2);

-- Adaptive-throttle thresholds.  The default 0.7 / 0.5 pair gives a
-- 20-percentage-point hysteresis band that avoids thrash during the
-- spiky load patterns games produce (ADR-0029 §Layer 2).
ALTER TABLE app_settings
    ADD COLUMN cpu_throttle_high_threshold REAL NOT NULL DEFAULT 0.7
        CHECK (cpu_throttle_high_threshold >= 0.5
               AND cpu_throttle_high_threshold <= 0.9);

ALTER TABLE app_settings
    ADD COLUMN cpu_throttle_low_threshold REAL NOT NULL DEFAULT 0.5
        CHECK (cpu_throttle_low_threshold >= 0.3
               AND cpu_throttle_low_threshold <= 0.8);

UPDATE schema_meta
   SET value = 'See docs/data-model.md; migrations are append-only. Phase 8: quality settings.'
 WHERE key = 'schema_notes';

PRAGMA user_version = 15;
