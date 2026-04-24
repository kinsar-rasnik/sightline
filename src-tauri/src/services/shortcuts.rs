//! Persist + retrieve user-customised keyboard shortcuts (Phase 4).
//!
//! Storage: a single JSON object in `app_settings.shortcuts_json`.
//! Keys are frontend-defined action IDs (`library`, `timeline`,
//! `focus_search`, …). Values are keystroke strings (e.g. `"g l"`,
//! `"mod+shift+f"`). The backend is deliberately opaque about shape —
//! the frontend owns the defaults and the conflict rules.

use std::collections::BTreeMap;

use sqlx::Row;

use crate::commands::app::Shortcut;
use crate::error::AppError;
use crate::infra::db::Db;

#[derive(Debug)]
pub struct ShortcutsService {
    db: Db,
}

impl ShortcutsService {
    pub fn new(db: Db) -> Self {
        Self { db }
    }

    async fn read_json(&self) -> Result<BTreeMap<String, String>, AppError> {
        let row = sqlx::query("SELECT shortcuts_json FROM app_settings WHERE id = 1")
            .fetch_one(self.db.pool())
            .await?;
        let raw: String = row.try_get(0)?;
        if raw.is_empty() {
            return Ok(BTreeMap::new());
        }
        serde_json::from_str(&raw).map_err(AppError::from)
    }

    async fn write_json(&self, map: &BTreeMap<String, String>) -> Result<(), AppError> {
        let raw = serde_json::to_string(map).map_err(AppError::from)?;
        sqlx::query("UPDATE app_settings SET shortcuts_json = ? WHERE id = 1")
            .bind(&raw)
            .execute(self.db.pool())
            .await?;
        Ok(())
    }

    /// List every customised shortcut. Returns sorted by action id for
    /// stable rendering.
    pub async fn list(&self) -> Result<Vec<Shortcut>, AppError> {
        let map = self.read_json().await?;
        Ok(map
            .into_iter()
            .map(|(action_id, keys)| Shortcut { action_id, keys })
            .collect())
    }

    /// Set a single action_id → keys mapping. An empty `keys` clears
    /// the override (falls back to the frontend default).
    ///
    /// Validates length and character set so a malicious / buggy caller
    /// can't balloon `shortcuts_json` via unbounded strings (tracked
    /// under the Phase 4 security review). `action_id` is an internal
    /// identifier: lowercase ASCII letters, digits, and underscores,
    /// up to 64 chars. `keys` is a keystroke or chord string: printable
    /// ASCII up to 32 chars.
    pub async fn set(&self, action_id: &str, keys: &str) -> Result<(), AppError> {
        let action_id = action_id.trim();
        if action_id.is_empty() {
            return Err(AppError::InvalidInput {
                detail: "action_id is empty".into(),
            });
        }
        if action_id.len() > 64 || !action_id.chars().all(is_valid_action_char) {
            return Err(AppError::InvalidInput {
                detail: "action_id must be 1-64 lowercase-ASCII / digits / underscore".into(),
            });
        }
        let keys = keys.trim();
        if keys.len() > 32 {
            return Err(AppError::InvalidInput {
                detail: "keys must be <= 32 characters".into(),
            });
        }
        if !keys.chars().all(is_valid_key_char) {
            return Err(AppError::InvalidInput {
                detail: "keys must be printable ASCII".into(),
            });
        }
        let mut map = self.read_json().await?;
        if keys.is_empty() {
            map.remove(action_id);
        } else {
            map.insert(action_id.to_string(), keys.to_string());
        }
        self.write_json(&map).await
    }

    /// Reset all overrides.
    pub async fn reset(&self) -> Result<(), AppError> {
        self.write_json(&BTreeMap::new()).await
    }
}

fn is_valid_action_char(c: char) -> bool {
    matches!(c, 'a'..='z' | '0'..='9' | '_')
}

fn is_valid_key_char(c: char) -> bool {
    // Printable ASCII + space + the two modifier separators we use (`+`, ` `).
    (c.is_ascii_graphic() || c == ' ') && !c.is_control()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    async fn fixture() -> ShortcutsService {
        let db = Db::open_in_memory().await.unwrap();
        db.migrate().await.unwrap();
        ShortcutsService::new(db)
    }

    #[tokio::test]
    async fn empty_by_default() {
        let svc = fixture().await;
        assert!(svc.list().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn set_and_list_round_trip() {
        let svc = fixture().await;
        svc.set("library", "g l").await.unwrap();
        svc.set("timeline", "g t").await.unwrap();
        let out = svc.list().await.unwrap();
        assert_eq!(out.len(), 2);
        assert!(
            out.iter()
                .any(|s| s.action_id == "library" && s.keys == "g l")
        );
    }

    #[tokio::test]
    async fn empty_keys_clears() {
        let svc = fixture().await;
        svc.set("library", "g l").await.unwrap();
        svc.set("library", "").await.unwrap();
        assert!(svc.list().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn reset_clears_all() {
        let svc = fixture().await;
        svc.set("library", "g l").await.unwrap();
        svc.set("timeline", "g t").await.unwrap();
        svc.reset().await.unwrap();
        assert!(svc.list().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn rejects_overlong_action_id() {
        let svc = fixture().await;
        let long = "a".repeat(65);
        let err = svc.set(&long, "g l").await.unwrap_err();
        assert!(matches!(err, AppError::InvalidInput { .. }));
    }

    #[tokio::test]
    async fn rejects_overlong_keys() {
        let svc = fixture().await;
        let long_keys = "a".repeat(33);
        let err = svc.set("library", &long_keys).await.unwrap_err();
        assert!(matches!(err, AppError::InvalidInput { .. }));
    }

    #[tokio::test]
    async fn rejects_non_printable_keys() {
        let svc = fixture().await;
        let err = svc.set("library", "g\u{0007}l").await.unwrap_err();
        assert!(matches!(err, AppError::InvalidInput { .. }));
    }

    #[tokio::test]
    async fn rejects_non_snake_action_id() {
        let svc = fixture().await;
        let err = svc.set("Library", "g l").await.unwrap_err();
        assert!(matches!(err, AppError::InvalidInput { .. }));
    }
}
