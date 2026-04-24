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
    pub async fn set(&self, action_id: &str, keys: &str) -> Result<(), AppError> {
        if action_id.trim().is_empty() {
            return Err(AppError::InvalidInput {
                detail: "action_id is empty".into(),
            });
        }
        let mut map = self.read_json().await?;
        let keys = keys.trim();
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
}
