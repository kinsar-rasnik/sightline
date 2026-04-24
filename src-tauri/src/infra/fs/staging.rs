//! Default staging directory resolution + stale-file cleanup.
//!
//! Staging lives outside any cloud-sync provider by default:
//!
//! * **macOS** — `$HOME/Library/Caches/sightline/staging`
//! * **Linux** — `$XDG_CACHE_HOME/sightline/staging` (falls back to
//!   `$HOME/.cache/sightline/staging`)
//! * **Windows** — `%LOCALAPPDATA%\sightline\staging`
//!
//! The user can override via `app_settings.staging_path`. Overrides
//! are validated at set-time; invalid paths fall back to the
//! platform default.
//!
//! Stale files older than `STALE_AGE_SECONDS` (48 h) are swept on
//! app startup.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use crate::error::AppError;

/// Files in staging older than this are considered stale remnants of
/// a crashed download cycle.
pub const STALE_AGE_SECONDS: u64 = 48 * 60 * 60;

/// Compute the platform's default staging path. Never creates the
/// directory — the caller does that lazily.
pub fn default_staging_dir() -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        home_dir()
            .join("Library")
            .join("Caches")
            .join("sightline")
            .join("staging")
    }
    #[cfg(target_os = "linux")]
    {
        if let Some(xdg) = std::env::var_os("XDG_CACHE_HOME") {
            return PathBuf::from(xdg).join("sightline").join("staging");
        }
        home_dir().join(".cache").join("sightline").join("staging")
    }
    #[cfg(target_os = "windows")]
    {
        if let Some(local) = std::env::var_os("LOCALAPPDATA") {
            return PathBuf::from(local).join("sightline").join("staging");
        }
        home_dir()
            .join("AppData")
            .join("Local")
            .join("sightline")
            .join("staging")
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        home_dir().join(".sightline").join("staging")
    }
}

fn home_dir() -> PathBuf {
    if let Some(h) = std::env::var_os("HOME") {
        return PathBuf::from(h);
    }
    #[cfg(target_os = "windows")]
    {
        if let Some(p) = std::env::var_os("USERPROFILE") {
            return PathBuf::from(p);
        }
    }
    PathBuf::from(".")
}

/// Sweep a staging directory for files older than
/// [`STALE_AGE_SECONDS`]. Returns the count of files removed.
///
/// Uses blocking IO on the Tokio blocking pool — sweep runs once at
/// startup, so the slightly more conservative `spawn_blocking` is
/// worth the isolation.
pub async fn cleanup_stale(staging: &Path) -> Result<u64, AppError> {
    let staging = staging.to_owned();
    tokio::task::spawn_blocking(move || cleanup_stale_blocking(&staging))
        .await
        .map_err(|e| AppError::Io {
            detail: format!("staging sweep join: {e}"),
        })?
}

fn cleanup_stale_blocking(staging: &Path) -> Result<u64, AppError> {
    if !staging.exists() {
        return Ok(0);
    }
    let now = SystemTime::now();
    let cutoff = now - Duration::from_secs(STALE_AGE_SECONDS);
    let mut removed: u64 = 0;
    for entry in std::fs::read_dir(staging)? {
        let entry = entry?;
        let meta = entry.metadata()?;
        if !meta.is_file() {
            continue;
        }
        let modified = meta.modified().unwrap_or(now);
        if modified < cutoff && std::fs::remove_file(entry.path()).is_ok() {
            removed += 1;
        }
    }
    Ok(removed)
}

/// Validate a user-supplied staging override. Returns `true` when the
/// path is writable (or can be created) and not under the library
/// root. The caller refuses the override if this returns `false`.
pub async fn validate_override(staging_override: &Path, library_root: Option<&Path>) -> bool {
    if let Some(root) = library_root
        && staging_override.starts_with(root)
    {
        return false;
    }
    // Try to create (idempotent) and write a probe file.
    if tokio::fs::create_dir_all(staging_override).await.is_err() {
        return false;
    }
    let probe = staging_override.join(".sightline-probe");
    let ok = tokio::fs::write(&probe, b"").await.is_ok();
    let _ = tokio::fs::remove_file(&probe).await;
    ok
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn default_dir_mentions_sightline_and_staging() {
        let p = default_staging_dir();
        let s = p.to_string_lossy();
        assert!(s.contains("sightline"), "got {s}");
        assert!(s.contains("staging"), "got {s}");
    }

    #[tokio::test]
    async fn cleanup_stale_deletes_old_files_only() {
        let tmp = tempfile::tempdir().unwrap();
        let old = tmp.path().join("old.part");
        let fresh = tmp.path().join("fresh.part");
        tokio::fs::write(&old, b"stale").await.unwrap();
        tokio::fs::write(&fresh, b"fresh").await.unwrap();

        // Backdate the old file to 3 days ago.
        let three_days = SystemTime::now() - Duration::from_secs(3 * 24 * 3600);
        filetime::set_file_mtime(&old, three_days.into()).unwrap();

        let removed = cleanup_stale(tmp.path()).await.unwrap();
        assert_eq!(removed, 1);
        assert!(!old.exists());
        assert!(fresh.exists());
    }

    #[tokio::test]
    async fn validate_override_rejects_under_library_root() {
        let tmp = tempfile::tempdir().unwrap();
        let library = tmp.path().join("lib");
        tokio::fs::create_dir_all(&library).await.unwrap();
        let bad = library.join("stage");
        assert!(!validate_override(&bad, Some(&library)).await);
    }

    #[tokio::test]
    async fn validate_override_accepts_writable_path() {
        let tmp = tempfile::tempdir().unwrap();
        let good = tmp.path().join("stage");
        assert!(validate_override(&good, None).await);
        assert!(good.exists());
    }

    #[tokio::test]
    async fn cleanup_on_missing_dir_is_noop() {
        let removed = cleanup_stale(Path::new("/nonexistent/sightline/staging"))
            .await
            .unwrap();
        assert_eq!(removed, 0);
    }
}
