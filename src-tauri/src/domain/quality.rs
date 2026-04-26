//! Phase 8 video-quality vocabulary (ADR-0028).
//!
//! Pure types — no I/O, no allocation per call where it can be
//! avoided.  The persistence layer round-trips these through the
//! `video_quality_profile` and `encoder_capability` columns added by
//! migration 0015.

use serde::{Deserialize, Serialize};
use specta::Type;

/// User-facing quality profile.  The wire strings are the strings we
/// persist in `app_settings.video_quality_profile`; do not rename
/// them without a migration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum VideoQualityProfile {
    #[serde(rename = "480p30")]
    P480p30,
    #[serde(rename = "480p60")]
    P480p60,
    #[serde(rename = "720p30")]
    P720p30,
    #[serde(rename = "720p60")]
    P720p60,
    #[serde(rename = "1080p30")]
    P1080p30,
    #[serde(rename = "1080p60")]
    P1080p60,
    #[serde(rename = "source")]
    Source,
}

impl VideoQualityProfile {
    /// Wire string for SQL persistence.  Stable; column-level CHECK
    /// in migration 0015 enforces the same set.
    pub fn as_db_str(self) -> &'static str {
        match self {
            VideoQualityProfile::P480p30 => "480p30",
            VideoQualityProfile::P480p60 => "480p60",
            VideoQualityProfile::P720p30 => "720p30",
            VideoQualityProfile::P720p60 => "720p60",
            VideoQualityProfile::P1080p30 => "1080p30",
            VideoQualityProfile::P1080p60 => "1080p60",
            VideoQualityProfile::Source => "source",
        }
    }

    pub fn from_db_str(s: &str) -> Option<Self> {
        Some(match s {
            "480p30" => VideoQualityProfile::P480p30,
            "480p60" => VideoQualityProfile::P480p60,
            "720p30" => VideoQualityProfile::P720p30,
            "720p60" => VideoQualityProfile::P720p60,
            "1080p30" => VideoQualityProfile::P1080p30,
            "1080p60" => VideoQualityProfile::P1080p60,
            "source" => VideoQualityProfile::Source,
            _ => return None,
        })
    }

    /// Height ceiling implied by the profile.  `None` for `Source`
    /// (it follows whatever the upstream offers).
    pub fn max_height(self) -> Option<u32> {
        match self {
            VideoQualityProfile::P480p30 | VideoQualityProfile::P480p60 => Some(480),
            VideoQualityProfile::P720p30 | VideoQualityProfile::P720p60 => Some(720),
            VideoQualityProfile::P1080p30 | VideoQualityProfile::P1080p60 => Some(1080),
            VideoQualityProfile::Source => None,
        }
    }

    /// FPS ceiling implied by the profile.  `None` for `Source`.
    pub fn max_fps(self) -> Option<u32> {
        match self {
            VideoQualityProfile::P480p30
            | VideoQualityProfile::P720p30
            | VideoQualityProfile::P1080p30 => Some(30),
            VideoQualityProfile::P480p60
            | VideoQualityProfile::P720p60
            | VideoQualityProfile::P1080p60 => Some(60),
            VideoQualityProfile::Source => None,
        }
    }

    /// Whether the profile re-encodes the video stream.  `Source`
    /// is the only profile that passes through the upstream codec.
    pub fn re_encodes(self) -> bool {
        !matches!(self, VideoQualityProfile::Source)
    }

    /// Estimated GB / hour at this profile.  Static lookup table
    /// per ADR-0032 §Decision.  Used by the storage forecast and
    /// by the Settings UI's example math.
    pub fn quality_factor_gb_per_hour(self) -> f64 {
        match self {
            VideoQualityProfile::P480p30 => 0.30,
            VideoQualityProfile::P480p60 => 0.45,
            VideoQualityProfile::P720p30 => 0.70,
            VideoQualityProfile::P720p60 => 1.10,
            VideoQualityProfile::P1080p30 => 1.50,
            VideoQualityProfile::P1080p60 => 2.20,
            VideoQualityProfile::Source => 4.00,
        }
    }

    /// Human-readable label used by the Settings UI.  Stable strings
    /// are not part of the IPC contract — the renderer i18ns these
    /// in v2.x — but keeping them in the domain layer keeps the
    /// "quality factor + label" pair next to its source of truth.
    pub fn label(self) -> &'static str {
        match self {
            VideoQualityProfile::P480p30 => "480p · 30 fps · H.265",
            VideoQualityProfile::P480p60 => "480p · 60 fps · H.265",
            VideoQualityProfile::P720p30 => "720p · 30 fps · H.265 (recommended)",
            VideoQualityProfile::P720p60 => "720p · 60 fps · H.265",
            VideoQualityProfile::P1080p30 => "1080p · 30 fps · H.265",
            VideoQualityProfile::P1080p60 => "1080p · 60 fps · H.265",
            VideoQualityProfile::Source => "Source · no re-encode",
        }
    }
}

