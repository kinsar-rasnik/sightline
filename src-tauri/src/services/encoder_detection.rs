//! Hardware-encoder detection service (Phase 8, ADR-0028).
//!
//! Two-stage probe:
//!
//! 1. `ffmpeg -encoders` listing via [`crate::infra::ffmpeg::Ffmpeg::list_encoders`].
//!    Cheap; identifies which encoders the bundled ffmpeg knows
//!    about.  Necessary but not sufficient — a sidecar built with
//!    NVENC support might still fail on a machine without an NVIDIA
//!    GPU.
//! 2. 1-second synthetic test encode via
//!    [`crate::infra::ffmpeg::Ffmpeg::test_encoder`].  Confirms the
//!    encoder actually initialises.  We test only the platform's
//!    preference order from
//!    [`crate::domain::quality::EncoderCapability::detection_order`]
//!    and stop at the first encoder that passes both stages.
//!
//! The result is persisted via
//! [`crate::services::settings::SettingsService::record_encoder_capability`]
//! so the Settings UI can render "Auto-detected (5 minutes ago)"
//! without re-running the probe on every page load.

use std::sync::Arc;

use tokio::sync::Mutex;
use tracing::{debug, info, warn};

use crate::domain::quality::{EncoderCapability, EncoderKind};
use crate::error::AppError;
use crate::infra::clock::Clock;
use crate::infra::ffmpeg::SharedFfmpeg;
use crate::services::settings::SettingsService;

#[derive(Debug)]
pub struct EncoderDetectionService {
    ffmpeg: SharedFfmpeg,
    settings: SettingsService,
    clock: Arc<dyn Clock>,
    target_os: &'static str,
    /// Serialises concurrent `detect_and_persist` calls.  The
    /// startup task and the user-driven "Re-detect" button share
    /// the same SettingsService write path; without this guard, two
    /// 1-second test encodes can race in the cold-start window.
    /// See R-RC-01 finding P1 on commit 94e4340.
    detect_lock: Arc<Mutex<()>>,
}

impl EncoderDetectionService {
    pub fn new(ffmpeg: SharedFfmpeg, settings: SettingsService, clock: Arc<dyn Clock>) -> Self {
        Self {
            ffmpeg,
            settings,
            clock,
            target_os: detect_target_os(),
            detect_lock: Arc::new(Mutex::new(())),
        }
    }

    /// Construct with an explicit target_os string.  Used by tests
    /// to exercise the per-OS preference order without rebuilding
    /// the binary.
    #[cfg(test)]
    pub fn with_target_os(
        ffmpeg: SharedFfmpeg,
        settings: SettingsService,
        clock: Arc<dyn Clock>,
        target_os: &'static str,
    ) -> Self {
        Self {
            ffmpeg,
            settings,
            clock,
            target_os,
            detect_lock: Arc::new(Mutex::new(())),
        }
    }

    /// Run the detection probe and persist the result.  Returns the
    /// chosen capability for the caller to render immediately
    /// (avoids a second round-trip to the settings table).
    ///
    /// Guarded by `detect_lock` — concurrent callers (startup task +
    /// "Re-detect" button) serialise so the 1-second test encode
    /// runs at most once per logical detection request.
    pub async fn detect_and_persist(&self) -> Result<EncoderCapability, AppError> {
        let _guard = self.detect_lock.lock().await;
        let capability = self.detect().await?;
        self.settings.record_encoder_capability(&capability).await?;
        info!(
            primary = %capability.primary.as_str(),
            available = ?capability.available.iter().map(|e| e.as_str()).collect::<Vec<_>>(),
            "encoder detection complete"
        );
        Ok(capability)
    }

    /// Pure detection — does not write to the DB.  Useful when the
    /// caller wants to preview the result before persisting (the
    /// "Re-detect" button in Settings shows this then asks for
    /// confirmation when the primary encoder changes).
    pub async fn detect(&self) -> Result<EncoderCapability, AppError> {
        let listed = self.ffmpeg.list_encoders().await.map_err(|e| {
            warn!(error = ?e, "ffmpeg -encoders failed");
            e
        })?;
        let listed_names: Vec<&str> = listed.iter().map(|e| e.name.as_str()).collect();
        debug!(?listed_names, "ffmpeg -encoders parsed");

        let order = EncoderCapability::detection_order(self.target_os);
        let mut available: Vec<EncoderKind> = Vec::new();
        let mut primary: Option<EncoderKind> = None;
        let mut primary_h265 = false;
        let mut primary_h264 = false;

        for kind in &order {
            let hevc = kind.hevc_encoder_arg();
            let h264 = kind.h264_encoder_arg();
            let hevc_listed = listed_names.contains(&hevc);
            let h264_listed = listed_names.contains(&h264);
            if !(hevc_listed || h264_listed) {
                continue;
            }
            // Test the H.265 path first; that's our default.  If
            // H.265 fails, fall back to testing H.264 — some
            // hardware encoders (older AMF) shipped without HEVC.
            let hevc_works = hevc_listed && self.ffmpeg.test_encoder(hevc).await.is_ok();
            let h264_works = if hevc_works {
                // Skip the second test — we know the encoder
                // initialises.  Save the cold-start ~1 s.
                h264_listed
            } else {
                h264_listed && self.ffmpeg.test_encoder(h264).await.is_ok()
            };
            if !hevc_works && !h264_works {
                debug!(kind = %kind.as_str(), "test_encoder failed for both codecs");
                continue;
            }
            available.push(*kind);
            if primary.is_none() {
                primary = Some(*kind);
                primary_h265 = hevc_works;
                primary_h264 = h264_works;
            }
        }

        // Software fallback always present (libx265/libx264 ship
        // with every ffmpeg sidecar build per ADR-0013).  Only
        // when the listing genuinely doesn't contain libx265 do we
        // record an "everything failed" state.
        let primary = primary.unwrap_or(EncoderKind::Software);
        if !available.contains(&primary) {
            available.push(primary);
        }
        if primary == EncoderKind::Software {
            primary_h265 = listed_names.contains(&"libx265");
            primary_h264 = listed_names.contains(&"libx264");
        }

        Ok(EncoderCapability {
            primary,
            available,
            h265: primary_h265,
            h264: primary_h264,
            tested_at: self.clock.unix_seconds(),
        })
    }
}

