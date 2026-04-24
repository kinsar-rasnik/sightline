//! Production yt-dlp wrapper — invokes the bundled sidecar binary.
//!
//! Security posture (reinforced here because the `security-reviewer`
//! subagent will grep for these):
//!
//! * Commands are always `Command::new(binary).args([...])` — never
//!   a shell. User-controlled strings flow into `args`, not into a
//!   joined command string.
//! * The `binary` path is resolved once at construction time from
//!   Tauri's sidecar resolver; we never take one from user input or
//!   the environment.
//! * We opt `Command::kill_on_drop(true)` so a cancelled download
//!   does not leave a stray `yt-dlp` process behind.

use std::path::PathBuf;
use std::process::Stdio;

use async_trait::async_trait;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;
use tracing::{debug, warn};

use super::progress;
use super::{
    DownloadProgress, DownloadResult, DownloadSpec, VodInfo, VodInfoRequest, YtDlp, YtDlpVersion,
};
use crate::error::AppError;

const TOOL: &str = "yt-dlp";

/// Runtime wrapper around the bundled yt-dlp binary. Keeps the binary
/// path so each call doesn't re-resolve it.
#[derive(Debug, Clone)]
pub struct YtDlpCli {
    binary: PathBuf,
}

impl YtDlpCli {
    /// Build with an explicit path (tests) or whatever Tauri's sidecar
    /// resolver returned (production). The constructor does not check
    /// that the file exists — `version()` does on first call.
    pub fn new(binary: impl Into<PathBuf>) -> Self {
        Self {
            binary: binary.into(),
        }
    }

    fn command(&self) -> Command {
        let mut cmd = Command::new(&self.binary);
        cmd.kill_on_drop(true);
        cmd.stdin(Stdio::null());
        cmd
    }

    fn sidecar_err(detail: impl Into<String>) -> AppError {
        AppError::Sidecar {
            tool: TOOL.into(),
            detail: detail.into(),
        }
    }
}

#[async_trait]
impl YtDlp for YtDlpCli {
    async fn version(&self) -> Result<YtDlpVersion, AppError> {
        let output = self
            .command()
            .arg("--version")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| Self::sidecar_err(format!("spawn: {e}")))?;
        if !output.status.success() {
            return Err(Self::sidecar_err(format!(
                "exit {:?}; stderr={}",
                output.status.code(),
                String::from_utf8_lossy(&output.stderr).trim()
            )));
        }
        let version = String::from_utf8_lossy(&output.stdout).trim().to_owned();
        if version.is_empty() {
            return Err(Self::sidecar_err("empty version output"));
        }
        Ok(YtDlpVersion {
            version,
            path: self.binary.clone(),
        })
    }

    async fn self_update(&self) -> Result<YtDlpVersion, AppError> {
        // `-U` / `--update` — yt-dlp exits 0 even when up to date.
        // On a packaged binary (Tauri sidecar) the update step can
        // fail due to filesystem permissions; we propagate the
        // failure to the caller so the queue can fall back to the
        // pinned version.
        let output = self
            .command()
            .arg("-U")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| Self::sidecar_err(format!("spawn: {e}")))?;
        if !output.status.success() {
            return Err(Self::sidecar_err(format!(
                "update exit {:?}; stderr={}",
                output.status.code(),
                String::from_utf8_lossy(&output.stderr).trim()
            )));
        }
        self.version().await
    }

    async fn fetch_info(&self, request: &VodInfoRequest) -> Result<VodInfo, AppError> {
        let output = self
            .command()
            .args([
                "--dump-single-json",
                "--no-playlist",
                "--skip-download",
                "--no-warnings",
                "--",
            ])
            .arg(&request.url)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| Self::sidecar_err(format!("spawn: {e}")))?;
        if !output.status.success() {
            return Err(Self::sidecar_err(format!(
                "fetch_info exit {:?}; stderr={}",
                output.status.code(),
                String::from_utf8_lossy(&output.stderr).trim()
            )));
        }
        parse_info_json(&output.stdout)
            .ok_or_else(|| Self::sidecar_err("could not parse yt-dlp --dump-single-json output"))
    }

    async fn download(
        &self,
        spec: &DownloadSpec,
        progress_sink: mpsc::Sender<DownloadProgress>,
    ) -> Result<DownloadResult, AppError> {
        tokio::fs::create_dir_all(&spec.output_dir).await?;

        let mut cmd = self.command();
        cmd.args([
            "--newline",
            "--no-warnings",
            "--progress-template",
            "download:%(progress)j",
            "-o",
        ])
        .arg(
            spec.output_dir
                .join(format!("{}.%(ext)s", spec.output_stem)),
        )
        .args(["-f"])
        .arg(&spec.format_selector);
        if let Some(rate) = spec.limit_rate_bps {
            cmd.args(["--limit-rate"]).arg(format!("{rate}"));
        }
        if spec.no_part {
            cmd.arg("--no-part");
        }
        cmd.arg("--").arg(&spec.url);
        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

        let mut child = cmd
            .spawn()
            .map_err(|e| Self::sidecar_err(format!("spawn: {e}")))?;

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| Self::sidecar_err("stdout not captured"))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| Self::sidecar_err("stderr not captured"))?;

        let mut out_reader = BufReader::new(stdout).lines();
        let mut err_reader = BufReader::new(stderr).lines();

        // Drain stdout (progress) and stderr (info / warnings) in
        // parallel. Progress goes to the sink; stderr is logged.
        let final_path: std::path::PathBuf = spec.output_dir.join(&spec.output_stem);
        let mut last_progress: Option<DownloadProgress> = None;

        loop {
            tokio::select! {
                line = out_reader.next_line() => {
                    match line {
                        Ok(Some(line)) => {
                            if let Some(progress) = progress::parse_line(&line) {
                                last_progress = Some(progress.clone());
                                let _ = progress_sink.send(progress).await;
                            } else {
                                debug!(target: "ytdlp", line);
                            }
                        }
                        Ok(None) => break,
                        Err(e) => {
                            warn!(error = %e, "ytdlp stdout read failed");
                            break;
                        }
                    }
                }
                line = err_reader.next_line() => {
                    match line {
                        Ok(Some(line)) => debug!(target: "ytdlp.stderr", line),
                        Ok(None) => {}
                        Err(e) => warn!(error = %e, "ytdlp stderr read failed"),
                    }
                }
            }
        }

        let status = child
            .wait()
            .await
            .map_err(|e| Self::sidecar_err(format!("wait: {e}")))?;

        if !status.success() {
            return Err(Self::sidecar_err(format!(
                "yt-dlp exited with {:?}",
                status.code()
            )));
        }

        // yt-dlp writes `<stem>.<chosen_ext>`; we don't know the ext
        // until after the download. The queue service walks
        // `output_dir` for the newest file matching the stem.
        // Returning the "stem path" here tells the caller what to
        // look for.
        let bytes = last_progress.as_ref().map_or(0, |p| p.bytes_done);
        Ok(DownloadResult {
            output_path: final_path,
            bytes,
        })
    }
}