/// Hardware (or software) encoder available on this machine.  The
/// detection service (`services::encoder_detection`) builds an
/// [`EncoderCapability`] from running `ffmpeg -encoders` plus a
/// 2-second test encode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum EncoderKind {
    VideoToolbox,
    Nvenc,
    Amf,
    QuickSync,
    Vaapi,
    Software,
}

impl EncoderKind {
    pub fn as_str(self) -> &'static str {
        match self {
            EncoderKind::VideoToolbox => "video_toolbox",
            EncoderKind::Nvenc => "nvenc",
            EncoderKind::Amf => "amf",
            EncoderKind::QuickSync => "quick_sync",
            EncoderKind::Vaapi => "vaapi",
            EncoderKind::Software => "software",
        }
    }

    pub fn from_str_opt(s: &str) -> Option<Self> {
        Some(match s {
            "video_toolbox" => EncoderKind::VideoToolbox,
            "nvenc" => EncoderKind::Nvenc,
            "amf" => EncoderKind::Amf,
            "quick_sync" => EncoderKind::QuickSync,
            "vaapi" => EncoderKind::Vaapi,
            "software" => EncoderKind::Software,
            _ => return None,
        })
    }

    /// ffmpeg encoder identifier for the H.265 (HEVC) encode path.
    /// Returned as a static string — no allocation per call.
    pub fn hevc_encoder_arg(self) -> &'static str {
        match self {
            EncoderKind::VideoToolbox => "hevc_videotoolbox",
            EncoderKind::Nvenc => "hevc_nvenc",
            EncoderKind::Amf => "hevc_amf",
            EncoderKind::QuickSync => "hevc_qsv",
            EncoderKind::Vaapi => "hevc_vaapi",
            EncoderKind::Software => "libx265",
        }
    }

    /// ffmpeg encoder identifier for the H.264 fallback path.
    pub fn h264_encoder_arg(self) -> &'static str {
        match self {
            EncoderKind::VideoToolbox => "h264_videotoolbox",
            EncoderKind::Nvenc => "h264_nvenc",
            EncoderKind::Amf => "h264_amf",
            EncoderKind::QuickSync => "h264_qsv",
            EncoderKind::Vaapi => "h264_vaapi",
            EncoderKind::Software => "libx264",
        }
    }
}

/// Detection result persisted in `app_settings.encoder_capability`
/// as JSON.  Documented in ADR-0028 §Detection.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct EncoderCapability {
    /// Encoder the service will use first.  When this is
    /// [`EncoderKind::Software`] but `software_encode_opt_in = 0`
    /// in settings, the re-encode pass refuses to run and surfaces
    /// an error instead.
    pub primary: EncoderKind,
    /// Every encoder that passed both the `-encoders` listing AND
    /// the runtime test encode.  Always contains [`EncoderKind::Software`]
    /// in the fallback position because libx265 is bundled with
    /// every ffmpeg sidecar build (ADR-0013).
    pub available: Vec<EncoderKind>,
    /// Whether the primary encoder supports H.265.
    pub h265: bool,
    /// Whether the primary encoder supports H.264.
    pub h264: bool,
    /// Wall-clock seconds at which the detection ran.  Used by the
    /// Settings UI to render "Auto-detected (NN minutes ago)".
    pub tested_at: i64,
}

