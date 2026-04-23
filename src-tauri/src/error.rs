//! Crate-wide error type for the IPC boundary.
//!
//! Every command returns `Result<T, AppError>`. Lower-level errors from
//! `infra` / `services` are mapped into a variant here so the frontend
//! always sees a small, typed union.

use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error, Serialize)]
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