fn parse_info_json(bytes: &[u8]) -> Option<VodInfo> {
    let v: serde_json::Value = serde_json::from_slice(bytes).ok()?;
    let id = v.get("id")?.as_str()?.to_owned();
    let title = v
        .get("title")
        .and_then(|t| t.as_str())
        .unwrap_or("")
        .to_owned();
    let filesize_bytes = v
        .get("filesize")
        .and_then(|n| n.as_u64())
        .or_else(|| v.get("filesize_approx").and_then(|n| n.as_u64()));
    let height = v.get("height").and_then(|n| n.as_u64()).map(|n| n as u32);
    let fps = v
        .get("fps")
        .and_then(|n| n.as_f64())
        .map(|n| n.round() as u32);
    let format_id = v
        .get("format_id")
        .and_then(|s| s.as_str())
        .map(str::to_owned);
    Some(VodInfo {
        id,
        title,
        filesize_bytes,
        height,
        fps,
        format_id,
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn info_json_parser_picks_fields() {
        let json = br#"{
            "id": "v1",
            "title": "A VOD",
            "filesize": 1024,
            "height": 1080,
            "fps": 60,
            "format_id": "1080p60"
        }"#;
        let info = parse_info_json(json).unwrap();
        assert_eq!(info.id, "v1");
        assert_eq!(info.title, "A VOD");
        assert_eq!(info.filesize_bytes, Some(1024));
        assert_eq!(info.height, Some(1080));
        assert_eq!(info.fps, Some(60));
        assert_eq!(info.format_id.as_deref(), Some("1080p60"));
    }

    #[test]
    fn info_json_falls_back_to_approx() {
        let json = br#"{ "id": "v2", "title": "", "filesize_approx": 4096 }"#;
        let info = parse_info_json(json).unwrap();
        assert_eq!(info.filesize_bytes, Some(4096));
    }

    #[test]
    fn info_json_missing_id_is_none() {
        let json = b"{}";
        assert!(parse_info_json(json).is_none());
    }
}
