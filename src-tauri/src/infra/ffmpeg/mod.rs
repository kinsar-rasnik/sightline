//! ffmpeg sidecar wrapper.
//!
//! Scope is much narrower than yt-dlp: two operations only.
//!
//! * `remux_to_mp4` — container swap (e.g. `.ts` → `.mp4`) with
//!   `-c copy`, no re-encoding. yt-dlp usually produces an `.mp4`
//!   directly; this is the fallback path for format combinations it
//!   delivers fragmented.
//! * `extract_thumbnail` — single JPEG frame at a percentage offset
//!   into the VOD. Used for the library grid and the NFO thumb
//!   sidecar.
//!
//! Same trait-with-fake pattern as `infra::ytdlp`, plus a `version()`
//! for the startup health-check.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::error::AppError;

pub mod cli;

#[cfg(any(test, feature = "test-support"))]
pub mod fake;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FfmpegVersion {
    pub version: String,
    pub path: PathBuf,
}

/// Source and destination for a remux pass. `source` is read `-c copy`
/// into `destination` with the container ffmpeg infers from the
/// extension.
#[derive(Debug, Clone)]
pub struct RemuxSpec {
    pub source: PathBuf,
    pub destination: PathBuf,
}

/// Extract a single frame. `percent` is 0.0 .. 100.0; we translate to
/// an `-ss` seek time given the VOD's `duration_seconds`.
#[derive(Debug, Clone)]
pub struct ThumbnailSpec {
    pub source: PathBuf,
    pub destination: PathBuf,
    pub duration_seconds: i64,
    pub percent: f64,
}

/// Multi-frame extract used for the library grid's hover preview
/// (Phase 5 housekeeping). `frames[i]` is written atomically — if
/// any one frame fails the caller should treat the whole preview as
/// missing and fall back to the single thumbnail.
#[derive(Debug, Clone)]
pub struct PreviewFramesSpec {
    pub source: PathBuf,
    pub duration_seconds: i64,
    /// One (percent, destination) tuple per frame. The caller decides
    /// the percentages (we default to 6 evenly-spaced points at
    /// 15/30/45/60/75/90%) so the extractor stays parameter-free.
    pub frames: Vec<(f64, PathBuf)>,
}

/// Percentages used by the library grid preview. Public so tests and
/// the service layer share a single source of truth.
pub const PREVIEW_FRAME_PERCENTS: [f64; 6] = [15.0, 30.0, 45.0, 60.0, 75.0, 90.0];

/// Phase 8 (ADR-0028) — re-encode a source file with a chosen video
/// encoder.  Audio is always passed through with `-c:a copy`; the
/// service layer's `audio_passthrough_is_byte_exact` regression test
/// asserts this invariant on the round-trip output.
#[derive(Debug, Clone)]
pub struct ReencodeSpec {
    pub source: PathBuf,
    pub destination: PathBuf,
    /// `-c:v` argument (e.g. `"hevc_videotoolbox"`).  Picked by
    /// `services::encoder_detection` from the [`crate::domain::quality::EncoderKind`]
    /// chosen for this run.
    pub video_encoder_arg: String,
    /// Maximum height for the output, in pixels.  `None` for the
    /// `Source` profile (which never re-encodes anyway).
    pub max_height: Option<u32>,
    /// Maximum frame rate.  `None` for `Source`.
    pub max_fps: Option<u32>,
    /// Process scheduling priority for the spawned ffmpeg child.  See
    /// `infra::process::priority`.
    pub priority: ProcessPriority,
}

/// Encoder name that comes back from `ffmpeg -encoders`.  We parse
/// stdout into one of these per available encoder so the detection
/// service can do membership checks without re-parsing strings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncoderListing {
    pub name: String,
}

/// Process priority hint passed into the re-encode call (ADR-0029).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessPriority {
    /// Default OS scheduling.  Used by the existing remux/thumbnail
    /// paths where priority lowering would slow latency-sensitive
    /// post-processing of a finished download.
    Normal,
    /// Lowest reasonable scheduling priority — `nice 19` on Unix,
    /// `BELOW_NORMAL_PRIORITY_CLASS` on Windows.  Used by the
    /// background re-encode pass.
    Background,
}

