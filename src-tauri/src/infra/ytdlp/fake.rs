//! Scripted yt-dlp wrapper for deterministic tests.
//!
//! Behaviour is controlled by a [`FakeScript`] the test builds before
//! handing the wrapper to the queue service. Every call records its
//! inputs so assertions can check we passed the right URL / stem /
//! rate / etc.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use tokio::sync::mpsc;

use super::{
    DownloadProgress, DownloadResult, DownloadSpec, VodInfo, VodInfoRequest, YtDlp, YtDlpVersion,
};
use crate::error::AppError;

/// What the fake should do when methods fire. Every field is optional;
/// unset == "return a bland success".
#[derive(Debug, Default, Clone)]
pub struct FakeScript {
    /// Value returned from `version()`; defaults to `"2026.04.24"`.
    pub version: Option<String>,
    /// Error `version()` returns (e.g. to simulate missing binary).
    pub version_err: Option<String>,
    /// Value returned from `fetch_info`. Defaults to `default_info()`.
    pub info: Option<VodInfo>,
    /// Error `fetch_info` returns.
    pub info_err: Option<String>,
    /// Sequence of progress events the fake emits before completing.
    /// An empty sequence emits one 100% event so the sink sees
    /// exactly one message.
    pub progress_ticks: Vec<DownloadProgress>,
    /// Error `download` returns instead of producing a result.
    pub download_err: Option<String>,
}

fn default_info() -> VodInfo {
    VodInfo {
        id: "v1".into(),
        title: "fake vod".into(),
        filesize_bytes: Some(1024 * 1024),
        height: Some(1080),
        fps: Some(60),
        format_id: Some("1080p60".into()),
    }
}

/// Records every call the fake receives, in order. The queue's
/// integration tests assert on these records.
#[derive(Debug, Default, Clone)]
pub struct CallLog {
    pub fetch_info: Vec<VodInfoRequest>,
    pub downloads: Vec<DownloadSpec>,
    pub versions: u32,
    pub self_updates: u32,
}

#[derive(Debug, Default, Clone)]
pub struct YtDlpFake {
    script: Arc<Mutex<FakeScript>>,
    calls: Arc<Mutex<CallLog>>,
}

impl YtDlpFake {
    pub fn new(script: FakeScript) -> Self {
        Self {
            script: Arc::new(Mutex::new(script)),
            calls: Arc::new(Mutex::new(CallLog::default())),
        }
    }

    pub fn calls(&self) -> CallLog {
        #[allow(clippy::unwrap_used)]
        self.calls.lock().unwrap().clone()
    }
}

#[async_trait]
impl YtDlp for YtDlpFake {
    async fn version(&self) -> Result<YtDlpVersion, AppError> {
        let s = {
            #[allow(clippy::unwrap_used)]
            let mut c = self.calls.lock().unwrap();
            c.versions += 1;
            #[allow(clippy::unwrap_used)]
            self.script.lock().unwrap().clone()
        };
        if let Some(err) = s.version_err {
            return Err(AppError::Sidecar {
                tool: "yt-dlp".into(),
                detail: err,
            });
        }
        Ok(YtDlpVersion {
            version: s.version.unwrap_or_else(|| "2026.04.24".into()),
            path: PathBuf::from("/fake/yt-dlp"),
        })
    }

    async fn self_update(&self) -> Result<YtDlpVersion, AppError> {
        {
            #[allow(clippy::unwrap_used)]
            let mut c = self.calls.lock().unwrap();
            c.self_updates += 1;
        }
        self.version().await
    }

    async fn fetch_info(&self, request: &VodInfoRequest) -> Result<VodInfo, AppError> {
        let s = {
            #[allow(clippy::unwrap_used)]
            let mut c = self.calls.lock().unwrap();
            c.fetch_info.push(request.clone());
            #[allow(clippy::unwrap_used)]
            self.script.lock().unwrap().clone()
        };
        if let Some(err) = s.info_err {
            return Err(AppError::Sidecar {
                tool: "yt-dlp".into(),
                detail: err,
            });
        }
        Ok(s.info.unwrap_or_else(default_info))
    }

