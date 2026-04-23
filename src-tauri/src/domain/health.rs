//! Health-report value type. Shape is canonical for the IPC boundary.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HealthReport {
    /// Short program identifier. Always `"sightline"`.
    pub app_name: String,
    /// Cargo package version (from `CARGO_PKG_VERSION`).
    pub app_version: String,
    /// SQLite `PRAGMA user_version` at the time of the call.
    pub schema_version: i64,
    /// Unix seconds UTC when the process started.
    pub started_at: i64,
    /// Unix seconds UTC when this report was assembled.
    pub checked_at: i64,
}
