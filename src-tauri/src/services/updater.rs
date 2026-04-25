//! GitHub Releases update checker (Phase 7, ADR-0026).
//!
//! Privacy-first: opt-in via `app_settings.update_check_enabled`,
//! Default off.  Once enabled, fires at most once per 24 h via the
//! tray daemon tick, persisting `update_check_last_run` to enforce
//! the gate across daemon restarts.
//!
//! No telemetry: the only outbound call is a single GET to
//! `https://api.github.com/repos/<owner>/<repo>/releases/latest` with
//! a static `User-Agent: Sightline/<version>` header.  No request
//! body, no Authorization, no cookies, no client identification
//! beyond the binary's compile-time version string.

use std::sync::Arc;
use std::time::Duration;

use semver::Version;
use serde::{Deserialize, Serialize};
use specta::Type;
use tokio::sync::RwLock;
use tracing::{debug, warn};

use crate::error::AppError;
use crate::infra::clock::Clock;
use crate::services::settings::SettingsService;

/// GitHub Releases endpoint Sightline pulls from.  Constant rather
/// than user-configurable: changing it is a release event, not a
/// runtime setting.
const RELEASES_URL: &str = "https://api.github.com/repos/kinsar-rasnik/sightline/releases/latest";

/// Hard cap on response read size — a malicious or weirdly-large
/// release body would otherwise eat the parse budget.  64 KB is
/// orders of magnitude beyond anything we expect.
const MAX_RESPONSE_BYTES: usize = 64 * 1024;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

/// 24 h between scheduled checks; user-tunable via the manual
/// "Check now" button in Settings (which passes `force: true`).
const CHECK_INTERVAL_SECONDS: i64 = 86_400;

/// Surfaced to the renderer.  `version` is normalised (no leading
/// `v`) so the UI can compare against `process.env.npm_package_version`
/// equivalents directly.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub version: String,
    pub release_url: String,
    pub body: String,
    pub published_at: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct UpdateStatus {
    pub enabled: bool,
    pub current_version: String,
    pub last_checked_at: Option<i64>,
    pub skip_version: Option<String>,
    pub available: Option<UpdateInfo>,
}

#[derive(Debug, Clone)]
pub enum UpdaterEvent {
    UpdateAvailable(UpdateInfo),
    CheckFailed { reason: String },
}

pub type UpdaterEventSink = Arc<dyn Fn(UpdaterEvent) + Send + Sync>;

#[derive(Debug)]
pub struct UpdaterService {
    http: reqwest::Client,
    settings: SettingsService,
    clock: Arc<dyn Clock>,
    /// Most-recent newer-than-current `UpdateInfo` seen by any check
    /// since the daemon started.  In-memory only — a fresh process
    /// re-fetches on the first scheduled tick.  See ADR-0026 §UI.
    cache: Arc<RwLock<Option<UpdateInfo>>>,
    /// The version string the binary advertises.  Read from
    /// `CARGO_PKG_VERSION` at compile time so the channel under test
    /// matches the channel the user shipped.
    current_version: String,
}

impl UpdaterService {
    pub fn new(http: reqwest::Client, settings: SettingsService, clock: Arc<dyn Clock>) -> Self {
        Self::new_with_version(http, settings, clock, env!("CARGO_PKG_VERSION").to_string())
    }

    pub fn new_with_version(
        http: reqwest::Client,
        settings: SettingsService,
        clock: Arc<dyn Clock>,
        current_version: String,
    ) -> Self {
        Self {
            http,
            settings,
            clock,
            cache: Arc::new(RwLock::new(None)),
            current_version,
        }
    }

    /// User-facing summary used by the Settings page to avoid an
    /// extra GET.  Does **not** call out to the network — pure read
    /// of `app_settings` plus the cached most-recent fetched info.
    pub async fn get_status(&self) -> Result<UpdateStatus, AppError> {
        let settings = self.settings.get().await?;
        let cache = self.cache.read().await.clone();
        Ok(UpdateStatus {
            enabled: settings.update_check_enabled,
            current_version: self.current_version.clone(),
            last_checked_at: settings.update_check_last_run,
            skip_version: settings.update_check_skip_version.clone(),
            available: cache,
        })
    }

