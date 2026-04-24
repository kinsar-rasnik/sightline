//! Scripted ffmpeg fake. Writes empty destination files so the queue
//! service's post-processing steps can stat them, but does no actual
//! encoding.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;

use super::{Ffmpeg, FfmpegVersion, RemuxSpec, ThumbnailSpec};
use crate::error::AppError;

#[derive(Debug, Default, Clone)]
pub struct FfmpegScript {
    pub version: Option<String>,
    pub version_err: Option<String>,
    pub remux_err: Option<String>,
    pub thumbnail_err: Option<String>,
}

#[derive(Debug, Default, Clone)]
pub struct FfmpegCallLog {
    pub versions: u32,
    pub remuxes: Vec<RemuxSpec>,
    pub thumbnails: Vec<ThumbnailSpec>,
}

#[derive(Debug, Default, Clone)]
pub struct FfmpegFake {
    script: Arc<Mutex<FfmpegScript>>,
    calls: Arc<Mutex<FfmpegCallLog>>,
}

impl FfmpegFake {
    pub fn new(script: FfmpegScript) -> Self {
        Self {
            script: Arc::new(Mutex::new(script)),
            calls: Arc::new(Mutex::new(FfmpegCallLog::default())),
        }
    }

    pub fn calls(&self) -> FfmpegCallLog {
        #[allow(clippy::unwrap_used)]
        self.calls.lock().unwrap().clone()
    }
}

#[async_trait]
impl Ffmpeg for FfmpegFake {
    async fn version(&self) -> Result<FfmpegVersion, AppError> {
        let s = {
            #[allow(clippy::unwrap_used)]
            {
                self.calls.lock().unwrap().versions += 1;
            }
            #[allow(clippy::unwrap_used)]
            self.script.lock().unwrap().clone()
        };
        if let Some(err) = s.version_err {
            return Err(AppError::Sidecar {
                tool: "ffmpeg".into(),
                detail: err,
            });
        }
        Ok(FfmpegVersion {
            version: s.version.unwrap_or_else(|| "ffmpeg version 7.0".into()),
            path: PathBuf::from("/fake/ffmpeg"),
        })
    }

    async fn remux_to_mp4(&self, spec: &RemuxSpec) -> Result<(), AppError> {
        let s = {
            #[allow(clippy::unwrap_used)]
            self.calls.lock().unwrap().remuxes.push(spec.clone());
            #[allow(clippy::unwrap_used)]
            self.script.lock().unwrap().clone()
        };
        if let Some(err) = s.remux_err {
            return Err(AppError::Sidecar {
                tool: "ffmpeg".into(),
                detail: err,
            });
        }
        if let Some(parent) = spec.destination.parent() {
            tokio::fs::create_dir_all(parent).await.ok();
        }
        tokio::fs::write(&spec.destination, b"").await.ok();
        Ok(())
    }

    async fn extract_thumbnail(&self, spec: &ThumbnailSpec) -> Result<(), AppError> {
        let s = {
            #[allow(clippy::unwrap_used)]
            self.calls.lock().unwrap().thumbnails.push(spec.clone());
            #[allow(clippy::unwrap_used)]
            self.script.lock().unwrap().clone()
        };
        if let Some(err) = s.thumbnail_err {
            return Err(AppError::Sidecar {
                tool: "ffmpeg".into(),
                detail: err,
            });
        }
        if let Some(parent) = spec.destination.parent() {
            tokio::fs::create_dir_all(parent).await.ok();
        }
        tokio::fs::write(&spec.destination, b"\xff\xd8\xff\xe0")
            .await
            .ok();
        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::infra::ffmpeg::Ffmpeg;

    #[tokio::test]
    async fn default_fake_writes_destination_stub() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("in.ts");
        let dst = tmp.path().join("out.mp4");
        tokio::fs::write(&src, b"source").await.unwrap();
        let fake = FfmpegFake::new(FfmpegScript::default());
        fake.remux_to_mp4(&RemuxSpec {
            source: src,
            destination: dst.clone(),
        })
        .await
        .unwrap();
        assert!(dst.exists());
        assert_eq!(fake.calls().remuxes.len(), 1);
    }

    #[tokio::test]
    async fn error_scripts_surface() {
        let fake = FfmpegFake::new(FfmpegScript {
            thumbnail_err: Some("oh no".into()),
            ..Default::default()
        });
        let tmp = tempfile::tempdir().unwrap();
        let err = fake
            .extract_thumbnail(&ThumbnailSpec {
                source: tmp.path().join("in.mp4"),
                destination: tmp.path().join("out.jpg"),
                duration_seconds: 100,
                percent: 10.0,
            })
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::Sidecar { .. }));
    }
}