impl EncoderCapability {
    /// Pick the order in which to evaluate encoders for a given OS.
    /// The order matters for detection: we stop at the first encoder
    /// that passes both the listing and the runtime test, so this
    /// list IS the platform-specific preference policy from
    /// ADR-0028 §Encoder vocabulary.
    pub fn detection_order(target_os: &str) -> Vec<EncoderKind> {
        match target_os {
            "macos" => vec![EncoderKind::VideoToolbox, EncoderKind::Software],
            "windows" => vec![
                EncoderKind::Nvenc,
                EncoderKind::Amf,
                EncoderKind::QuickSync,
                EncoderKind::Software,
            ],
            "linux" => vec![EncoderKind::Vaapi, EncoderKind::Software],
            _ => vec![EncoderKind::Software],
        }
    }
}

/// Adaptive throttle thresholds, persisted in
/// `cpu_throttle_high_threshold` / `cpu_throttle_low_threshold`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ThrottleThresholds {
    pub high: f64,
    pub low: f64,
}

impl ThrottleThresholds {
    pub const DEFAULT: Self = ThrottleThresholds {
        high: 0.7,
        low: 0.5,
    };

    /// Clamp into the permitted ranges defined by migration 0015.
    pub fn clamped(self) -> Self {
        Self {
            high: self.high.clamp(0.5, 0.9),
            low: self.low.clamp(0.3, 0.8),
        }
    }

