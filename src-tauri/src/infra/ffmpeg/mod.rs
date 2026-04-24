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

#[async_trait]
pub trait Ffmpeg: Send + Sync {
    async fn version(&self) -> Result<FfmpegVersion, AppError>;
    async fn remux_to_mp4(&self, spec: &RemuxSpec) -> Result<(), AppError>;
    async fn extract_thumbnail(&self, spec: &ThumbnailSpec) -> Result<(), AppError>;
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
