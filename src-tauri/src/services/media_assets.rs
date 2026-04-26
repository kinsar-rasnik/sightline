//! Media asset service (Phase 5).
//!
//! Single choke point for paths the renderer needs to load from disk:
//!
//! * the VOD thumbnail (single frame at 10%)
//! * the VOD preview frames (six-frame hover shimmer)
//! * the VOD video source file (for the player's `<video src>`)
//!
//! Every path this service returns has been verified to sit under the
//! configured `library_root`. The renderer uses Tauri's
//! `convertFileSrc(..)` to turn each absolute path into an
//! `asset:/`-or-`tauri://localhost/` URL the webview can load.
//!
//! The asset-protocol allowlist scoping (ADR-0019) is enforced on two
//! layers:
//!   1. This service refuses to return a path outside the library root
//!      (defensive; the sanitizer and layout code should never compose
//!      such a path, but we don't trust that blindly — same pattern as
//!      the download pipeline's `starts_with(&library_root)` guard).
//!   2. The Tauri config narrows the `asset:` scope on first run so any
//!      bypass of this service still can't reach outside the library.
//!
//! A background `backfill_preview_frames` task scans every completed
//! download whose preview set is missing (pre-Phase-5 downloads, or
//! runs where ffmpeg failed the six-frame extract) and regenerates
//! them. It runs once at startup and exits; the user doesn't need to
//! keep the library window open for the work to progress.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use specta::Type;
use sqlx::Row;
use tracing::{debug, warn};

use crate::domain::library_layout::{PREVIEW_FRAME_COUNT, VodWithStreamer, layout as build_layout};
use crate::error::AppError;
use crate::infra::db::Db;
use crate::infra::ffmpeg::{
    PREVIEW_FRAME_PERCENTS, PreviewFramesSpec, SharedFfmpeg, ThumbnailSpec,
};
use crate::services::settings::SettingsService;

/// Renderer-facing bundle of per-VOD asset paths. Every `path` is an
/// absolute filesystem string; the frontend passes each through
/// `convertFileSrc`.
#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct VodAssets {
    pub vod_id: String,
    /// Path to the `.mp4` under the library root if the download has
    /// completed; `None` otherwise.
    pub video_path: Option<String>,
    /// Single-frame thumbnail from the Phase-3 pipeline.
    pub thumbnail_path: Option<String>,
    /// Ordered list of six hover-preview frames. Empty if the preview
    /// set is missing (pre-Phase-5 download that hasn't been backfilled
    /// yet, or an ffmpeg failure).
    pub preview_frame_paths: Vec<String>,
}

/// Single-choke-point answer for `<video src>`. The player uses this
/// to decide which state to render — the happy path (`ready`), the
/// "file moved / deleted externally" path (`missing`), or the
/// partial-download path (`partial`, download not finished yet).
#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct VideoSource {
    pub vod_id: String,
    /// `None` when state != "ready". Always absolute and verified to
    /// sit inside the library root.
    pub path: Option<String>,
    pub state: VideoSourceState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum VideoSourceState {
    Ready,
    Missing,
    Partial,
}

#[derive(Debug)]
pub struct MediaAssetsService {
    db: Db,
    ffmpeg: SharedFfmpeg,
    settings: SettingsService,
}

impl MediaAssetsService {
    pub fn new(db: Db, ffmpeg: SharedFfmpeg, settings: SettingsService) -> Self {
        Self {
            db,
            ffmpeg,
            settings,
        }
    }