    async fn download(
        &self,
        spec: &DownloadSpec,
        progress_sink: mpsc::Sender<DownloadProgress>,
    ) -> Result<DownloadResult, AppError> {
        let s = {
            #[allow(clippy::unwrap_used)]
            let mut c = self.calls.lock().unwrap();
            c.downloads.push(spec.clone());
            #[allow(clippy::unwrap_used)]
            self.script.lock().unwrap().clone()
        };
        if let Some(err) = s.download_err {
            return Err(AppError::Sidecar {
                tool: "yt-dlp".into(),
                detail: err,
            });
        }

        let ticks = if s.progress_ticks.is_empty() {
            vec![DownloadProgress {
                progress: Some(1.0),
                bytes_done: 1024,
                bytes_total: Some(1024),
                speed_bps: Some(1024),
                eta_seconds: Some(0),
            }]
        } else {
            s.progress_ticks
        };

        for tick in &ticks {
            // Best-effort send; if the caller dropped the receiver we
            // still want to complete the download (mirrors the real
            // wrapper's behaviour).
            let _ = progress_sink.send(tick.clone()).await;
        }

        let final_bytes = ticks.last().map(|t| t.bytes_done).unwrap_or(0);
        // Manifest the "output file" on disk so the caller's post-
        // download step can actually move it. Tests using a tempdir
        // pass that as `spec.output_dir`.
        let output_path = spec.output_dir.join(format!("{}.mp4", spec.output_stem));
        tokio::fs::create_dir_all(&spec.output_dir).await.ok();
        tokio::fs::write(&output_path, vec![0u8; 16]).await.ok();
        Ok(DownloadResult {
            output_path,
            bytes: final_bytes,
        })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::path::Path;

    #[tokio::test]
    async fn default_script_yields_default_version_and_info() {
        let fake = YtDlpFake::new(FakeScript::default());
        let v = fake.version().await.unwrap();
        assert_eq!(v.version, "2026.04.24");
        let info = fake
            .fetch_info(&VodInfoRequest {
                url: "https://twitch.tv/videos/v1".into(),
            })
            .await
            .unwrap();
        assert_eq!(info.id, "v1");
    }

    #[tokio::test]
    async fn download_emits_progress_and_writes_stub_file() {
        let tmp = tempfile::tempdir().unwrap();
        let fake = YtDlpFake::new(FakeScript {
            progress_ticks: vec![
                DownloadProgress {
                    progress: Some(0.5),
                    bytes_done: 512,
                    bytes_total: Some(1024),
                    speed_bps: Some(100),
                    eta_seconds: Some(5),
                },
                DownloadProgress {
                    progress: Some(1.0),
                    bytes_done: 1024,
                    bytes_total: Some(1024),
                    speed_bps: Some(100),
                    eta_seconds: Some(0),
                },
            ],
            ..Default::default()
        });
        let (tx, mut rx) = mpsc::channel(4);
        let result = fake
            .download(
                &DownloadSpec {
                    url: "https://twitch.tv/videos/v1".into(),
                    output_dir: tmp.path().to_owned(),
                    output_stem: "v1".into(),
                    format_selector: "bestvideo+bestaudio/best".into(),
                    limit_rate_bps: None,
                    no_part: false,
                },
                tx,
            )
            .await
            .unwrap();
        assert_eq!(result.bytes, 1024);
        assert!(Path::new(&result.output_path).exists());
        let mut ticks = Vec::new();
        while let Some(t) = rx.recv().await {
            ticks.push(t);
        }
        assert_eq!(ticks.len(), 2);
    }

    #[tokio::test]
    async fn scripted_error_surfaces_as_sidecar() {
        let fake = YtDlpFake::new(FakeScript {
            info_err: Some("not found".into()),
            ..Default::default()
        });
        let err = fake
            .fetch_info(&VodInfoRequest { url: "x".into() })
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::Sidecar { .. }));
    }

    #[tokio::test]
    async fn call_log_records_inputs() {
        let fake = YtDlpFake::new(FakeScript::default());
        let _ = fake.version().await.unwrap();
        let _ = fake.self_update().await.unwrap();
        let _ = fake
            .fetch_info(&VodInfoRequest {
                url: "https://twitch.tv/videos/v1".into(),
            })
            .await
            .unwrap();
        let calls = fake.calls();
        assert_eq!(calls.versions, 2);
        assert_eq!(calls.self_updates, 1);
        assert_eq!(calls.fetch_info.len(), 1);
    }
}
