//! Library layout backends — pluggable file layout for downloaded VODs.
//!
//! The user picks between two layouts:
//!
//! * **Plex/Jellyfin** — one folder per streamer, one sub-folder per
//!   calendar month, each VOD as a trio of files (video + NFO + thumb)
//!   named `<Display> - YYYY-MM-DD - <Title> [twitch-<id>].<ext>`.
//!   Kodi-compatible NFOs let Infuse / Plex / Jellyfin pick up the
//!   metadata on scan.
//! * **Flat** — one folder per streamer (login), each VOD as a single
//!   `.mp4` named `YYYY-MM-DD_<id>_<slug>.mp4`. Thumbnails go into a
//!   hidden `.thumbs/` subdirectory so scanners don't index them.
//!
//! All paths are computed from pure data (`VodWithStreamer`) without
//! touching the filesystem. The concrete `LibraryMigrator` in
//! `services/library_migrator.rs` owns the I/O.

use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use specta::Type;

use crate::domain::sanitize::{sanitize_component, slug};
use crate::domain::vod::Vod;

/// The two layout options surfaced on the Settings page. Wire strings
/// match the `CHECK` constraint on `app_settings.library_layout`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum LibraryLayoutKind {
    /// Plex / Jellyfin / Infuse — one folder per streamer, Kodi NFO sidecar.
    Plex,
    /// Hidden-thumb flat layout. Minimal, no sidecars on disk beyond
    /// the video itself.
    Flat,
}

impl LibraryLayoutKind {
    pub fn as_db_str(self) -> &'static str {
        match self {
            LibraryLayoutKind::Plex => "plex",
            LibraryLayoutKind::Flat => "flat",
        }
    }

    pub fn from_db_str(s: &str) -> Option<Self> {
        Some(match s {
            "plex" => LibraryLayoutKind::Plex,
            "flat" => LibraryLayoutKind::Flat,
            _ => return None,
        })
    }
}

/// Purely-data view of a VOD + owning streamer, just enough to compute
/// library paths. Constructed by the queue service before handing off
/// to a `LibraryLayout` implementation.
#[derive(Debug, Clone)]
pub struct VodWithStreamer<'a> {
    pub vod: &'a Vod,
    pub streamer_display_name: &'a str,
    pub streamer_login: &'a str,
}

/// Extra files produced alongside the video. Each entry's `filename`
/// is relative to the same directory as the video file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SidecarFile {
    pub filename: String,
    pub kind: SidecarKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SidecarKind {
    /// Kodi-compatible XML metadata file.
    Nfo,
    /// JPEG thumbnail.
    Thumbnail,
}

/// Layout strategy. Paths are always computed relative to the user's
/// library root; the caller joins the root onto the return value.
pub trait LibraryLayout: Send + Sync {
    /// Absolute-within-library path for the video file of this VOD,
    /// e.g. `<root>/Sampler/Season 2026-04/Sampler - 2026-04-01 - Title [twitch-v1].mp4`.
    fn path_for(&self, v: &VodWithStreamer) -> PathBuf;

    /// Extra files the backend wants written next to the video.
    fn sidecars_for(&self, v: &VodWithStreamer) -> Vec<SidecarFile>;

    /// Thumbnail path — may be inside a hidden subdirectory for the
    /// Flat backend. Returned as a plain `PathBuf` rather than a
    /// `SidecarFile` because the thumbnail path isn't always
    /// co-located with the video (see Flat's `.thumbs/`).
    fn thumbnail_path(&self, v: &VodWithStreamer) -> PathBuf;

    /// Hover-preview frame paths (Phase 5). Co-located with the
    /// thumbnail so `frame_count` is the public contract regardless of
    /// layout — default impl derives paths from `thumbnail_path`,
    /// appending `-preview-NN.jpg`. The caller ensures `frame_count`
    /// matches the scheme in [`PREVIEW_FRAME_COUNT`].
    fn preview_frame_paths(&self, v: &VodWithStreamer) -> Vec<PathBuf> {
        let thumb = self.thumbnail_path(v);
        let parent = thumb.parent().map(|p| p.to_path_buf()).unwrap_or_default();
        let stem = thumb
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("preview")
            .to_owned();
        (1..=PREVIEW_FRAME_COUNT)
            .map(|i| parent.join(format!("{stem}-preview-{i:02}.jpg")))
            .collect()
    }