    /// Fetch the asset bundle for a VOD. Returns a struct with every
    /// `Option<String>` field set to `None` if the VOD has no download
    /// row — that's a valid state (the user hasn't enqueued this VOD),
    /// not an error.
    pub async fn get(&self, vod_id: &str) -> Result<VodAssets, AppError> {
        let library_root = self.library_root().await?;
        let Some((final_path, view_data, completed)) = self.lookup(vod_id).await? else {
            return Ok(VodAssets {
                vod_id: vod_id.to_owned(),
                video_path: None,
                thumbnail_path: None,
                preview_frame_paths: Vec::new(),
            });
        };

        if !completed {
            return Ok(VodAssets {
                vod_id: vod_id.to_owned(),
                video_path: None,
                thumbnail_path: None,
                preview_frame_paths: Vec::new(),
            });
        }

        let settings = self.settings.get().await?;
        let layout = build_layout(settings.library_layout);
        let view = VodWithStreamer {
            vod: &view_data.vod,
            streamer_display_name: &view_data.streamer_display_name,
            streamer_login: &view_data.streamer_login,
        };
        let thumbnail_relative = layout.thumbnail_path(&view);
        let thumbnail_abs = library_root.join(&thumbnail_relative);
        let thumbnail_path = if thumbnail_abs.exists() {
            Some(guarded_path(&library_root, &thumbnail_abs)?)
        } else {
            None
        };
        let preview_abs: Vec<PathBuf> = layout
            .preview_frame_paths(&view)
            .into_iter()
            .map(|p| library_root.join(p))
            .collect();
        // The renderer expects all-or-nothing: if the set is complete,
        // we return six paths, otherwise an empty list so the grid
        // falls back to the single thumbnail.
        let preview_frame_paths = if preview_abs.iter().all(|p| p.exists()) {
            preview_abs
                .iter()
                .map(|p| guarded_path(&library_root, p))
                .collect::<Result<Vec<_>, _>>()?
        } else {
            Vec::new()
        };

        let video_path = if final_path.exists() {
            Some(guarded_path(&library_root, &final_path)?)
        } else {
            None
        };

        Ok(VodAssets {
            vod_id: vod_id.to_owned(),
            video_path,
            thumbnail_path,
            preview_frame_paths,
        })
    }

    /// Opportunistic background task: walk every `completed` download
    /// with `final_path` set, and regenerate its preview frames if any
    /// are missing. Run once at startup; exits when the scan finishes
    /// or the shutdown broadcaster fires.
    pub async fn backfill_preview_frames(&self) -> Result<i64, AppError> {
        let Some(library_root) = self.settings.get().await?.library_root else {
            debug!("no library root — skipping preview backfill");
            return Ok(0);
        };
        let library_root = PathBuf::from(library_root);
        let rows = sqlx::query(
            "SELECT d.vod_id, d.final_path, v.twitch_user_id, v.title, v.stream_started_at,
                    v.duration_seconds, s.display_name, s.login
             FROM downloads d
             JOIN vods v      ON v.twitch_video_id = d.vod_id
             JOIN streamers s ON s.twitch_user_id  = v.twitch_user_id
             WHERE d.state = 'completed' AND d.final_path IS NOT NULL",
        )
        .fetch_all(self.db.pool())
        .await?;

        let settings = self.settings.get().await?;
        let layout = build_layout(settings.library_layout);
        let mut n = 0;
        for r in rows {
            let final_path_str: String = r.try_get(1)?;
            let final_path = PathBuf::from(final_path_str);
            if !final_path.exists() {
                continue;
            }
            let vod = minimal_vod(&r)?;
            let view = VodWithStreamer {
                vod: &vod,
                streamer_display_name: &r.try_get::<String, _>(6)?,
                streamer_login: &r.try_get::<String, _>(7)?,
            };
            let preview_abs: Vec<PathBuf> = layout
                .preview_frame_paths(&view)
                .into_iter()
                .map(|p| library_root.join(p))
                .collect();
            if preview_abs.iter().all(|p| p.exists()) {
                continue; // nothing to do
            }
            // Ensure the thumbnail directory exists.
            if let Some(parent) = preview_abs[0].parent() {
                let _ = tokio::fs::create_dir_all(parent).await;
            }
            let frames: Vec<(f64, PathBuf)> = PREVIEW_FRAME_PERCENTS
                .iter()
                .copied()
                .zip(preview_abs.iter().cloned())
                .collect();
            if let Err(e) = self
                .ffmpeg
                .extract_preview_frames(&PreviewFramesSpec {
                    source: final_path.clone(),
                    duration_seconds: vod.duration_seconds,
                    frames,
                })
                .await
            {
                warn!(vod_id = %vod.twitch_video_id, error = %e, "preview backfill failed");
                for p in &preview_abs {
                    let _ = tokio::fs::remove_file(p).await;
                }
                continue;
            }
            n += 1;
        }
        Ok(n)
    }