    /// True iff the low threshold sits below the high threshold by
    /// at least 5 percentage points (anti-thrash).  The service-
    /// layer write path rejects updates that violate this.
    pub fn is_well_formed(self) -> bool {
        self.high - self.low >= 0.05
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn quality_profile_round_trip() {
        for p in [
            VideoQualityProfile::P480p30,
            VideoQualityProfile::P480p60,
            VideoQualityProfile::P720p30,
            VideoQualityProfile::P720p60,
            VideoQualityProfile::P1080p30,
            VideoQualityProfile::P1080p60,
            VideoQualityProfile::Source,
        ] {
            assert_eq!(VideoQualityProfile::from_db_str(p.as_db_str()), Some(p));
        }
        assert_eq!(VideoQualityProfile::from_db_str("4k"), None);
    }

    #[test]
    fn quality_factor_table_matches_adr_0032() {
        assert!((VideoQualityProfile::P480p30.quality_factor_gb_per_hour() - 0.30).abs() < 1e-6);
        assert!((VideoQualityProfile::P720p30.quality_factor_gb_per_hour() - 0.70).abs() < 1e-6);
        assert!((VideoQualityProfile::P1080p60.quality_factor_gb_per_hour() - 2.20).abs() < 1e-6);
        assert!((VideoQualityProfile::Source.quality_factor_gb_per_hour() - 4.00).abs() < 1e-6);
    }

    #[test]
    fn re_encodes_only_non_source() {
        assert!(!VideoQualityProfile::Source.re_encodes());
        assert!(VideoQualityProfile::P720p30.re_encodes());
        assert!(VideoQualityProfile::P1080p60.re_encodes());
    }

    #[test]
    fn encoder_kind_round_trip() {
        for k in [
            EncoderKind::VideoToolbox,
            EncoderKind::Nvenc,
            EncoderKind::Amf,
            EncoderKind::QuickSync,
            EncoderKind::Vaapi,
            EncoderKind::Software,
        ] {
            assert_eq!(EncoderKind::from_str_opt(k.as_str()), Some(k));
        }
        assert_eq!(EncoderKind::from_str_opt("av1_nvenc"), None);
    }

    #[test]
    fn encoder_kind_args_are_stable() {
        // hevc_videotoolbox is the macOS H.265 encoder name. If this
        // ever changes upstream, the sidecar rebuild ADR captures it.
        assert_eq!(
            EncoderKind::VideoToolbox.hevc_encoder_arg(),
            "hevc_videotoolbox"
        );
        assert_eq!(EncoderKind::Nvenc.hevc_encoder_arg(), "hevc_nvenc");
        assert_eq!(EncoderKind::Software.hevc_encoder_arg(), "libx265");
        assert_eq!(EncoderKind::Software.h264_encoder_arg(), "libx264");
    }

    #[test]
    fn detection_order_is_platform_specific() {
        let mac = EncoderCapability::detection_order("macos");
        assert_eq!(mac.first(), Some(&EncoderKind::VideoToolbox));
        assert_eq!(mac.last(), Some(&EncoderKind::Software));

        let win = EncoderCapability::detection_order("windows");
        assert_eq!(win.first(), Some(&EncoderKind::Nvenc));
        assert!(win.contains(&EncoderKind::QuickSync));

        let lin = EncoderCapability::detection_order("linux");
        assert_eq!(lin.first(), Some(&EncoderKind::Vaapi));

        let other = EncoderCapability::detection_order("freebsd");
        assert_eq!(other, vec![EncoderKind::Software]);
    }

    #[test]
    fn throttle_thresholds_clamp_into_range() {
        let oob = ThrottleThresholds {
            high: 1.5,
            low: 0.0,
        }
        .clamped();
        assert!((oob.high - 0.9).abs() < 1e-6);
        assert!((oob.low - 0.3).abs() < 1e-6);
        let inrange = ThrottleThresholds {
            high: 0.65,
            low: 0.45,
        }
        .clamped();
        assert!((inrange.high - 0.65).abs() < 1e-6);
        assert!((inrange.low - 0.45).abs() < 1e-6);
    }

    #[test]
    fn throttle_thresholds_well_formed_check() {
        assert!(ThrottleThresholds::DEFAULT.is_well_formed());
        // 5-pp gap at exactly the boundary is well-formed.
        assert!(
            ThrottleThresholds {
                high: 0.55,
                low: 0.50
            }
            .is_well_formed()
        );
        // Low above high — not well-formed.
        assert!(
            !ThrottleThresholds {
                high: 0.50,
                low: 0.60
            }
            .is_well_formed()
        );
        // Equal values — not well-formed.
        assert!(
            !ThrottleThresholds {
                high: 0.50,
                low: 0.50
            }
            .is_well_formed()
        );
    }

    #[test]
    fn encoder_capability_serde_round_trip() {
        let cap = EncoderCapability {
            primary: EncoderKind::VideoToolbox,
            available: vec![EncoderKind::VideoToolbox, EncoderKind::Software],
            h265: true,
            h264: true,
            tested_at: 1_700_000_000,
        };
        let json = serde_json::to_string(&cap).unwrap();
        let parsed: EncoderCapability = serde_json::from_str(&json).unwrap();
        assert_eq!(cap, parsed);
    }

    #[test]
    fn quality_profile_max_height_and_fps_align_with_label() {
        assert_eq!(VideoQualityProfile::P720p30.max_height(), Some(720));
        assert_eq!(VideoQualityProfile::P720p30.max_fps(), Some(30));
        assert_eq!(VideoQualityProfile::P1080p60.max_height(), Some(1080));
        assert_eq!(VideoQualityProfile::P1080p60.max_fps(), Some(60));
        assert_eq!(VideoQualityProfile::Source.max_height(), None);
        assert_eq!(VideoQualityProfile::Source.max_fps(), None);
    }
}
