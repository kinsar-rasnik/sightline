//! Atomic-or-verify-and-delete file move.
//!
//! `tokio::fs::rename` is atomic only when source and destination are
//! on the same filesystem. When they aren't (staging on local disk,
//! library on an external drive is a typical case), `rename` returns
//! `EXDEV`. In that case we fall back to:
//!
//! 1. `tokio::fs::copy` from source to destination while computing a
//!    SHA-256 of the source.
//! 2. `tokio::fs::File::sync_all` on the destination (fsync).
//! 3. Re-hash the destination and compare.
//! 4. On match, delete the source. On mismatch, delete the
//!    destination and return an error.
//!
//! The destination parent directory is created if missing.

use std::io;
use std::path::Path;

use sha2::{Digest, Sha256};
use tokio::fs;
use tokio::io::AsyncReadExt;

use crate::error::AppError;

/// Outcome of an atomic move — useful for tracing + telemetry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveKind {
    /// `rename` succeeded — the move was truly atomic.
    Renamed,
    /// Cross-filesystem: copy + verify + delete-source.
    Copied,
}

pub async fn atomic_move(src: &Path, dst: &Path) -> Result<MoveKind, AppError> {
    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent).await?;
    }

    match fs::rename(src, dst).await {
        Ok(()) => Ok(MoveKind::Renamed),
        Err(e) if cross_filesystem(&e) => copy_verify_delete(src, dst)
            .await
            .map(|()| MoveKind::Copied),
        Err(e) => Err(AppError::Io {
            detail: format!("rename failed: {e}"),
        }),
    }
}

async fn copy_verify_delete(src: &Path, dst: &Path) -> Result<(), AppError> {
    let source_hash = hash_file(src).await?;
    fs::copy(src, dst).await?;
    sync_destination(dst).await?;
    let dst_hash = hash_file(dst).await?;
    if source_hash != dst_hash {
        let _ = fs::remove_file(dst).await;
        return Err(AppError::Io {
            detail: format!(
                "copy verification failed: {} != {}",
                hex::encode(source_hash),
                hex::encode(dst_hash)
            ),
        });
    }
    fs::remove_file(src).await?;
    Ok(())
}

async fn sync_destination(dst: &Path) -> Result<(), AppError> {
    let f = fs::OpenOptions::new().write(true).open(dst).await?;
    f.sync_all().await?;
    Ok(())
}

async fn hash_file(path: &Path) -> Result<Vec<u8>, AppError> {
    let mut f = fs::File::open(path).await?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = f.read(&mut buf).await?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hasher.finalize().to_vec())
}

fn cross_filesystem(e: &io::Error) -> bool {
    #[cfg(unix)]
    {
        e.raw_os_error() == Some(libc_exdev())
    }
    #[cfg(not(unix))]
    {
        // Windows maps EXDEV-equivalent to ERROR_NOT_SAME_DEVICE (0x11).
        // Fall back to comparing the message prefix because Rust's
        // `io::ErrorKind::CrossesDevices` is unstable.
        let msg = e.to_string().to_lowercase();
        msg.contains("different device") || msg.contains("not_same_device") || msg.contains("17")
    }
}

#[cfg(unix)]
fn libc_exdev() -> i32 {
    // EXDEV = 18 on every Unix we care about. Hardcoded rather than
    // pulled in via `libc` — the whole point of this helper is to
    // keep the infra crate dependency footprint small.
    18
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn rename_within_tempdir_succeeds() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("a.mp4");
        let dst = tmp.path().join("sub/b.mp4");
        fs::write(&src, b"payload").await.unwrap();
        let kind = atomic_move(&src, &dst).await.unwrap();
        assert_eq!(kind, MoveKind::Renamed);
        assert!(!src.exists());
        assert_eq!(fs::read(&dst).await.unwrap(), b"payload");
    }

    #[tokio::test]
    async fn copy_verify_delete_round_trip() {
        // We can't force a cross-FS rename in a test, so exercise the
        // copy path directly.
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("a.bin");
        let dst = tmp.path().join("out/b.bin");
        let data: Vec<u8> = (0..10_000).map(|i| (i % 251) as u8).collect();
        fs::write(&src, &data).await.unwrap();
        fs::create_dir_all(dst.parent().unwrap()).await.unwrap();
        copy_verify_delete(&src, &dst).await.unwrap();
        assert!(!src.exists());
        assert_eq!(fs::read(&dst).await.unwrap(), data);
    }

    #[tokio::test]
    async fn copy_verify_errors_on_missing_source() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("missing.bin");
        let dst = tmp.path().join("out.bin");
        let err = copy_verify_delete(&src, &dst).await.unwrap_err();
        assert!(matches!(err, AppError::Io { .. }));
    }

    #[test]
    fn cross_filesystem_recognises_exdev() {
        #[cfg(unix)]
        {
            let err = io::Error::from_raw_os_error(18);
            assert!(cross_filesystem(&err));
        }
    }
}