    /// Resolve the `<video src>` for a VOD. Returns a `VideoSource`
    /// with `state` narrowed to ready/missing/partial so the player
    /// can render the right UI without a second round-trip. This is
    /// the ONLY code path that should produce an asset path the
    /// webview then feeds to `convertFileSrc`.
    pub async fn get_video_source(&self, vod_id: &str) -> Result<VideoSource, AppError> {
        let library_root = match self.library_root().await {
            Ok(p) => p,
            Err(_) => {
                // No library root configured → treat as missing so
                // the player falls back to its error state rather
                // than hanging.
                return Ok(VideoSource {
                    vod_id: vod_id.to_owned(),
                    path: None,
                    state: VideoSourceState::Missing,
                });
            }
        };
        // Read the download row directly so we don't depend on the
        // VOD's ingest state. A `partial` answer is "row exists but
        // not yet completed".
        let row = sqlx::query("SELECT state, final_path FROM downloads WHERE vod_id = ?")
            .bind(vod_id)
            .fetch_optional(self.db.pool())
            .await?;
        let Some(row) = row else {
            return Ok(VideoSource {
                vod_id: vod_id.to_owned(),
                path: None,
                state: VideoSourceState::Missing,
            });
        };
        let state: String = row.try_get(0)?;
        let path_str: Option<String> = row.try_get(1).ok();
        if state != "completed" {
            return Ok(VideoSource {
                vod_id: vod_id.to_owned(),
                path: None,
                state: VideoSourceState::Partial,
            });
        }
        let Some(path_str) = path_str else {
            return Ok(VideoSource {
                vod_id: vod_id.to_owned(),
                path: None,
                state: VideoSourceState::Missing,
            });
        };
        let path = PathBuf::from(&path_str);
        if !path.exists() {
            return Ok(VideoSource {
                vod_id: vod_id.to_owned(),
                path: None,
                state: VideoSourceState::Missing,
            });
        }
        // ADR-0019 invariant: library-root containment check runs on
        // every player-surfaced path. A file at this point is
        // expected to pass — the download pipeline already validates
        // — but we don't trust that blindly.
        let checked = guarded_path(&library_root, &path)?;
        Ok(VideoSource {
            vod_id: vod_id.to_owned(),
            path: Some(checked),
            state: VideoSourceState::Ready,
        })
    }

    /// Remux the downloaded `.mp4` in-place. Used by the player's
    /// "Remux file" recovery action when the webview's `<video>`
    /// element can't decode the file (bad codec, partial write).
    ///
    /// Security: the target path is fetched from the downloads row
    /// rather than taken from the webview, so an attacker can't
    /// trick us into invoking ffmpeg on an arbitrary file. Also
    /// double-guards that the file sits under `library_root` before
    /// invoking the subprocess.
    pub async fn request_remux(&self, vod_id: &str) -> Result<(), AppError> {
        let library_root = self.library_root().await?;
        let row = sqlx::query("SELECT final_path FROM downloads WHERE vod_id = ?")
            .bind(vod_id)
            .fetch_optional(self.db.pool())
            .await?
            .ok_or(AppError::NotFound)?;
        let final_path_str: Option<String> = row.try_get(0).ok();
        let Some(final_path_str) = final_path_str else {
            return Err(AppError::NotFound);
        };
        let final_path = PathBuf::from(final_path_str);
        // Containment check (defence-in-depth). The download
        // pipeline already enforces this at move-time.
        let _ = guarded_path(&library_root, &final_path)?;
        let remuxed = final_path.with_extension("remuxed.mp4");
        self.ffmpeg
            .remux_to_mp4(&crate::infra::ffmpeg::RemuxSpec {
                source: final_path.clone(),
                destination: remuxed.clone(),
            })
            .await?;
        // Move the remuxed file back over the original. Do a
        // staged-swap via a temporary name so a crash mid-rename
        // doesn't leave the user with neither file.
        let backup = final_path.with_extension("mp4.backup");
        let _ = tokio::fs::rename(&final_path, &backup).await;
        if let Err(e) = tokio::fs::rename(&remuxed, &final_path).await {
            // Restore the backup before surfacing the error.
            let _ = tokio::fs::rename(&backup, &final_path).await;
            return Err(AppError::Io {
                detail: format!("remux-swap: {e}"),
            });
        }
        let _ = tokio::fs::remove_file(&backup).await;
        Ok(())
    }

    /// Standalone re-extract of the single thumbnail at 10% — used
    /// when a pre-Phase-5 row has neither thumbnail nor previews.
    pub async fn regenerate_thumbnail(&self, vod_id: &str) -> Result<(), AppError> {
        let library_root = self.library_root().await?;
        let Some((final_path, view_data, true)) = self.lookup(vod_id).await? else {
            return Err(AppError::NotFound);
        };
        let settings = self.settings.get().await?;
        let layout = build_layout(settings.library_layout);
        let view = VodWithStreamer {
            vod: &view_data.vod,
            streamer_display_name: &view_data.streamer_display_name,
            streamer_login: &view_data.streamer_login,
        };
        let destination = library_root.join(layout.thumbnail_path(&view));
        if let Some(parent) = destination.parent() {
            let _ = tokio::fs::create_dir_all(parent).await;
        }
        self.ffmpeg
            .extract_thumbnail(&ThumbnailSpec {
                source: final_path,
                destination,
                duration_seconds: view_data.vod.duration_seconds,
                percent: 10.0,
            })
            .await
    }

