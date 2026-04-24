//! Parse yt-dlp's `--progress-template` JSON output into
//! [`DownloadProgress`] events.
//!
//! We invoke yt-dlp with:
//!
//! ```text
//! --progress-template "download:%(progress)j"
//! --newline
//! ```
//!
//! `%(progress)j` expands to a JSON object per progress tick. Fields
//! we care about:
//!
//! * `downloaded_bytes` — running total
//! * `total_bytes` / `total_bytes_estimate` — whichever yt-dlp knows
//! * `speed` — bytes/sec (float)
//! * `eta` — seconds remaining (int)
//!
//! The parser is tolerant: any missing or malformed field falls back
//! to `None`; the download path never aborts because a progress line
//! couldn't be read.

use serde::Deserialize;

use super::DownloadProgress;

/// Line prefix the CLI prepends to progress JSON lines (the
/// `download:` token in the template). Anything without this prefix
/// is treated as normal stderr / info output.
pub const PROGRESS_PREFIX: &str = "download:";

/// Parse one stdout line. Returns `None` for lines that don't carry
/// progress JSON.
pub fn parse_line(line: &str) -> Option<DownloadProgress> {
    let rest = line.strip_prefix(PROGRESS_PREFIX)?;
    let raw: RawProgress = serde_json::from_str(rest.trim()).ok()?;
    Some(raw.into_typed())
}

#[derive(Debug, Deserialize)]
struct RawProgress {
    // yt-dlp reports several keys; any of them may be missing.
    #[serde(default)]
    downloaded_bytes: Option<f64>,
    #[serde(default)]
    total_bytes: Option<f64>,
    #[serde(default)]
    total_bytes_estimate: Option<f64>,
    #[serde(default)]
    speed: Option<f64>,
    #[serde(default)]
    eta: Option<f64>,
}

impl RawProgress {
    fn into_typed(self) -> DownloadProgress {
        let done = self.downloaded_bytes.unwrap_or(0.0).max(0.0) as u64;
        let total = self
            .total_bytes
            .or(self.total_bytes_estimate)
            .and_then(|n| if n >= 0.0 { Some(n as u64) } else { None });
        let speed = self
            .speed
            .and_then(|n| if n > 0.0 { Some(n as u64) } else { None });
        let eta = self
            .eta
            .and_then(|n| if n >= 0.0 { Some(n as u64) } else { None });
        let progress = match total {
            Some(t) if t > 0 => Some((done as f64 / t as f64).clamp(0.0, 1.0)),
            _ => None,
        };
        DownloadProgress {
            progress,
            bytes_done: done,
            bytes_total: total,
            speed_bps: speed,
            eta_seconds: eta,
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn parses_complete_line() {
        let line = r#"download:{"downloaded_bytes": 524288, "total_bytes": 1048576, "speed": 262144.0, "eta": 2}"#;
        let p = parse_line(line).unwrap();
        assert_eq!(p.bytes_done, 524288);
        assert_eq!(p.bytes_total, Some(1048576));
        assert_eq!(p.speed_bps, Some(262144));
        assert_eq!(p.eta_seconds, Some(2));
        assert_eq!(p.progress, Some(0.5));
    }

    #[test]
    fn tolerates_missing_total() {
        let line = r#"download:{"downloaded_bytes": 123, "speed": 100.0}"#;
        let p = parse_line(line).unwrap();
        assert_eq!(p.bytes_done, 123);
        assert_eq!(p.bytes_total, None);
        assert_eq!(p.progress, None);
    }

    #[test]
    fn falls_back_to_total_bytes_estimate() {
        let line = r#"download:{"downloaded_bytes": 100, "total_bytes_estimate": 500}"#;
        let p = parse_line(line).unwrap();
        assert_eq!(p.bytes_total, Some(500));
        assert_eq!(p.progress, Some(0.2));
    }

    #[test]
    fn ignores_non_progress_lines() {
        assert!(parse_line("[info] downloading format 1080p60").is_none());
        assert!(parse_line("").is_none());
        assert!(parse_line("download:not-json").is_none());
    }

    #[test]
    fn ignores_negative_or_missing_speed_and_eta() {
        let line =
            r#"download:{"downloaded_bytes": 10, "total_bytes": 100, "speed": -1.0, "eta": -1}"#;
        let p = parse_line(line).unwrap();
        assert_eq!(p.speed_bps, None);
        assert_eq!(p.eta_seconds, None);
    }

    #[test]
    fn progress_clamps_over_hundred_percent() {
        // yt-dlp sometimes reports downloaded_bytes > total_bytes when
        // muxing mid-stream. Keep the clamp.
        let line = r#"download:{"downloaded_bytes": 105, "total_bytes": 100}"#;
        let p = parse_line(line).unwrap();
        assert_eq!(p.progress, Some(1.0));
    }
}
