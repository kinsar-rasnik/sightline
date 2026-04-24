//! User-facing download quality preset and the yt-dlp format selector
//! we hand off to the sidecar. Pure strings; no I/O.
//!
//! The fallback chain: if `cap ≥ requested`, the requested preset
//! stands. If the source is only 720p60 and the user asked for 1080p60,
//! the preset falls back one step (Source → 1080p60 → 720p60 → 480p)
//! until a level is satisfiable. The resolved preset is recorded in
//! `downloads.quality_resolved` for UI transparency.

use serde::{Deserialize, Serialize};
use specta::Type;

/// The four presets exposed on the UI. Wire strings are stable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum QualityPreset {
    /// Best video + best audio, whatever the source offers.
    Source,
    /// 1080p60 or less.
    #[serde(rename = "1080p60")]
    P1080p60,
    /// 720p60 or less.
    #[serde(rename = "720p60")]
    P720p60,
    /// 480p or less, 30fps cap implied.
    #[serde(rename = "480p")]
    P480p,
}

impl QualityPreset {
    pub fn as_db_str(self) -> &'static str {
        match self {
            QualityPreset::Source => "source",
            QualityPreset::P1080p60 => "1080p60",
            QualityPreset::P720p60 => "720p60",
            QualityPreset::P480p => "480p",
        }
    }

    pub fn from_db_str(s: &str) -> Option<Self> {
        Some(match s {
            "source" => QualityPreset::Source,
            "1080p60" => QualityPreset::P1080p60,
            "720p60" => QualityPreset::P720p60,
            "480p" => QualityPreset::P480p,
            _ => return None,
        })
    }

    /// yt-dlp `-f` format selector. Returned as a static string — no
    /// allocation per invocation. The selectors mirror the Phase 3
    /// spec in `docs/implementation-plan.md`.
    pub fn format_selector(self) -> &'static str {
        match self {
            QualityPreset::Source => "bestvideo+bestaudio/best",
            QualityPreset::P1080p60 => {
                "bestvideo[height<=1080][fps<=60]+bestaudio/best[height<=1080][fps<=60]/best"
            }
            QualityPreset::P720p60 => {
                "bestvideo[height<=720][fps<=60]+bestaudio/best[height<=720][fps<=60]/best"
            }
            QualityPreset::P480p => "bestvideo[height<=480]+bestaudio/best[height<=480]/best",
        }
    }

    /// Ordered fallback chain: each call yields the next-weaker preset,
    /// or `None` when already at the bottom. Callers apply this after
    /// probing `yt-dlp --print "%(height)s %(fps)s"` for the source to
    /// pick a preset the source can actually satisfy.
    pub fn weaker(self) -> Option<Self> {
        Some(match self {
            QualityPreset::Source => QualityPreset::P1080p60,
            QualityPreset::P1080p60 => QualityPreset::P720p60,
            QualityPreset::P720p60 => QualityPreset::P480p,
            QualityPreset::P480p => return None,
        })
    }

    /// Height ceiling implied by the preset; `None` for Source. Used
    /// by the resolver to check against the source's `height` field
    /// from yt-dlp's info JSON.
    pub fn max_height(self) -> Option<u32> {
        match self {
            QualityPreset::Source => None,
            QualityPreset::P1080p60 => Some(1080),
            QualityPreset::P720p60 => Some(720),
            QualityPreset::P480p => Some(480),
        }
    }

    /// FPS ceiling. `480p` drops the 60fps requirement; `Source`
    /// imposes none.
    pub fn max_fps(self) -> Option<u32> {
        match self {
            QualityPreset::Source => None,
            QualityPreset::P1080p60 | QualityPreset::P720p60 => Some(60),
            QualityPreset::P480p => Some(30),
        }
    }
}

/// Resolve a requested preset against the source's actual height /
/// fps, falling back one step at a time until the source can satisfy
/// it. Returns the chosen preset. Never panics; the worst case falls
/// through to `P480p`.
pub fn resolve(requested: QualityPreset, source_height: u32, source_fps: u32) -> QualityPreset {
    let mut current = requested;
    loop {
        let height_ok = current
            .max_height()
            .map_or(true, |cap| source_height >= cap);
        let fps_ok = current.max_fps().map_or(true, |cap| source_fps >= cap);
        // If the preset's ceiling is *higher* than the source can
        // provide, we need to downgrade. `Source` always stays —
        // yt-dlp will simply take the best the source has.
        if current == QualityPreset::Source || (height_ok && fps_ok) {
            return current;
        }
        match current.weaker() {
            Some(next) => current = next,
            None => return current,
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn db_string_round_trip() {
        for p in [
            QualityPreset::Source,
            QualityPreset::P1080p60,
            QualityPreset::P720p60,
            QualityPreset::P480p,
        ] {
            assert_eq!(QualityPreset::from_db_str(p.as_db_str()), Some(p));
        }
        assert_eq!(QualityPreset::from_db_str("4k"), None);
    }

    #[test]
    fn fallback_walks_one_step_at_a_time() {
        // 1080p60 request on a 720p stream falls to 720p60.
        assert_eq!(
            resolve(QualityPreset::P1080p60, 720, 60),
            QualityPreset::P720p60
        );
        // 720p60 request on a 480p 30fps stream: 720p60 fails (height),
        // 480p accepts (height OK, fps OK).
        assert_eq!(
            resolve(QualityPreset::P720p60, 480, 30),
            QualityPreset::P480p
        );
    }

    #[test]
    fn source_never_downgrades() {
        // Source always stays Source, even if the numbers look tiny.
        assert_eq!(resolve(QualityPreset::Source, 360, 24), QualityPreset::Source);
    }

    #[test]
    fn exact_match_keeps_preset() {
        assert_eq!(
            resolve(QualityPreset::P1080p60, 1080, 60),
            QualityPreset::P1080p60
        );
        assert_eq!(
            resolve(QualityPreset::P720p60, 720, 60),
            QualityPreset::P720p60
        );
    }

    #[test]
    fn fps_short_still_downgrades() {
        // User asks for 1080p60 on a 1080p30 stream; falls to 720p60
        // (no — wait, 720p60 still requires 60fps, it'd fail too),
        // then to 480p (which requires fps >= 30 — pass).
        assert_eq!(
            resolve(QualityPreset::P1080p60, 1080, 30),
            QualityPreset::P480p
        );
    }

    #[test]
    fn selectors_are_non_empty() {
        for p in [
            QualityPreset::Source,
            QualityPreset::P1080p60,
            QualityPreset::P720p60,
            QualityPreset::P480p,
        ] {
            assert!(!p.format_selector().is_empty());
        }
    }

    #[test]
    fn weaker_chain_terminates() {
        let mut cur = QualityPreset::Source;
        let mut steps = 0;
        while let Some(next) = cur.weaker() {
            cur = next;
            steps += 1;
            assert!(steps < 100, "chain must terminate");
        }
        assert_eq!(cur, QualityPreset::P480p);
    }
}