    /// Human-readable name used in the Settings UI preview.
    fn describe(&self) -> &'static str;
}

/// Number of frames used by the library-grid hover preview. Matches
/// [`crate::infra::ffmpeg::PREVIEW_FRAME_PERCENTS`] length.
pub const PREVIEW_FRAME_COUNT: usize = 6;

/// Plex/Jellyfin layout.
#[derive(Debug, Default, Clone, Copy)]
pub struct PlexLayout;

impl LibraryLayout for PlexLayout {
    fn path_for(&self, v: &VodWithStreamer) -> PathBuf {
        let streamer_dir = sanitize_component(v.streamer_display_name);
        let (season_dir, date) = plex_season_and_date(v.vod.stream_started_at);
        let filename = plex_filename(v, &date, "mp4");
        PathBuf::from(streamer_dir).join(season_dir).join(filename)
    }

    fn sidecars_for(&self, v: &VodWithStreamer) -> Vec<SidecarFile> {
        let (_, date) = plex_season_and_date(v.vod.stream_started_at);
        vec![
            SidecarFile {
                filename: plex_filename(v, &date, "nfo"),
                kind: SidecarKind::Nfo,
            },
            SidecarFile {
                filename: plex_thumb_filename(v, &date),
                kind: SidecarKind::Thumbnail,
            },
        ]
    }

    fn thumbnail_path(&self, v: &VodWithStreamer) -> PathBuf {
        let streamer_dir = sanitize_component(v.streamer_display_name);
        let (season_dir, date) = plex_season_and_date(v.vod.stream_started_at);
        PathBuf::from(streamer_dir)
            .join(season_dir)
            .join(plex_thumb_filename(v, &date))
    }

    fn describe(&self) -> &'static str {
        "Plex / Jellyfin / Infuse — one folder per streamer with Kodi .nfo + thumbnail sidecars."
    }
}

/// Flat per-streamer-login layout.
#[derive(Debug, Default, Clone, Copy)]
pub struct FlatLayout;

impl LibraryLayout for FlatLayout {
    fn path_for(&self, v: &VodWithStreamer) -> PathBuf {
        let login_dir = sanitize_component(v.streamer_login);
        let date = format_iso_date(v.vod.stream_started_at);
        PathBuf::from(login_dir).join(flat_filename(v, &date, "mp4"))
    }

    fn sidecars_for(&self, _v: &VodWithStreamer) -> Vec<SidecarFile> {
        // Flat layout is intentionally bare-bones. Thumbnails go under
        // a hidden `.thumbs/` subdirectory (not a sidecar — handled
        // by `thumbnail_path` directly).
        Vec::new()
    }

    fn thumbnail_path(&self, v: &VodWithStreamer) -> PathBuf {
        let login_dir = sanitize_component(v.streamer_login);
        let date = format_iso_date(v.vod.stream_started_at);
        PathBuf::from(login_dir)
            .join(".thumbs")
            .join(flat_filename(v, &date, "jpg"))
    }

    fn describe(&self) -> &'static str {
        "Flat — one folder per streamer login; thumbnails hidden in .thumbs/."
    }
}

/// Return both the Plex season folder name ("Season YYYY-MM") and the
/// ISO date string ("YYYY-MM-DD") for a stream-start unix timestamp.
fn plex_season_and_date(unix_seconds: i64) -> (String, String) {
    let dt = DateTime::<Utc>::from_timestamp(unix_seconds, 0)
        .unwrap_or_else(|| DateTime::<Utc>::from_timestamp(0, 0).unwrap_or_default());
    let season = format!("Season {}", dt.format("%Y-%m"));
    let date = dt.format("%Y-%m-%d").to_string();
    (season, date)
}

fn format_iso_date(unix_seconds: i64) -> String {
    let dt = DateTime::<Utc>::from_timestamp(unix_seconds, 0)
        .unwrap_or_else(|| DateTime::<Utc>::from_timestamp(0, 0).unwrap_or_default());
    dt.format("%Y-%m-%d").to_string()
}

