//! Crate-wide error type for the IPC boundary.
//!
//! Every command returns `Result<T, AppError>`. Lower-level errors from
//! `infra` / `services` are mapped into a variant here so the frontend
//! always sees a small, typed union.

use serde::Serialize;
use specta::Type;
use thiserror::Error;

/// Typed application error. Serialized with a `kind` tag so the frontend
/// can exhaustively narrow on it via a discriminated union.
#[derive(Debug, Error, Serialize, Type)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AppError {
    #[error("database error: {detail}")]
    Db { detail: String },

    #[error("i/o error: {detail}")]
    Io { detail: String },

    #[error("invalid input: {detail}")]
    InvalidInput { detail: String },

    #[error("not found")]
    NotFound,

    #[error("internal error: {detail}")]
    Internal { detail: String },

    // --- Phase 2: Twitch + credentials + ingest errors. ---
    /// Credentials-related failure (keychain read/write, missing creds, invalid input shape).
    #[error("credentials error: {detail}")]
    Credentials { detail: String },

    /// Twitch App Access Token acquisition / refresh failed.
    #[error("twitch auth error: {detail}")]
    TwitchAuth { detail: String },

    /// Twitch rate limit hit. `retry_after_seconds` is a conservative hint
    /// derived from the `Ratelimit-Reset` header (or a policy floor).
    #[error("twitch rate limited, retry after {retry_after_seconds}s")]
    TwitchRateLimit { retry_after_seconds: u32 },

    /// Generic Twitch Helix API error (non-rate-limit HTTP failure).
    #[error("twitch api error ({status}): {detail}")]
    TwitchApi { status: u16, detail: String },

    /// Helix returned 404 / empty set for a user or video that should exist.
    #[error("twitch resource not found: {detail}")]
    TwitchNotFound { detail: String },

    /// Twitch GraphQL endpoint failure. Separately typed so the UI can
    /// surface "chapters unavailable" differently from Helix failures.
    #[error("twitch gql error: {detail}")]
    TwitchGql { detail: String },

    /// VOD ingest pipeline failure (persistence, chapter merge, etc.).
    #[error("ingest error: {detail}")]
    Ingest { detail: String },

    /// Parser failure — duration, chapter payload, ISO timestamp.
    #[error("parse error: {detail}")]
    Parse { detail: String },
}

impl From<sqlx::Error> for AppError {
    fn from(e: sqlx::Error) -> Self {
        AppError::Db {
            detail: e.to_string(),
        }
    }
}

impl From<sqlx::migrate::MigrateError> for AppError {
    fn from(e: sqlx::migrate::MigrateError) -> Self {
        AppError::Db {
            detail: format!("migrate: {e}"),
        }
    }
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        AppError::Io {
            detail: e.to_string(),
        }
    }
}

impl From<reqwest::Error> for AppError {
    fn from(e: reqwest::Error) -> Self {
        if let Some(status) = e.status() {
            AppError::TwitchApi {
                status: status.as_u16(),
                detail: e.to_string(),
            }
        } else {
            AppError::TwitchAuth {
                detail: e.to_string(),
            }
        }
    }
}

impl From<serde_json::Error> for AppError {
    fn from(e: serde_json::Error) -> Self {
        AppError::Parse {
            detail: e.to_string(),
        }
    }
}

impl From<keyring::Error> for AppError {
    fn from(e: keyring::Error) -> Self {
        AppError::Credentials {
            detail: e.to_string(),
        }
    }
}