/// Resolve the target OS string used by [`EncoderCapability::detection_order`].
fn detect_target_os() -> &'static str {
    if cfg!(target_os = "macos") {
        "macos"
    } else if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "linux") {
        "linux"
    } else {
        "other"
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::infra::clock::FixedClock;
    use crate::infra::db::Db;
    use crate::infra::ffmpeg::SharedFfmpeg;
    use crate::infra::ffmpeg::fake::{FfmpegFake, FfmpegScript};

    async fn setup(target_os: &'static str, script: FfmpegScript) -> EncoderDetectionService {
        let db = Db::open_in_memory().await.unwrap();
        db.migrate().await.unwrap();
        let clock: Arc<dyn Clock> = Arc::new(FixedClock::at(1_700_000_000));
        let settings = SettingsService::new(db.clone(), clock.clone());
        let ffmpeg: SharedFfmpeg = Arc::new(FfmpegFake::new(script));
        EncoderDetectionService::with_target_os(ffmpeg, settings, clock, target_os)
    }

    #[tokio::test]
    async fn macos_detects_videotoolbox_when_listed_and_tested() {
        let svc = setup(
            "macos",
            FfmpegScript {
                encoders: vec![
                    "hevc_videotoolbox".into(),
                    "libx265".into(),
                    "libx264".into(),
                ],
                working_encoders: vec!["hevc_videotoolbox".into(), "libx265".into()],
                ..Default::default()
            },
        )
        .await;
        let cap = svc.detect().await.unwrap();
        assert_eq!(cap.primary, EncoderKind::VideoToolbox);
        assert!(cap.h265);
        assert!(cap.available.contains(&EncoderKind::VideoToolbox));
        assert!(cap.available.contains(&EncoderKind::Software));
    }

    #[tokio::test]
    async fn windows_falls_through_when_nvenc_listed_but_test_fails() {
        // Listed (driver thinks NVENC is present) but test_encoder
        // fails (no NVIDIA card actually attached).  Should fall
        // through to software.
        let svc = setup(
            "windows",
            FfmpegScript {
                encoders: vec!["hevc_nvenc".into(), "libx265".into(), "libx264".into()],
                working_encoders: vec!["libx265".into()], // NVENC test fails
                ..Default::default()
            },
        )
        .await;
        let cap = svc.detect().await.unwrap();
        assert_eq!(cap.primary, EncoderKind::Software);
        assert!(cap.h265);
    }

    #[tokio::test]
    async fn detection_persists_via_settings_service() {
        let svc = setup(
            "macos",
            FfmpegScript {
                encoders: vec!["hevc_videotoolbox".into(), "libx265".into()],
                working_encoders: vec!["hevc_videotoolbox".into()],
                ..Default::default()
            },
        )
        .await;
        let cap = svc.detect_and_persist().await.unwrap();
        let from_settings = svc.settings.get().await.unwrap().encoder_capability;
        assert_eq!(from_settings, Some(cap));
    }

    #[tokio::test]
    async fn linux_prefers_vaapi_when_available() {
        let svc = setup(
            "linux",
            FfmpegScript {
                encoders: vec!["hevc_vaapi".into(), "libx265".into(), "libx264".into()],
                working_encoders: vec!["hevc_vaapi".into(), "libx265".into()],
                ..Default::default()
            },
        )
        .await;
        let cap = svc.detect().await.unwrap();
        assert_eq!(cap.primary, EncoderKind::Vaapi);
    }

    #[tokio::test]
    async fn empty_listing_yields_software_with_no_codecs() {
        let svc = setup(
            "macos",
            FfmpegScript {
                encoders: vec![],
                working_encoders: vec![],
                ..Default::default()
            },
        )
        .await;
        let cap = svc.detect().await.unwrap();
        assert_eq!(cap.primary, EncoderKind::Software);
        // Without libx265/libx264 in the listing the codec flags
        // should be false — caller should refuse to enable software
        // encode in this state.
        assert!(!cap.h265);
        assert!(!cap.h264);
    }
}