fn plex_filename(v: &VodWithStreamer, date: &str, ext: &str) -> String {
    let title = sanitize_component(&v.vod.title);
    let streamer = sanitize_component(v.streamer_display_name);
    let stem = format!(
        "{streamer} - {date} - {title} [twitch-{id}]",
        id = v.vod.twitch_video_id
    );
    let stem = sanitize_component(&stem);
    format!("{stem}.{ext}")
}

fn plex_thumb_filename(v: &VodWithStreamer, date: &str) -> String {
    let title = sanitize_component(&v.vod.title);
    let streamer = sanitize_component(v.streamer_display_name);
    let stem = format!(
        "{streamer} - {date} - {title} [twitch-{id}]-thumb",
        id = v.vod.twitch_video_id
    );
    let stem = sanitize_component(&stem);
    format!("{stem}.jpg")
}

fn flat_filename(v: &VodWithStreamer, date: &str, ext: &str) -> String {
    let s = slug(&v.vod.title);
    let stem = if s.is_empty() {
        format!("{date}_{id}", id = v.vod.twitch_video_id)
    } else {
        format!("{date}_{id}_{s}", id = v.vod.twitch_video_id, s = s)
    };
    // Filename-safe by construction (date + numeric id + slug) but run
    // through the component sanitizer for belt+braces.
    let stem = sanitize_component(&stem);
    format!("{stem}.{ext}")
}