    /// Check the GitHub Releases API for a newer version.
    ///
    /// `force = false` honours the once-per-24h gate (used by the
    /// scheduled tick).  `force = true` bypasses it (used by the
    /// "Check now" button in Settings).  The scheduled tick wraps
    /// this in a swallowing `if let Err(e) = ...` — the manual path
    /// surfaces errors via `updater:check_failed`.
    pub async fn check_for_update(&self, force: bool) -> Result<Option<UpdateInfo>, AppError> {
        let settings = self.settings.get().await?;
        if !force && !settings.update_check_enabled {
            return Ok(None);
        }
        let now = self.clock.unix_seconds();
        if !force
            && let Some(last) = settings.update_check_last_run
            && now - last < CHECK_INTERVAL_SECONDS
        {
            return Ok(None);
        }

        let outcome = self.fetch_and_compare().await;
        // Always record the run timestamp — succeeded or failed —
        // so a network-down day doesn't burn a user's once-per-day
        // budget on retries.
        if let Err(e) = self.settings.record_update_check_run(now).await {
            warn!(error = ?e, "record_update_check_run failed");
        }

        let info = outcome?;
        if let Some(ref candidate) = info {
            if Some(&candidate.version) == settings.update_check_skip_version.as_ref() {
                debug!(version = %candidate.version, "skip-version matched");
                return Ok(None);
            }
            *self.cache.write().await = Some(candidate.clone());
        }
        Ok(info)
    }

    /// Tray-daemon tick wrapper: calls `check_for_update(false)`,
    /// fans out the resulting event, swallows network errors so a
    /// flaky API does not pollute the system log.
    pub async fn schedule_tick(&self, sink: &UpdaterEventSink) {
        match self.check_for_update(false).await {
            Ok(Some(info)) => {
                (sink)(UpdaterEvent::UpdateAvailable(info));
            }
            Ok(None) => {}
            Err(err) => {
                warn!(error = ?err, "scheduled update check failed");
            }
        }
    }

    /// Persist the user's "skip this version" choice.  Empty string
    /// clears any prior skip — used by the Settings UI's "Don't
    /// skip" inverse action.
    pub async fn skip_version(&self, version: String) -> Result<(), AppError> {
        self.settings
            .update(crate::services::settings::SettingsPatch {
                update_check_skip_version: Some(version),
                ..Default::default()
            })
            .await?;
        Ok(())
    }

    async fn fetch_and_compare(&self) -> Result<Option<UpdateInfo>, AppError> {
        let body_text = self.fetch_release_body().await?;
        let release: ReleaseEnvelope =
            serde_json::from_str(&body_text).map_err(|e| AppError::UpdateCheck {
                detail: format!("parse: {e}"),
            })?;
        compare_release(&self.current_version, release)
            .map_err(|err| AppError::UpdateCheck { detail: err })
    }

    async fn fetch_release_body(&self) -> Result<String, AppError> {
        let resp = self
            .http
            .get(RELEASES_URL)
            .header(
                reqwest::header::USER_AGENT,
                user_agent(&self.current_version),
            )
            .header(reqwest::header::ACCEPT, "application/vnd.github+json")
            .timeout(REQUEST_TIMEOUT)
            .send()
            .await
            .map_err(|e| AppError::UpdateCheck {
                detail: format!("request: {e}"),
            })?;
        if !resp.status().is_success() {
            return Err(AppError::UpdateCheck {
                detail: format!("github api status {}", resp.status().as_u16()),
            });
        }
        // Stream-cap: a malicious or oversized body must never cause
        // us to allocate beyond the documented `MAX_RESPONSE_BYTES`.
        // `Response::chunk()` pulls one chunk at a time without
        // requiring the `stream` reqwest feature; we bail the moment
        // the running total would cross the cap, before extending
        // the buffer.
        let mut resp = resp;
        let mut buf: Vec<u8> = Vec::new();
        while let Some(chunk) = resp.chunk().await.map_err(|e| AppError::UpdateCheck {
            detail: format!("read body: {e}"),
        })? {
            if buf.len().saturating_add(chunk.len()) > MAX_RESPONSE_BYTES {
                return Err(AppError::UpdateCheck {
                    detail: format!("release body too large: > {MAX_RESPONSE_BYTES} bytes"),
                });
            }
            buf.extend_from_slice(&chunk);
        }
        String::from_utf8(buf).map_err(|e| AppError::UpdateCheck {
            detail: format!("utf8: {e}"),
        })
    }
}

