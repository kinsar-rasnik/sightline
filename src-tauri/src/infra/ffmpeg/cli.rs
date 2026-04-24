//! Production ffmpeg wrapper.

use std::path::PathBuf;
use std::process::Stdio;

use async_trait::async_trait;
use tokio::process::Command;
use tracing::debug;

use super::{Ffmpeg, FfmpegVersion, RemuxSpec, ThumbnailSpec, already_mp4, seek_seconds};
use crate::error::AppError;

const TOOL: &str = "ffmpeg";

#[derive(Debug, Clone)]
pub struct FfmpegCli {
    binary: PathBuf,
}

impl FfmpegCli {
    pub fn new(binary: impl Into<PathBuf>) -> Self {
        Self {
            binary: binary.into(),
        }
    }

    fn command(&self) -> Command {
        let mut cmd = Command::new(&self.binary);
        cmd.kill_on_drop(true);
        cmd.stdin(Stdio::null());
        // ffmpeg is noisy by default; silence the banner + bring
        // errors into stderr.
        cmd.args(["-hide_banner", "-loglevel", "error", "-nostats"]);
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
impl Ffmpeg for FfmpegCli {
    async fn version(&self) -> Result<FfmpegVersion, AppError> {
        let mut cmd = Command::new(&self.binary);
        cmd.kill_on_drop(true).stdin(Stdio::null());
        cmd.args(["-version"]);
        let output = cmd
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| Self::sidecar_err(format!("spawn: {e}")))?;
        if !output.status.success() {
            return Err(Self::sidecar_err(format!(
                "exit {:?}",
                output.status.code()
            )));
        }
        let version_line = String::from_utf8_lossy(&output.stdout)
            .lines()
            .next()
            .map(|s| s.trim().to_owned())
            .unwrap_or_default();
        if version_line.is_empty() {
            return Err(Self::sidecar_err("empty version output"));
        }
        Ok(FfmpegVersion {
            version: version_line,
            path: self.binary.clone(),
        })
    }

    async fn remux_to_mp4(&self, spec: &RemuxSpec) -> Result<(), AppError> {
        if already_mp4(&spec.source) && spec.source == spec.destination {
            debug!("remux skipped: source already mp4 at destination");
            return Ok(());
        }
        let mut cmd = self.command();
        cmd.args(["-y", "-i"])
            .arg(&spec.source)
            .args(["-c", "copy", "-movflags", "+faststart"])
            .arg(&spec.destination);
        let status = cmd
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .status()
            .await
            .map_err(|e| Self::sidecar_err(format!("spawn: {e}")))?;
        if !status.success() {
            return Err(Self::sidecar_err(format!("remux exit {:?}", status.code())));
        }
        Ok(())
    }

    async fn extract_thumbnail(&self, spec: &ThumbnailSpec) -> Result<(), AppError> {
        let seek = seek_seconds(spec.duration_seconds, spec.percent);
        let mut cmd = self.command();
        cmd.args(["-y", "-ss"])
            .arg(format!("{seek:.2}"))
            .args(["-i"])
            .arg(&spec.source)
            .args(["-frames:v", "1", "-q:v", "4"])
            .arg(&spec.destination);
        let status = cmd
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .status()
            .await
            .map_err(|e| Self::sidecar_err(format!("spawn: {e}")))?;
        if !status.success() {
            return Err(Self::sidecar_err(format!(
                "thumbnail exit {:?}",
                status.code()
            )));
        }
        Ok(())
    }
}