/// Build the right layout impl for a `LibraryLayoutKind`. Returned as
/// `Box<dyn LibraryLayout>` so callers don't need to duplicate the
/// match everywhere.
pub fn layout(kind: LibraryLayoutKind) -> Box<dyn LibraryLayout> {
    match kind {
        LibraryLayoutKind::Plex => Box::new(PlexLayout),
        LibraryLayoutKind::Flat => Box::new(FlatLayout),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::domain::vod::IngestStatus;

    fn vod_fixture() -> Vod {
        Vod {
            twitch_video_id: "v1".into(),
            twitch_user_id: "100".into(),
            stream_id: None,
            title: "GTA RP: Test Run — dan's fun time".into(),
            description: String::new(),
            stream_started_at: 1_775_088_000, // 2026-04-02 00:00:00 UTC
            published_at: 1_712_068_800,
            url: "https://twitch.tv/videos/v1".into(),
            thumbnail_url: None,
            duration_seconds: 3600,
            view_count: 0,
            language: "en".into(),
            muted_segments: vec![],
            is_sub_only: false,
            helix_game_id: None,
            helix_game_name: None,
            ingest_status: IngestStatus::Eligible,
            status_reason: String::new(),
            first_seen_at: 0,
            last_seen_at: 0,
            status: crate::domain::distribution::VodStatus::Available,
        }
    }

    fn with_streamer<'a>(vod: &'a Vod) -> VodWithStreamer<'a> {
        VodWithStreamer {
            vod,
            streamer_display_name: "Sampler",
            streamer_login: "sampler",
        }
    }

    /// Walk `path.components()` to return the OS-native
    /// display of each component. Keeps tests free of `/` vs `\`
    /// concerns.
    fn components(path: &std::path::Path) -> Vec<String> {
        path.components()
            .filter_map(|c| c.as_os_str().to_str().map(str::to_owned))
            .collect()
    }

    #[test]
    fn plex_path_has_season_folder_and_nfo_sidecar() {
        let vod = vod_fixture();
        let v = with_streamer(&vod);
        let layout = PlexLayout;
        let path = layout.path_for(&v);
        let parts = components(&path);
        assert_eq!(parts.first().map(String::as_str), Some("Sampler"));
        assert!(parts.iter().any(|p| p == "Season 2026-04"), "got {parts:?}");
        let filename = parts.last().unwrap();
        assert!(filename.contains("[twitch-v1]"), "got {filename}");
        assert!(filename.ends_with(".mp4"));
        let sidecars = layout.sidecars_for(&v);
        assert_eq!(sidecars.len(), 2);
        assert!(sidecars.iter().any(|s| s.kind == SidecarKind::Nfo));
        assert!(sidecars.iter().any(|s| s.kind == SidecarKind::Thumbnail));
    }

    #[test]
    fn flat_path_uses_login_slug() {
        let vod = vod_fixture();
        let v = with_streamer(&vod);
        let layout = FlatLayout;
        let path = layout.path_for(&v);
        let parts = components(&path);
        assert_eq!(parts.first().map(String::as_str), Some("sampler"));
        let filename = parts.last().unwrap();
        assert!(filename.contains("2026-04-02_v1_"), "got {filename}");
        assert!(filename.ends_with(".mp4"));
        assert!(layout.sidecars_for(&v).is_empty());
        // Thumbnail lives under `.thumbs/` — check via components
        // rather than a `/.thumbs/` substring so the test passes on
        // Windows where the separator is `\`.
        let thumb_parts = components(&layout.thumbnail_path(&v));
        assert!(
            thumb_parts.iter().any(|p| p == ".thumbs"),
            "got {thumb_parts:?}"
        );
    }

    #[test]
    fn strips_illegal_chars_in_title() {
        let mut vod = vod_fixture();
        vod.title = "bad/title:with*chars?".into();
        let v = with_streamer(&vod);
        let path = PlexLayout.path_for(&v);
        let s = path.to_string_lossy();
        assert!(!s.contains(':'));
        assert!(!s.contains('?'));
        assert!(!s.contains('*'));
    }

    #[test]
    fn kind_roundtrips_db_strings() {
        assert_eq!(
            LibraryLayoutKind::from_db_str("plex"),
            Some(LibraryLayoutKind::Plex)
        );
        assert_eq!(
            LibraryLayoutKind::from_db_str("flat"),
            Some(LibraryLayoutKind::Flat)
        );
        assert_eq!(LibraryLayoutKind::from_db_str("other"), None);
    }

    #[test]
    fn round_trip_layout_switch_yields_different_paths() {
        // Same input produces different paths under plex vs flat —
        // sanity that our layout-migration code actually has work to
        // do when the user flips the setting.
        let vod = vod_fixture();
        let v = with_streamer(&vod);
        let plex_path = PlexLayout.path_for(&v);
        let flat_path = FlatLayout.path_for(&v);
        assert_ne!(plex_path, flat_path);
    }

    #[test]
    fn preview_frame_paths_colocate_with_thumbnail_plex() {
        let vod = vod_fixture();
        let v = with_streamer(&vod);
        let layout = PlexLayout;
        let frames = layout.preview_frame_paths(&v);
        assert_eq!(frames.len(), PREVIEW_FRAME_COUNT);
        let thumb = layout.thumbnail_path(&v);
        let thumb_dir = thumb.parent().unwrap();
        // Every frame shares the thumbnail directory.
        for f in &frames {
            assert_eq!(f.parent().unwrap(), thumb_dir);
            let name = f.file_name().and_then(|s| s.to_str()).unwrap();
            assert!(name.contains("-preview-"), "got {name}");
            assert!(name.ends_with(".jpg"), "got {name}");
        }
        // Index suffixes are 01..06 and distinct.
        let mut suffixes: Vec<String> = frames
            .iter()
            .filter_map(|p| p.file_name().and_then(|s| s.to_str()).map(|s| s.to_owned()))
            .collect();
        suffixes.sort();
        let suffixes_unique: std::collections::HashSet<_> = suffixes.iter().cloned().collect();
        assert_eq!(suffixes_unique.len(), PREVIEW_FRAME_COUNT);
    }

    #[test]
    fn preview_frame_paths_colocate_with_thumbnail_flat() {
        // Flat layout's thumbnail sits under `.thumbs/`; previews
        // should land there too rather than spilling into the video
        // folder.
        let vod = vod_fixture();
        let v = with_streamer(&vod);
        let layout = FlatLayout;
        let frames = layout.preview_frame_paths(&v);
        assert_eq!(frames.len(), PREVIEW_FRAME_COUNT);
        let thumb_dir = layout.thumbnail_path(&v).parent().unwrap().to_path_buf();
        for f in &frames {
            assert_eq!(f.parent().unwrap(), thumb_dir);
        }
        let parts = components(&frames[0]);
        assert!(parts.iter().any(|p| p == ".thumbs"), "got {parts:?}");
    }
}
