//! Production ffmpeg wrapper.

use std::path::PathBuf;
use std::process::Stdio;

use async_trait::async_trait;
use tokio::process::Command;
use tracing::debug;

use super::{
    EncoderListing, Ffmpeg, FfmpegVersion, ReencodeSpec, RemuxSpec, ThumbnailSpec, already_mp4,
    seek_seconds,
};
use crate::error::AppError;
use crate::infra::process::priority::apply_priority;

const TOOL: &str = "ffmpeg";

/// Encoder names the detection layer cares about.  The `ffmpeg
/// -encoders` output contains hundreds of entries — we only want the
/// hardware/software encoders that actually drive Phase 8's pipeline.
const TRACKED_ENCODERS: &[&str] = &[
    "hevc_videotoolbox",
    "h264_videotoolbox",
    "hevc_nvenc",
    "h264_nvenc",
    "hevc_amf",
    "h264_amf",
    "hevc_qsv",
    "h264_qsv",
    "hevc_vaapi",
    "h264_vaapi",
    "libx265",
    "libx264",
];

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

    async fn list_encoders(&self) -> Result<Vec<EncoderListing>, AppError> {
        let mut cmd = Command::new(&self.binary);
        cmd.kill_on_drop(true).stdin(Stdio::null());
        cmd.args(["-hide_banner", "-encoders"]);
        let output = cmd
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output()
            .await
            .map_err(|e| Self::sidecar_err(format!("spawn: {e}")))?;
        if !output.status.success() {
            return Err(Self::sidecar_err(format!(
                "encoders exit {:?}",
                output.status.code()
            )));
        }
        let text = String::from_utf8_lossy(&output.stdout);
        Ok(parse_encoders_output(&text))
    }

    async fn test_encoder(&self, video_encoder_arg: &str) -> Result<(), AppError> {
        // 1-second synthetic colour-bar source piped to the encoder
        // and discarded.  `lavfi` + `testsrc` is the canonical
        // ffmpeg way to produce a synthetic input without touching
        // disk.  Output goes to /dev/null.
        let mut cmd = self.command();
        cmd.args([
            "-y",
            "-f",
            "lavfi",
            "-t",
            "1",
            "-i",
            "testsrc=size=256x144:rate=30",
            "-c:v",
            video_encoder_arg,
            "-f",
            "null",
            "-",
        ]);
        // Bound the test encode to 10 s wall-clock — a real encoder
        // takes well under 1 s; anything beyond 10 s indicates the
        // encoder is mis-configured (or the host is wedged).
        let result = tokio::time::timeout(std::time::Duration::from_secs(10), async {
            cmd.stdout(Stdio::null())
                .stderr(Stdio::piped())
                .status()
                .await
        })
        .await
        .map_err(|_| Self::sidecar_err(format!("test_encoder timeout for {video_encoder_arg}")))?;
        let status = result.map_err(|e| Self::sidecar_err(format!("spawn: {e}")))?;
        if !status.success() {
            return Err(Self::sidecar_err(format!(
                "test_encoder {video_encoder_arg} exit {:?}",
                status.code()
            )));
        }
        Ok(())
    }

    async fn reencode(&self, spec: &ReencodeSpec) -> Result<(), AppError> {
        let mut cmd = self.command();
        cmd.args(["-y", "-i"]).arg(&spec.source);

        // Build the video filter chain: width/height + fps if
        // requested.  We use `scale` rather than `scale_vt` /
        // `scale_npp` to avoid filter-name divergence between the
        // hardware encoders — the hwaccel-aware path is a v2.x
        // optimisation.  Software scale is fine for the bandwidth
        // numbers Phase 8 targets.
        let mut filters: Vec<String> = Vec::new();
        if let Some(h) = spec.max_height {
            filters.push(format!("scale=-2:'min({h},ih)'"));
        }
        if let Some(fps) = spec.max_fps {
            filters.push(format!("fps={fps}"));
        }
        if !filters.is_empty() {
            cmd.args(["-vf", &filters.join(",")]);
        }

        cmd.args([
            "-c:v",
            &spec.video_encoder_arg,
            // Audio passthrough — load-bearing invariant per
            // ADR-0028 §Audio policy.  Tested by
            // `services::reencode::tests::audio_passthrough_is_byte_exact`.
            "-c:a",
            "copy",
            "-movflags",
            "+faststart",
        ])
        .arg(&spec.destination);

        let mut child = cmd
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| Self::sidecar_err(format!("spawn: {e}")))?;

        // Apply scheduling priority post-spawn (Unix `nice`-style or
        // Windows `BELOW_NORMAL`).  Failure to lower priority is not
        // fatal — log and continue.  See ADR-0029 §Layer 1.
        if let Some(pid) = child.id()
            && let Err(e) = apply_priority(pid, spec.priority)
        {
            debug!(pid, error = %e, priority = ?spec.priority, "apply_priority failed");
        }

        let status = child
            .wait()
            .await
            .map_err(|e| Self::sidecar_err(format!("wait: {e}")))?;
        if !status.success() {
            return Err(Self::sidecar_err(format!(
                "reencode {} exit {:?}",
                spec.video_encoder_arg,
                status.code()
            )));
        }
        Ok(())
    }
}

