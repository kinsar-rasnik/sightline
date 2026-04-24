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

use crate::domain::library_layout::{
    PREVIEW_FRAME_COUNT, VodWithStreamer, layout as build_layout,
};
use crate::error::AppError;
use crate::infra::db::Db;
use crate::infra::ffmpeg::{PREVIEW_FRAME_PERCENTS, PreviewFramesSpec, SharedFfmpeg, ThumbnailSpec};
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

    async fn lookup(
        &self,
        vod_id: &str,
    ) -> Result<Option<(PathBuf, MinimalVod, bool)>, AppError> {
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

/// Return `path` as a string iff it's inside `root`; otherwise reject.
/// Path comparisons use canonicalized absolute paths to defeat
/// `..`-escape attempts.
fn guarded_path(root: &Path, path: &Path) -> Result<String, AppError> {
    // Canonicalize when possible; otherwise fall back to a literal
    // starts_with check on the abs path. We can't require canonicalize
    // to succeed — on Windows, missing-file paths can't be
    // canonicalized.
    let canon_root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let canon_path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    if !canon_path.starts_with(&canon_root) {
        return Err(AppError::InvalidInput {
            detail: format!(
                "media path {} outside library root {}",
                path.display(),
                root.display()
            ),
        });
    }
    Ok(path.display().to_string())
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
}