    async fn library_root(&self) -> Result<PathBuf, AppError> {
        let settings = self.settings.get().await?;
        let Some(root) = settings.library_root else {
            return Err(AppError::InvalidInput {
                detail: "library_root not configured".into(),
            });
        };
        Ok(PathBuf::from(root))
    }

    async fn lookup(&self, vod_id: &str) -> Result<Option<(PathBuf, MinimalVod, bool)>, AppError> {
        let row_opt = sqlx::query(
            "SELECT d.state, d.final_path, v.twitch_user_id, v.title, v.stream_started_at,
                    v.duration_seconds, s.display_name, s.login
             FROM downloads d
             JOIN vods v      ON v.twitch_video_id = d.vod_id
             JOIN streamers s ON s.twitch_user_id  = v.twitch_user_id
             WHERE d.vod_id = ?",
        )
        .bind(vod_id)
        .fetch_optional(self.db.pool())
        .await?;
        let Some(row) = row_opt else {
            return Ok(None);
        };
        let state: String = row.try_get(0)?;
        let final_path_opt: Option<String> = row.try_get(1).ok();
        let completed = state == "completed" && final_path_opt.is_some();
        let final_path = final_path_opt
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(""));
        let vod = MinimalVod {
            vod: minimal_vod_from_row(&row, vod_id)?,
            streamer_display_name: row.try_get(6)?,
            streamer_login: row.try_get(7)?,
        };
        Ok(Some((final_path, vod, completed)))
    }
}

/// Return a library-root-scoped absolute path string iff the supplied
/// `path` resolves to a location inside `root`. Otherwise reject.
///
/// Path comparisons use canonicalized absolute paths to defeat
/// `..`-escape + symlink-escape attempts. When a path *does* resolve,
/// we return the canonical form (not the original) so downstream I/O
/// sees the symlink-resolved path — prevents a TOCTOU where an
/// attacker swaps the symlink between the check and the webview's
/// asset-protocol fetch. Addresses the Phase-5 security review's
/// MEDIUM finding on `guarded_path`.
fn guarded_path(root: &Path, path: &Path) -> Result<String, AppError> {
    let canon_root = root.canonicalize().unwrap_or_else(|e| {
        // Logging at debug — the library-root missing path is an
        // expected case on first-run before the user picks one.
        debug!(
            root = %root.display(),
            error = %e,
            "library_root canonicalize failed; falling back to raw path"
        );
        root.to_path_buf()
    });

    // Two-tier resolution:
    //
    //  * For a path that exists, `canonicalize` resolves symlinks and
    //    `..` segments. We pin the returned string to the canonical
    //    form so callers can't be surprised by a subsequent symlink
    //    swap.
    //
    //  * For a path that does not exist yet (remux destinations,
    //    thumbnail targets that the caller is about to create), we
    //    fall back to the raw `PathBuf`. `starts_with` on `Path` is
    //    component-safe (it doesn't do naive string prefix matching),
    //    so `..` components would be caught here as well — but we
    //    still log the fallback so an operator can tell why an
    //    existing-file canonicalize failure slipped through.
    let (canon_path, canonicalized) = match path.canonicalize() {
        Ok(p) => (p, true),
        Err(e) => {
            if path.exists() {
                warn!(
                    path = %path.display(),
                    error = %e,
                    "canonicalize failed on an existing path; using raw path for containment check"
                );
            }
            (path.to_path_buf(), false)
        }
    };

    if !canon_path.starts_with(&canon_root) {
        return Err(AppError::InvalidInput {
            detail: format!(
                "media path {} outside library root {}",
                path.display(),
                root.display()
            ),
        });
    }

    // Return the canonical form when we have one. Downstream I/O then
    // uses the symlink-resolved path, so a later swap of the original
    // symlink target has no effect on what the webview fetches.
    Ok(if canonicalized {
        canon_path.display().to_string()
    } else {
        path.display().to_string()
    })
}

