//! yt-dlp sidecar wrapper.
//!
//! Two implementations behind the [`YtDlp`] trait:
//!
//! * [`cli::YtDlpCli`] — invokes the bundled sidecar binary via
//!   `tokio::process::Command`. Used at runtime.
//! * [`fake::YtDlpFake`] — scripted responses for deterministic tests.
//!   Lives behind the crate-wide `test-support` feature.
//!
//! The trait is async-trait-based (matching the existing Helix / GQL
//! clients). Progress is streamed through an
//! [`mpsc::Sender<DownloadProgress>`] the caller passes in; the final
//! [`DownloadResult`] returns once yt-dlp exits.
//!
//! Security posture: argv-only invocation, never a shell. User-
//! supplied URLs are validated (`https://www.twitch.tv/videos/<id>`)
//! at the service layer before being handed to the wrapper.

use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use crate::error::AppError;

pub mod cli;
pub mod progress;

#[cfg(any(test, feature = "test-support"))]
pub mod fake;

/// Installed sidecar version + self-check outcome. Returned from
/// [`YtDlp::version`] at startup and every time the user triggers an
/// auto-update.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct YtDlpVersion {
    /// yt-dlp's raw version string (e.g. `"2026.04.01"`).
    pub version: String,
    /// Absolute path of the binary that answered `--version`.
    pub path: PathBuf,
}

/// What we ask the sidecar to do when `fetch_info` runs. The caller
/// provides a VOD URL; we ask yt-dlp for a JSON info document (no
/// download).
#[derive(Debug, Clone)]
pub struct VodInfoRequest {
    pub url: String,
}

/// Subset of the yt-dlp info JSON we actually care about. yt-dlp
/// returns ~100 keys; we keep only what the queue and the disk-space
/// preflight need.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VodInfo {
    pub id: String,
    pub title: String,
    /// From `filesize` if present, otherwise `filesize_approx`. May be
    /// `None` on live VODs where yt-dlp can't estimate ahead of time.
    pub filesize_bytes: Option<u64>,
    /// Source height in pixels (e.g. 1080).
    pub height: Option<u32>,
    /// Source fps (e.g. 60).
    pub fps: Option<u32>,
    /// Raw yt-dlp `format_id` that would be chosen for the current
    /// default selector. Mirrored into `downloads.quality_resolved`.
    pub format_id: Option<String>,
}

/// Spec a concrete download. All paths are absolute.
#[derive(Debug, Clone)]
pub struct DownloadSpec {
    pub url: String,
    pub output_dir: PathBuf,
    /// Filename stem yt-dlp writes to; the `%(ext)s` suffix is added
    /// internally by the wrapper.
    pub output_stem: String,
    /// `-f` format selector string from `QualityPreset::format_selector`.
    pub format_selector: String,
    /// Optional per-worker rate limit in bytes/sec.
    pub limit_rate_bps: Option<u64>,
    /// Whether to pass `--no-part` — set true on network filesystems
    /// where `.part` rename semantics are unreliable (Proton Drive,
    /// Dropbox). The queue decides based on the staging path.
    pub no_part: bool,
}

/// What comes back when yt-dlp finishes successfully.
#[derive(Debug, Clone)]
pub struct DownloadResult {
    /// Absolute path of the downloaded file yt-dlp wrote.
    pub output_path: PathBuf,
    /// yt-dlp-reported final size in bytes.
    pub bytes: u64,
}

/// Progress event parsed from yt-dlp's `--progress-template` output.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DownloadProgress {
    /// 0.0 .. 1.0. `None` if yt-dlp hasn't reported a total yet
    /// (e.g. very start of the download, or live-stream archive
    /// fetch).
    pub progress: Option<f64>,
    pub bytes_done: u64,
    pub bytes_total: Option<u64>,
    pub speed_bps: Option<u64>,
    pub eta_seconds: Option<u64>,
}

/// Runtime-swappable yt-dlp abstraction. Implementations must be
/// `Send + Sync` so the queue service can share a single instance
/// across worker tasks via `Arc<dyn YtDlp>`.
#[async_trait]
pub trait YtDlp: Send + Sync {
    /// Return the version of the binary under management. Fails if
    /// the binary isn't on disk, isn't executable, or doesn't answer
    /// `--version`.
    async fn version(&self) -> Result<YtDlpVersion, AppError>;

    /// Optionally self-update the binary. Implementations may no-op
    /// when the update mechanism isn't available on the current
    /// platform.
    async fn self_update(&self) -> Result<YtDlpVersion, AppError>;

    /// Fetch the JSON info document for a VOD without downloading.
    /// Used by the disk-space preflight + the quality resolver.
    async fn fetch_info(&self, request: &VodInfoRequest) -> Result<VodInfo, AppError>;

    /// Start a download, streaming progress to `progress_sink`. Returns
    /// on exit (success or error). Cancellation is cooperative —
    /// dropping the future kills the child process via the Tokio
    /// runtime's `Kill on drop` behaviour, which we opt into in the
    /// CLI impl.
    async fn download(
        &self,
        spec: &DownloadSpec,
        progress_sink: mpsc::Sender<DownloadProgress>,
    ) -> Result<DownloadResult, AppError>;
}

/// Shared alias so callers aren't sprinkled with `Arc<dyn YtDlp>`.
pub type SharedYtDlp = Arc<dyn YtDlp>;

/// Tiny helper for the queue service — builds the full output filename
/// yt-dlp will create (`<stem>.<ext>`) given the chosen extension. The
/// queue uses this to cleanup after a cancelled download.
pub fn output_path_guess(spec: &DownloadSpec, ext: &str) -> PathBuf {
    let filename = format!("{stem}.{ext}", stem = spec.output_stem);
    spec.output_dir.join(filename)
}

/// Helper for tests + the service layer — pick the best size estimate
/// yt-dlp gave us, falling back to `filesize_approx`.
pub fn size_estimate(info: &VodInfo) -> Option<u64> {
    info.filesize_bytes
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use std::path::Path;

    use super::*;

    #[test]
    fn output_path_guess_joins_stem_and_ext() {
        let spec = DownloadSpec {
            url: "https://twitch.tv/videos/v1".into(),
            output_dir: Path::new("/tmp/staging").to_owned(),
            output_stem: "v1".into(),
            format_selector: "bestvideo+bestaudio/best".into(),
            limit_rate_bps: None,
            no_part: false,
        };
        assert_eq!(
            output_path_guess(&spec, "mp4"),
            PathBuf::from("/tmp/staging/v1.mp4")
        );
    }

    #[test]
    fn size_estimate_prefers_filesize() {
        let info = VodInfo {
            id: "v1".into(),
            title: "t".into(),
            filesize_bytes: Some(123),
            height: Some(1080),
            fps: Some(60),
            format_id: None,
        };
        assert_eq!(size_estimate(&info), Some(123));
        let without = VodInfo {
            filesize_bytes: None,
            ..info
        };
        assert_eq!(size_estimate(&without), None);
    }
}