#[async_trait]
pub trait Ffmpeg: Send + Sync + std::fmt::Debug {
    async fn version(&self) -> Result<FfmpegVersion, AppError>;
    async fn remux_to_mp4(&self, spec: &RemuxSpec) -> Result<(), AppError>;
    async fn extract_thumbnail(&self, spec: &ThumbnailSpec) -> Result<(), AppError>;
    /// Extract all frames in a single ffmpeg process. The default impl
    /// loops over `extract_thumbnail`; real implementations may batch
    /// for speed. Returning an error after some frames wrote means the
    /// caller should delete the partial set.
    async fn extract_preview_frames(&self, spec: &PreviewFramesSpec) -> Result<(), AppError> {
        for (percent, dest) in &spec.frames {
            self.extract_thumbnail(&ThumbnailSpec {
                source: spec.source.clone(),
                destination: dest.clone(),
                duration_seconds: spec.duration_seconds,
                percent: *percent,
            })
            .await?;
        }
        Ok(())
    }

    /// List the encoders the bundled ffmpeg knows about.  Output is
    /// parsed from `ffmpeg -encoders` and pre-filtered to the names
    /// the detection layer cares about (`hevc_*`, `h264_*`,
    /// `libx265`, `libx264`).
    async fn list_encoders(&self) -> Result<Vec<EncoderListing>, AppError>;

    /// Run a 2-second synthetic test encode using the supplied
    /// `-c:v` argument.  Used by the encoder-detection pass to
    /// confirm the encoder actually initialises on this hardware
    /// (e.g. NVENC may be advertised on an Ubuntu image without an
    /// NVIDIA card).  Returns `Ok(())` on a successful exit;
    /// `Err(_)` for any non-zero exit or timeout.
    async fn test_encoder(&self, video_encoder_arg: &str) -> Result<(), AppError>;

    /// Re-encode a source file with the supplied encoder.  Audio is
    /// always passed through unchanged via `-c:a copy`.  Honours the
    /// supplied [`ProcessPriority`] for the spawned child.
    async fn reencode(&self, spec: &ReencodeSpec) -> Result<(), AppError>;
}

pub type SharedFfmpeg = Arc<dyn Ffmpeg>;

/// Compute the seek argument `ffmpeg -ss` should receive given a
/// duration + percentage. Pure — exposed for tests.
pub fn seek_seconds(duration_seconds: i64, percent: f64) -> f64 {
    let dur = duration_seconds.max(0) as f64;
    let pct = percent.clamp(0.0, 100.0);
    dur * (pct / 100.0)
}

/// Whether a VOD's extension means it already is an mp4 and no remux
/// step is required.
pub fn already_mp4(p: &Path) -> bool {
    p.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("mp4"))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn seek_is_proportional() {
        assert!((seek_seconds(3600, 50.0) - 1800.0).abs() < 1e-6);
        assert!((seek_seconds(3600, 0.0) - 0.0).abs() < 1e-6);
        assert!((seek_seconds(3600, 100.0) - 3600.0).abs() < 1e-6);
    }

    #[test]
    fn seek_clamps_out_of_range() {
        assert!((seek_seconds(3600, -10.0) - 0.0).abs() < 1e-6);
        assert!((seek_seconds(3600, 500.0) - 3600.0).abs() < 1e-6);
    }

    #[test]
    fn seek_zero_duration_is_zero() {
        assert!((seek_seconds(0, 50.0) - 0.0).abs() < 1e-6);
    }

    #[test]
    fn already_mp4_recognises_case_variants() {
        assert!(already_mp4(Path::new("foo.mp4")));
        assert!(already_mp4(Path::new("foo.MP4")));
        assert!(!already_mp4(Path::new("foo.ts")));
        assert!(!already_mp4(Path::new("foo.mkv")));
        assert!(!already_mp4(Path::new("foo")));
    }
}