/// Parse the lines of `ffmpeg -encoders` and return the tracked
/// encoders.  Pure — exposed for unit tests.
///
/// Output looks like:
///
/// ```text
/// Encoders:
///  V..... = Video
///  A..... = Audio
///  ...
///  ------
///  V..... libx264              libx264 H.264 / AVC ...
///  V..... libx265              libx265 HEVC ...
///  V..... hevc_videotoolbox    VideoToolbox HEVC encoder ...
/// ```
///
/// We skip the header (lines until we see the `------` divider),
/// then parse each subsequent line by whitespace-splitting and
/// keeping the second token (the encoder name).
pub fn parse_encoders_output(text: &str) -> Vec<EncoderListing> {
    let mut found = Vec::new();
    let mut past_header = false;
    for line in text.lines() {
        if !past_header {
            if line.trim_start().starts_with("------") {
                past_header = true;
            }
            continue;
        }
        let trimmed = line.trim_start();
        // The flag column is e.g. "V.....", followed by whitespace,
        // followed by the encoder name. Skip empty lines.
        if trimmed.is_empty() {
            continue;
        }
        let mut parts = trimmed.split_whitespace();
        // First token = flag column.  We don't use it; only V/A/S.
        let _ = parts.next();
        if let Some(name) = parts.next()
            && TRACKED_ENCODERS.contains(&name)
        {
            found.push(EncoderListing {
                name: name.to_owned(),
            });
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_encoders_output_finds_tracked_names() {
        let sample = "ffmpeg version n6.1 Copyright (c) ...\n\
            Encoders:\n\
             V..... = Video\n\
             A..... = Audio\n\
             ------\n\
             V..... libx264              libx264 H.264 / AVC encoder\n\
             V..... libx265              libx265 HEVC encoder\n\
             V..... hevc_videotoolbox    VideoToolbox HEVC encoder\n\
             V..... mjpeg                MJPEG encoder\n\
             A..... aac                  AAC (Advanced Audio Coding)\n";
        let parsed = parse_encoders_output(sample);
        let names: Vec<&str> = parsed.iter().map(|e| e.name.as_str()).collect();
        assert!(names.contains(&"libx264"));
        assert!(names.contains(&"libx265"));
        assert!(names.contains(&"hevc_videotoolbox"));
        assert!(!names.contains(&"mjpeg")); // not tracked
        assert!(!names.contains(&"aac")); // not tracked
    }

    #[test]
    fn parse_encoders_output_handles_missing_header() {
        // No `------` divider means we're not past the header,
        // so we collect nothing.
        let sample = "Encoders:\n V..... libx264 desc\n";
        assert!(parse_encoders_output(sample).is_empty());
    }

    #[test]
    fn parse_encoders_output_skips_unknown_encoders() {
        let sample = "Encoders:\n ------\n V..... unknown_codec desc\n V..... libx264 desc\n";
        let parsed = parse_encoders_output(sample);
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].name, "libx264");
    }
}