fn user_agent(current_version: &str) -> String {
    format!("Sightline/{current_version}")
}

#[derive(Debug, Deserialize)]
struct ReleaseEnvelope {
    tag_name: String,
    body: Option<String>,
    html_url: String,
    published_at: Option<String>,
}

/// Pure half of `check_for_update` — exercised in unit tests with a
/// hand-built `ReleaseEnvelope`.
fn compare_release(current: &str, release: ReleaseEnvelope) -> Result<Option<UpdateInfo>, String> {
    let stripped = release
        .tag_name
        .strip_prefix('v')
        .unwrap_or(&release.tag_name);
    let latest =
        Version::parse(stripped).map_err(|e| format!("tag '{}': {e}", release.tag_name))?;
    let current = Version::parse(current).map_err(|e| format!("current '{current}': {e}"))?;
    if latest <= current {
        return Ok(None);
    }
    let published_at = release
        .published_at
        .as_deref()
        .and_then(parse_iso8601_to_unix_seconds);
    Ok(Some(UpdateInfo {
        version: stripped.to_owned(),
        release_url: release.html_url,
        body: release.body.unwrap_or_default(),
        published_at,
    }))
}

fn parse_iso8601_to_unix_seconds(s: &str) -> Option<i64> {
    // GitHub returns RFC 3339 timestamps like "2026-04-25T12:34:56Z".
    // chrono is already a workspace dep — use it rather than rolling
    // a hand parser.
    chrono::DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|d| d.timestamp())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    fn release(tag: &str, body: &str) -> ReleaseEnvelope {
        ReleaseEnvelope {
            tag_name: tag.into(),
            body: Some(body.into()),
            html_url: format!("https://github.com/owner/repo/releases/tag/{tag}"),
            published_at: Some("2026-04-25T12:00:00Z".into()),
        }
    }

    #[test]
    fn newer_release_returns_info_with_normalised_version() {
        let out = compare_release("1.0.0", release("v1.0.1", "fix bug")).unwrap();
        let info = out.expect("expected newer release");
        assert_eq!(info.version, "1.0.1");
        assert_eq!(info.body, "fix bug");
        assert!(info.published_at.is_some());
    }

    #[test]
    fn equal_release_returns_none() {
        assert!(
            compare_release("1.0.0", release("v1.0.0", ""))
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn older_release_returns_none() {
        assert!(
            compare_release("1.5.0", release("v1.4.9", ""))
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn pre_release_ranks_below_release() {
        // semver pre-release: 1.0.0-rc.1 < 1.0.0
        let out = compare_release("1.0.0", release("v1.0.0-rc.2", "")).unwrap();
        assert!(out.is_none(), "pre-release should not flag as 'newer'");
        // ...and the inverse: a release after rc shows up as newer.
        let out = compare_release("1.0.0-rc.1", release("v1.0.0", "")).unwrap();
        assert!(out.is_some());
    }

    #[test]
    fn invalid_tag_returns_error_string() {
        let err = compare_release("1.0.0", release("not-a-version", "")).unwrap_err();
        assert!(err.contains("tag 'not-a-version'"));
    }

    #[test]
    fn invalid_current_returns_error_string() {
        let err = compare_release("garbage", release("v2.0.0", "")).unwrap_err();
        assert!(err.contains("current 'garbage'"));
    }

    #[test]
    fn missing_v_prefix_still_parses() {
        let out = compare_release("1.0.0", release("1.1.0", "")).unwrap();
        assert!(out.is_some());
    }

    #[test]
    fn parse_iso8601_round_trip() {
        let ts = parse_iso8601_to_unix_seconds("1970-01-01T00:00:00Z").unwrap();
        assert_eq!(ts, 0);
        let ts = parse_iso8601_to_unix_seconds("2026-04-25T12:00:00Z").unwrap();
        assert!(ts > 1_700_000_000);
    }

    #[test]
    fn parse_iso8601_invalid_returns_none() {
        assert!(parse_iso8601_to_unix_seconds("not a date").is_none());
    }

    #[test]
    fn user_agent_carries_version() {
        assert_eq!(user_agent("1.2.3"), "Sightline/1.2.3");
    }
}
