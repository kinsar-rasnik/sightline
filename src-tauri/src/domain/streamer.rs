//! Streamer domain types.
//!
//! `Streamer` mirrors the `streamers` table row. `StreamerSummary` is the
//! enriched value the `cmd_list_streamers` command returns; the service
//! layer composes it from `Streamer` + per-streamer VOD counts and the
//! next-poll ETA. Both types are pure — no I/O, no tokio.

use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use specta::Type;
use thiserror::Error;

/// Canonical Twitch login validation rule. Twitch itself enforces:
/// 4-25 characters, lowercase alphanumeric, and underscores. We are a
/// little more permissive on the lower bound (3 characters) because a
/// handful of legacy accounts predate the 4-char floor.
// `unwrap` is safe here: the regex is a compile-time literal that we've
// verified parses. We cannot use `?` inside a `Lazy::new` closure.
#[allow(clippy::unwrap_used)]
static LOGIN_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"^[A-Za-z0-9_]{3,25}$").unwrap());

#[derive(Debug, Clone, PartialEq, Eq, Error, Serialize, Type)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum LoginValidationError {
    #[error("login is required")]
    Empty,
    #[error("login must be 3-25 characters, alphanumeric or underscore")]
    BadShape,
}

/// Validate and normalize a user-supplied login. Returns the lowercase
/// form suitable for equality and for the unique-active index.
pub fn normalize_login(raw: &str) -> Result<String, LoginValidationError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(LoginValidationError::Empty);
    }
    if !LOGIN_RE.is_match(trimmed) {
        return Err(LoginValidationError::BadShape);
    }
    Ok(trimmed.to_ascii_lowercase())
}

/// Storage-aligned streamer row. The `*_at` fields are unix seconds UTC.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Streamer {
    pub twitch_user_id: String,
    pub login: String,
    pub display_name: String,
    pub profile_image_url: Option<String>,
    pub broadcaster_type: String,
    pub twitch_created_at: i64,
    pub added_at: i64,
    pub deleted_at: Option<i64>,
    pub last_polled_at: Option<i64>,
    pub next_poll_at: Option<i64>,
    pub last_live_at: Option<i64>,
}

impl Streamer {
    pub fn is_active(&self) -> bool {
        self.deleted_at.is_none()
    }
}

/// Frontend-facing summary: the `Streamer` row plus derived fields.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct StreamerSummary {
    pub streamer: Streamer,
    pub vod_count: i64,
    pub eligible_vod_count: i64,
    pub live_now: bool,
    pub next_poll_eta_seconds: Option<i64>,
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn accepts_valid_logins() {
        assert_eq!(normalize_login("Test_User").unwrap(), "test_user");
        assert_eq!(normalize_login("  abc123 ").unwrap(), "abc123");
    }

    #[test]
    fn rejects_empty_login() {
        assert_eq!(normalize_login("   "), Err(LoginValidationError::Empty));
    }

    #[test]
    fn rejects_short_login() {
        assert_eq!(normalize_login("ab"), Err(LoginValidationError::BadShape));
    }

    #[test]
    fn rejects_long_login() {
        let too_long = "a".repeat(26);
        assert_eq!(
            normalize_login(&too_long),
            Err(LoginValidationError::BadShape)
        );
    }

    #[test]
    fn rejects_invalid_chars() {
        assert_eq!(
            normalize_login("invalid-login"),
            Err(LoginValidationError::BadShape)
        );
        assert_eq!(
            normalize_login("spaces here"),
            Err(LoginValidationError::BadShape)
        );
    }

    #[test]
    fn is_active_reflects_deleted_at() {
        let s = Streamer {
            twitch_user_id: "1".into(),
            login: "test".into(),
            display_name: "Test".into(),
            profile_image_url: None,
            broadcaster_type: String::new(),
            twitch_created_at: 0,
            added_at: 0,
            deleted_at: None,
            last_polled_at: None,
            next_poll_at: None,
            last_live_at: None,
        };
        assert!(s.is_active());
        let deleted = Streamer {
            deleted_at: Some(1),
            ..s
        };
        assert!(!deleted.is_active());
    }
}
