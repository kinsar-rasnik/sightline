//! Free-space reporting.
//!
//! `std::fs::metadata` doesn't expose disk space; we use `sysinfo`'s
//! `Disks` to enumerate mount points and pick the one whose path is
//! the deepest prefix of the query path. Returns bytes.
//!
//! For tests, `FreeSpaceProbe` is a trait so the queue service can
//! inject an in-memory probe.

use std::path::{Path, PathBuf};

use async_trait::async_trait;
use sysinfo::Disks;

use crate::error::AppError;

#[async_trait]
pub trait FreeSpaceProbe: Send + Sync + std::fmt::Debug {
    /// Return the free bytes available at `path` (which may or may
    /// not exist — we fall back to the deepest existing ancestor).
    async fn free_bytes(&self, path: &Path) -> Result<u64, AppError>;

    /// Return the partition's total capacity in bytes at `path`.
    /// Used by the auto-cleanup watermark math (Phase 7, ADR-0024) —
    /// any probe that can answer `free_bytes` can answer `total_bytes`
    /// because both come from the same `Disks` list.
    async fn total_bytes(&self, path: &Path) -> Result<u64, AppError>;
}

#[derive(Debug, Default)]
pub struct SystemFreeSpace;

#[async_trait]
impl FreeSpaceProbe for SystemFreeSpace {
    async fn free_bytes(&self, path: &Path) -> Result<u64, AppError> {
        // sysinfo is synchronous; run it on the blocking pool so we
        // don't stall the runtime.
        let path = path.to_owned();
        tokio::task::spawn_blocking(move || blocking_disk_lookup(&path).map(|(_, free)| free))
            .await
            .map_err(|e| AppError::Io {
                detail: format!("free_bytes join: {e}"),
            })?
    }

    async fn total_bytes(&self, path: &Path) -> Result<u64, AppError> {
        let path = path.to_owned();
        tokio::task::spawn_blocking(move || blocking_disk_lookup(&path).map(|(total, _)| total))
            .await
            .map_err(|e| AppError::Io {
                detail: format!("total_bytes join: {e}"),
            })?
    }
}

fn blocking_disk_lookup(path: &Path) -> Result<(u64, u64), AppError> {
    let target = canonicalise_for_lookup(path);
    let disks = Disks::new_with_refreshed_list();
    let mut best_match: Option<(PathBuf, u64, u64)> = None;
    for disk in disks.list() {
        let mount = disk.mount_point();
        if target.starts_with(mount) {
            let is_better = match &best_match {
                None => true,
                Some((current, _, _)) => mount.as_os_str().len() > current.as_os_str().len(),
            };
            if is_better {
                best_match = Some((mount.to_owned(), disk.total_space(), disk.available_space()));
            }
        }
    }
    best_match
        .map(|(_, total, free)| (total, free))
        .ok_or_else(|| AppError::Io {
            detail: format!("no disk mount covers {}", target.display()),
        })
}

fn canonicalise_for_lookup(path: &Path) -> PathBuf {
    // Walk up to the nearest existing ancestor so `canonicalize`
    // succeeds even when the target doesn't exist yet (common for
    // freshly-chosen library roots).
    let mut cur = path.to_owned();
    loop {
        if cur.exists() {
            if let Ok(canon) = std::fs::canonicalize(&cur) {
                return canon;
            }
            return cur;
        }
        match cur.parent() {
            Some(parent) => cur = parent.to_owned(),
            None => return path.to_owned(),
        }
    }
}

/// In-memory probe for tests. Always returns the configured value.
/// Total capacity defaults to `free_bytes * 4` so usage math stays
/// well-defined; pre-Phase-7 callers that only exercised
/// `free_bytes` continue to work unchanged.
#[derive(Debug, Clone)]
pub struct FakeFreeSpace(pub u64);

#[async_trait]
impl FreeSpaceProbe for FakeFreeSpace {
    async fn free_bytes(&self, _path: &Path) -> Result<u64, AppError> {
        Ok(self.0)
    }

    async fn total_bytes(&self, _path: &Path) -> Result<u64, AppError> {
        Ok(self.0.saturating_mul(4))
    }
}

/// Probe with explicit total + free (Phase 7 watermark tests).
#[derive(Debug, Clone)]
pub struct FakeDiskUsage {
    pub total: u64,
    pub free: u64,
}

#[async_trait]
impl FreeSpaceProbe for FakeDiskUsage {
    async fn free_bytes(&self, _path: &Path) -> Result<u64, AppError> {
        Ok(self.free)
    }

    async fn total_bytes(&self, _path: &Path) -> Result<u64, AppError> {
        Ok(self.total)
    }
}

/// The preflight rule from the mission brief: staging must hold
/// `size * 1.2`, library must hold `size * 1.1`. Returns `Ok(())` when
/// both partitions are fine; returns `Err(AppError::DiskFull {path})`
/// naming the first partition that's short.
pub async fn check_preflight<P: FreeSpaceProbe + ?Sized>(
    probe: &P,
    staging_path: &Path,
    library_path: &Path,
    download_bytes: u64,
) -> Result<(), AppError> {
    let required_staging = scaled(download_bytes, 12); // 1.2x
    let required_library = scaled(download_bytes, 11); // 1.1x
    let free_staging = probe.free_bytes(staging_path).await?;
    if free_staging < required_staging {
        return Err(AppError::DiskFull {
            path: staging_path.display().to_string(),
        });
    }
    let free_library = probe.free_bytes(library_path).await?;
    if free_library < required_library {
        return Err(AppError::DiskFull {
            path: library_path.display().to_string(),
        });
    }
    Ok(())
}

fn scaled(base: u64, tenths: u64) -> u64 {
    base.saturating_mul(tenths).saturating_div(10)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn fake_probe_returns_configured_value() {
        let probe = FakeFreeSpace(100);
        assert_eq!(probe.free_bytes(Path::new("/anything")).await.unwrap(), 100);
    }

    #[tokio::test]
    async fn preflight_passes_when_both_partitions_have_room() {
        let probe = FakeFreeSpace(10_000);
        check_preflight(&probe, &PathBuf::from("/s"), &PathBuf::from("/l"), 1_000)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn preflight_fails_when_staging_short() {
        let probe = FakeFreeSpace(1_000);
        // 2000 bytes download needs 2400 in staging; probe has 1000.
        let err = check_preflight(&probe, &PathBuf::from("/s"), &PathBuf::from("/l"), 2_000)
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::DiskFull { .. }));
    }

    #[test]
    fn scaled_math() {
        assert_eq!(scaled(100, 12), 120);
        assert_eq!(scaled(100, 11), 110);
        assert_eq!(scaled(0, 12), 0);
    }
}
