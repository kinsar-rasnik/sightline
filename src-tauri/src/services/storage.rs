//! Storage observation — reports on staging and library partitions to
//! the UI's Downloads & Storage settings panel. No writes; the queue
//! owns the disk-space preflight directly.

use std::path::PathBuf;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::error::AppError;
use crate::infra::fs::space::FreeSpaceProbe;
use crate::infra::fs::staging;

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct StagingInfo {
    pub path: String,
    pub free_bytes: i64,
    pub stale_file_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct LibraryInfo {
    pub path: Option<String>,
    pub free_bytes: Option<i64>,
    pub file_count: i64,
}

#[derive(Debug)]
pub struct StorageService {
    probe: Arc<dyn FreeSpaceProbe>,
}

impl StorageService {
    pub fn new(probe: Arc<dyn FreeSpaceProbe>) -> Self {
        Self { probe }
    }

    pub async fn staging_info(&self, override_path: Option<&str>) -> Result<StagingInfo, AppError> {
        let path = override_path
            .map(PathBuf::from)
            .unwrap_or_else(staging::default_staging_dir);
        let free_bytes = self.probe.free_bytes(&path).await? as i64;
        let stale = count_files(&path).await;
        Ok(StagingInfo {
            path: path.display().to_string(),
            free_bytes,
            stale_file_count: stale as i64,
        })
    }

    pub async fn library_info(&self, library_root: Option<&str>) -> Result<LibraryInfo, AppError> {
        let path = library_root.map(PathBuf::from);
        let free_bytes = if let Some(ref p) = path {
            Some(self.probe.free_bytes(p).await? as i64)
        } else {
            None
        };
        let file_count = if let Some(ref p) = path {
            walk_count(p).await as i64
        } else {
            0
        };
        Ok(LibraryInfo {
            path: path.map(|p| p.display().to_string()),
            free_bytes,
            file_count,
        })
    }
}

async fn count_files(path: &std::path::Path) -> u64 {
    let Ok(mut rd) = tokio::fs::read_dir(path).await else {
        return 0;
    };
    let mut n = 0;
    while let Ok(Some(entry)) = rd.next_entry().await {
        if entry.metadata().await.map(|m| m.is_file()).unwrap_or(false) {
            n += 1;
        }
    }
    n
}

async fn walk_count(path: &std::path::Path) -> u64 {
    // Walks at most 3 levels — enough for `<root>/<streamer>/<season>/<files>`
    // (plex) or `<root>/<login>/<files>` (flat). A full recursive
    // count on a TB-scale library would dominate the settings page
    // load time, so we cap depth intentionally.
    walk_depth(path, 0, 3).await
}

fn walk_depth<'a>(
    path: &'a std::path::Path,
    depth: u32,
    max_depth: u32,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = u64> + Send + 'a>> {
    Box::pin(async move {
        let mut total = 0;
        let Ok(mut rd) = tokio::fs::read_dir(path).await else {
            return 0;
        };
        while let Ok(Some(entry)) = rd.next_entry().await {
            let meta = entry.metadata().await;
            if let Ok(meta) = meta {
                if meta.is_file() {
                    total += 1;
                } else if meta.is_dir() && depth + 1 < max_depth {
                    total += walk_depth(&entry.path(), depth + 1, max_depth).await;
                }
            }
        }
        total
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::infra::fs::space::FakeFreeSpace;

    #[tokio::test]
    async fn staging_info_reports_default_when_no_override() {
        let svc = StorageService::new(Arc::new(FakeFreeSpace(1024 * 1024)));
        let info = svc.staging_info(None).await.unwrap();
        assert!(!info.path.is_empty());
        assert_eq!(info.free_bytes, 1024 * 1024);
    }

    #[tokio::test]
    async fn library_info_with_no_root_yields_empty() {
        let svc = StorageService::new(Arc::new(FakeFreeSpace(0)));
        let info = svc.library_info(None).await.unwrap();
        assert_eq!(info.path, None);
        assert_eq!(info.free_bytes, None);
        assert_eq!(info.file_count, 0);
    }

    #[tokio::test]
    async fn library_info_walks_shallow() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("Sampler").join("Season 2026-04");
        tokio::fs::create_dir_all(&dir).await.unwrap();
        tokio::fs::write(dir.join("one.mp4"), b"").await.unwrap();
        tokio::fs::write(dir.join("two.mp4"), b"").await.unwrap();
        let svc = StorageService::new(Arc::new(FakeFreeSpace(1024)));
        let info = svc
            .library_info(Some(&tmp.path().display().to_string()))
            .await
            .unwrap();
        assert_eq!(info.file_count, 2);
    }
}
