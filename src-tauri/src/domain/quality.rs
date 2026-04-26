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

    /// yt-dlp `-f` format selector for this profile.  Returned as a
    /// static string — no allocation per invocation.
    ///
    /// **Audio invariance (ADR-0028).**  Every selector pairs the
    /// video stream with `bestaudio` so the audio track passes
    /// through unchanged into the muxed file.  The downstream ffmpeg
    /// pass uses `-c:a copy`, completing the byte-exact passthrough
    /// chain.  A future change that swaps `bestaudio` for an
    /// audio-codec-restricted selector would silently break the
    /// guarantee — the unit test
    /// `every_selector_contains_bestaudio` is the trip-wire.
    ///
    /// **Codec policy.**  Selectors filter on `height` and `fps` only,
    /// never on `vcodec`.  Twitch overwhelmingly delivers H.264; the
    /// Phase 8 H.265 target is reached by the post-download re-encode
    /// pass (`services::reencode`), not by yt-dlp's source pick.
    ///
    /// **30-fps height-first chain.**  yt-dlp's `/` is a left-to-right
    /// fallback: the first arm whose filters match *any* available
    /// format wins, and `bestvideo` then picks the highest-quality
    /// match within that arm.  A naive
    /// `bestvideo[height<=720][fps<=30]` arm therefore picks 480p30
    /// out of `{720p60, 480p30}` — both formats satisfy
    /// `[height<=720][fps<=30]` after the filter, but 720p60 is
    /// excluded by the fps clause, so `bestvideo` picks 480p30 even
    /// though 720p60 was the user's actual height target.
    ///
    /// ADR-0028 explicitly frames 720p as the GTA-RP floor for text
    /// legibility, so we want the height target honoured even at
    /// the cost of the fps target (the re-encoder downsamples fps).
    /// The fix: probe the exact target height first (with and
    /// without the fps filter), only then walk down through lower
    /// heights.  Twitch heights are discrete (1080 / 720 / 480 /
    /// 360 / 160) so `[height=N]` is a reliable exact-match.
    ///
    /// 60-fps profiles don't need the same expansion — `[fps<=60]`
    /// accepts every reasonable Twitch variant — so they stay on
    /// the simpler `[height<=N][fps<=60]` chain.  The final `/best`
    /// in every chain is the safety net for sources missing every
    /// height tier.
    pub fn format_selector(self) -> &'static str {
        match self {
            VideoQualityProfile::Source => "bestvideo+bestaudio/best",
            VideoQualityProfile::P480p30 => {
                "bestvideo[height=480][fps<=30]+bestaudio\
                 /bestvideo[height=480]+bestaudio\
                 /bestvideo[height<480][fps<=30]+bestaudio\
                 /bestvideo[height<480]+bestaudio\
                 /best[height<=480]/best"
            }
            VideoQualityProfile::P480p60 => {
                "bestvideo[height<=480][fps<=60]+bestaudio\
                 /best[height<=480][fps<=60]/best"
            }
            VideoQualityProfile::P720p30 => {
                "bestvideo[height=720][fps<=30]+bestaudio\
                 /bestvideo[height=720]+bestaudio\
                 /bestvideo[height<720][fps<=30]+bestaudio\
                 /bestvideo[height<720]+bestaudio\
                 /best[height<=720]/best"
            }
            VideoQualityProfile::P720p60 => {
                "bestvideo[height<=720][fps<=60]+bestaudio\
                 /best[height<=720][fps<=60]/best"
            }
            VideoQualityProfile::P1080p30 => {
                "bestvideo[height=1080][fps<=30]+bestaudio\
                 /bestvideo[height=1080]+bestaudio\
                 /bestvideo[height<1080][fps<=30]+bestaudio\
                 /bestvideo[height<1080]+bestaudio\
                 /best[height<=1080]/best"
            }
            VideoQualityProfile::P1080p60 => {
                "bestvideo[height<=1080][fps<=60]+bestaudio\
                 /best[height<=1080][fps<=60]/best"
            }
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
// Test code: panics on assertion failures are the contract; the
// crate-wide bans on unwrap/expect don't apply here.
#[allow(clippy::unwrap_used, clippy::expect_used)]
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

    /// ADR-0028 audio-passthrough invariant trip-wire: every
    /// selector must include `bestaudio` so the muxed audio stream
    /// is the source stream byte-for-byte.  A future change that
    /// swaps in `aac` / `libopus` / etc. flips this test red.
    #[test]
    fn every_selector_contains_bestaudio() {
        for p in [
            VideoQualityProfile::P480p30,
            VideoQualityProfile::P480p60,
            VideoQualityProfile::P720p30,
            VideoQualityProfile::P720p60,
            VideoQualityProfile::P1080p30,
            VideoQualityProfile::P1080p60,
            VideoQualityProfile::Source,
        ] {
            let sel = p.format_selector();
            assert!(
                sel.contains("bestaudio"),
                "{p:?} selector missing bestaudio: {sel}"
            );
        }
    }

    /// Selectors must NEVER carry a `vcodec` filter — Twitch's H.264
    /// source is always acceptable as input and the H.265 target is
    /// the re-encode pass's job, not yt-dlp's source pick.
    #[test]
    fn no_selector_filters_on_vcodec() {
        for p in [
            VideoQualityProfile::P480p30,
            VideoQualityProfile::P480p60,
            VideoQualityProfile::P720p30,
            VideoQualityProfile::P720p60,
            VideoQualityProfile::P1080p30,
            VideoQualityProfile::P1080p60,
            VideoQualityProfile::Source,
        ] {
            let sel = p.format_selector();
            assert!(
                !sel.contains("vcodec"),
                "{p:?} selector unexpectedly filters on vcodec: {sel}"
            );
        }
    }

    #[test]
    fn source_selector_passes_through_best_quality() {
        // Source must not impose any height / fps cap — pure passthrough.
        let sel = VideoQualityProfile::Source.format_selector();
        assert_eq!(sel, "bestvideo+bestaudio/best");
        assert!(!sel.contains("height"), "Source must not cap height: {sel}");
        assert!(!sel.contains("fps"), "Source must not cap fps: {sel}");
    }

    #[test]
    fn capped_profiles_use_their_height_in_every_height_clause() {
        // For each non-Source profile, every `[height<=N]` clause must
        // use the profile's max_height.  Catches a copy/paste bug
        // where, e.g., the 720p30 selector accidentally references 1080.
        for p in [
            VideoQualityProfile::P480p30,
            VideoQualityProfile::P480p60,
            VideoQualityProfile::P720p30,
            VideoQualityProfile::P720p60,
            VideoQualityProfile::P1080p30,
            VideoQualityProfile::P1080p60,
        ] {
            let cap = p.max_height().unwrap();
            let needle = format!("[height<={cap}]");
            let sel = p.format_selector();
            assert!(
                sel.contains(&needle),
                "{p:?} selector missing {needle}: {sel}"
            );
        }
    }

    #[test]
    fn capped_profiles_request_their_target_fps() {
        // Each non-Source profile must include at least one
        // `[fps<=N]` clause matching its max_fps.  This is what
        // actually drives Twitch's variant pick — without it the
        // 30-fps profiles would silently grab 60-fps variants.
        for p in [
            VideoQualityProfile::P480p30,
            VideoQualityProfile::P480p60,
            VideoQualityProfile::P720p30,
            VideoQualityProfile::P720p60,
            VideoQualityProfile::P1080p30,
            VideoQualityProfile::P1080p60,
        ] {
            let target = p.max_fps().unwrap();
            let needle = format!("[fps<={target}]");
            let sel = p.format_selector();
            assert!(
                sel.contains(&needle),
                "{p:?} selector missing {needle}: {sel}"
            );
        }
    }

    #[test]
    fn thirty_fps_profiles_have_height_only_fallback() {
        // 30-fps profiles need a height-only fallback so a source
        // exposing only 60-fps variants still downloads at the
        // requested height (the re-encoder then downsamples fps).
        for p in [
            VideoQualityProfile::P480p30,
            VideoQualityProfile::P720p30,
            VideoQualityProfile::P1080p30,
        ] {
            let cap = p.max_height().unwrap();
            let needle = format!("/bestvideo[height={cap}]+bestaudio");
            let sel = p.format_selector();
            assert!(
                sel.contains(&needle),
                "{p:?} selector missing height-only fallback {needle}: {sel}"
            );
        }
    }

    /// R-RC-01 fix: yt-dlp's `/` is left-to-right "first match wins";
    /// each arm runs `bestvideo` against whatever survives that arm's
    /// filters.  For 30-fps profiles that means the
    /// `[height={N}]+bestaudio` arm (no fps filter) must come AFTER
    /// the `[height={N}][fps<=30]+bestaudio` arm — otherwise the
    /// height-only arm matches first and `bestvideo` picks the 60-fps
    /// variant even when a 30-fps variant exists.  This trip-wire pins
    /// the relative position so a future engineer who "tidies up" the
    /// chain can't silently regress it.
    #[test]
    fn thirty_fps_profiles_prefer_fps_match_over_any_fps_at_target_height() {
        for p in [
            VideoQualityProfile::P480p30,
            VideoQualityProfile::P720p30,
            VideoQualityProfile::P1080p30,
        ] {
            let cap = p.max_height().unwrap();
            let with_fps = format!("[height={cap}][fps<=30]+bestaudio");
            let any_fps = format!("[height={cap}]+bestaudio");
            let sel = p.format_selector();
            let with_fps_pos = sel.find(&with_fps).expect("with-fps arm present");
            let any_fps_pos = sel.find(&any_fps).expect("any-fps arm present");
            assert!(
                with_fps_pos < any_fps_pos,
                "{p:?}: `{with_fps}` arm must precede `{any_fps}` arm \
                 so yt-dlp picks the 30-fps variant when one exists; got: {sel}"
            );
        }
    }

    /// Equally important: the exact-target-height arms must come
    /// BEFORE the lower-height arms, so a `{1080p60, 720p60, 480p30}`
    /// source picks 720p60 (height target hit) for a P720p30 request
    /// rather than 480p30 (one of the lower-height variants).  Pins
    /// the height-first chain that ADR-0028's "720p as floor"
    /// argument depends on.
    #[test]
    fn thirty_fps_profiles_prefer_target_height_over_lower_height() {
        for p in [
            VideoQualityProfile::P480p30,
            VideoQualityProfile::P720p30,
            VideoQualityProfile::P1080p30,
        ] {
            let cap = p.max_height().unwrap();
            let target = format!("[height={cap}]+bestaudio");
            let lower = format!("[height<{cap}][fps<=30]+bestaudio");
            let sel = p.format_selector();
            let target_pos = sel.find(&target).expect("target-height arm present");
            let lower_pos = sel.find(&lower).expect("lower-height arm present");
            assert!(
                target_pos < lower_pos,
                "{p:?}: `{target}` arm must precede `{lower}` arm; got: {sel}"
            );
        }
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