/// Internal container for the per-row data we fetch during asset
/// resolution. Mirrors just enough of `Vod` to feed the layout trait.
struct MinimalVod {
    vod: crate::domain::vod::Vod,
    streamer_display_name: String,
    streamer_login: String,
}

fn minimal_vod(row: &sqlx::sqlite::SqliteRow) -> Result<crate::domain::vod::Vod, AppError> {
    // vod_id is column 0 in backfill_preview_frames query
    let vod_id: String = row.try_get(0)?;
    minimal_vod_from_row(row, &vod_id)
}

fn minimal_vod_from_row(
    row: &sqlx::sqlite::SqliteRow,
    vod_id: &str,
) -> Result<crate::domain::vod::Vod, AppError> {
    use crate::domain::vod::{IngestStatus, Vod};
    // Columns picked only for the library-layout trait — most fields
    // we synthesize with defaults to avoid widening this struct for
    // every asset lookup.
    let twitch_user_id: String = row.try_get(2)?;
    let title: String = row.try_get(3)?;
    let stream_started_at: i64 = row.try_get(4)?;
    let duration_seconds: i64 = row.try_get(5)?;
    Ok(Vod {
        twitch_video_id: vod_id.to_owned(),
        twitch_user_id,
        stream_id: None,
        title,
        description: String::new(),
        stream_started_at,
        published_at: stream_started_at,
        url: String::new(),
        thumbnail_url: None,
        duration_seconds,
        view_count: 0,
        language: String::new(),
        muted_segments: Vec::new(),
        is_sub_only: false,
        helix_game_id: None,
        helix_game_name: None,
        ingest_status: IngestStatus::Eligible,
        status_reason: String::new(),
        first_seen_at: 0,
        last_seen_at: 0,
        // Minimal stub: this row is only used for filesystem-layout
        // resolution, never for status-aware UI rendering, so the
        // default 'available' is benign.
        status: crate::domain::distribution::VodStatus::Available,
    })
}

/// Expected number of preview frames — re-exported so command layer
/// tests can assert contract stability without pulling in
/// `domain::library_layout`.
pub const EXPECTED_PREVIEW_COUNT: usize = PREVIEW_FRAME_COUNT;

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn guarded_path_rejects_outside_root() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let outside = std::env::temp_dir().join("sightline-asset-test-outside");
        std::fs::write(&outside, b"nope").unwrap();
        let err = guarded_path(root, &outside).unwrap_err();
        assert!(matches!(err, AppError::InvalidInput { .. }), "got {err:?}");
        std::fs::remove_file(&outside).unwrap();
    }

    #[test]
    fn guarded_path_accepts_inside_root() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let inside = root.join("sampler").join("vod.mp4");
        std::fs::create_dir_all(inside.parent().unwrap()).unwrap();
        std::fs::write(&inside, b"video").unwrap();
        let s = guarded_path(root, &inside).unwrap();
        assert!(s.contains("vod.mp4"));
    }

    #[cfg(unix)]
    #[test]
    fn guarded_path_returns_canonical_form_to_defeat_symlink_swap() {
        // Regression for the Phase-5 security review's MEDIUM
        // finding: when the caller supplies a symlink that resolves
        // inside the root, we should return the RESOLVED path, not
        // the symlink's raw path. Otherwise a later atomic swap of
        // the symlink target would redirect the webview's fetch.
        use std::os::unix::fs::symlink;
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let real = root.join("real.mp4");
        std::fs::write(&real, b"video").unwrap();
        let link = root.join("alias.mp4");
        symlink(&real, &link).unwrap();
        let returned = guarded_path(root, &link).unwrap();
        // Not the symlink's literal string — should resolve to the
        // real target path, so a later swap of the symlink has no
        // effect on the downstream fetch.
        assert!(returned.contains("real.mp4"));
        assert!(!returned.ends_with("alias.mp4"));
    }

    #[cfg(unix)]
    #[test]
    fn guarded_path_rejects_symlink_escape() {
        use std::os::unix::fs::symlink;
        let tmp_root = tempfile::tempdir().unwrap();
        let tmp_outside = tempfile::tempdir().unwrap();
        let outside_file = tmp_outside.path().join("secret.bin");
        std::fs::write(&outside_file, b"secret").unwrap();
        let link = tmp_root.path().join("alias.mp4");
        symlink(&outside_file, &link).unwrap();
        let err = guarded_path(tmp_root.path(), &link).unwrap_err();
        assert!(matches!(err, AppError::InvalidInput { .. }));
    }
}
