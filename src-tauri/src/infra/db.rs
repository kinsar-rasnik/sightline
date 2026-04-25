//! SQLite pool + migration runner.
//!
//! The migration pipeline runs at startup, before any other query is issued.
//! A migration failure is fatal — no partial schemas are acceptable.

use std::path::Path;

use sqlx::migrate::Migrator;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};
use sqlx::{Row, SqlitePool};

use crate::error::AppError;

static MIGRATOR: Migrator = sqlx::migrate!("./migrations");

/// Thin wrapper over `SqlitePool` with Sightline-specific pragmas applied.
#[derive(Debug, Clone)]
pub struct Db {
    pool: SqlitePool,
}

impl Db {
    /// Open or create the database at the given path. Applies WAL mode,
    /// `synchronous = NORMAL`, and a 5s busy timeout.
    pub async fn open(path: &Path) -> Result<Self, AppError> {
        let options = SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal)
            .synchronous(SqliteSynchronous::Normal)
            .busy_timeout(std::time::Duration::from_secs(5));

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await?;

        Ok(Self { pool })
    }

    /// In-memory DB — for tests.
    pub async fn open_in_memory() -> Result<Self, AppError> {
        let options = SqliteConnectOptions::new()
            .filename(":memory:")
            .create_if_missing(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await?;
        Ok(Self { pool })
    }

    /// Run pending migrations. Idempotent.
    pub async fn migrate(&self) -> Result<(), AppError> {
        MIGRATOR.run(&self.pool).await?;
        Ok(())
    }

    /// Current schema version (PRAGMA user_version).
    pub async fn schema_version(&self) -> Result<i64, AppError> {
        let row = sqlx::query("PRAGMA user_version")
            .fetch_one(&self.pool)
            .await?;
        let value: i64 = row.try_get(0)?;
        Ok(value)
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    /// Latest migration bumps `PRAGMA user_version` to this value. Keep this
    /// in lock-step with the highest-numbered `migrations/*.sql` file.
    const LATEST_SCHEMA_VERSION: i64 = 15;

    #[tokio::test]
    async fn in_memory_migrates_to_latest() {
        let db = Db::open_in_memory().await.unwrap();
        db.migrate().await.unwrap();
        assert_eq!(db.schema_version().await.unwrap(), LATEST_SCHEMA_VERSION);
    }

    #[tokio::test]
    async fn migrate_is_idempotent() {
        let db = Db::open_in_memory().await.unwrap();
        db.migrate().await.unwrap();
        db.migrate().await.unwrap();
        assert_eq!(db.schema_version().await.unwrap(), LATEST_SCHEMA_VERSION);
    }

    #[tokio::test]
    async fn schema_meta_seed_is_present() {
        let db = Db::open_in_memory().await.unwrap();
        db.migrate().await.unwrap();
        let row = sqlx::query("SELECT value FROM schema_meta WHERE key = 'app_name'")
            .fetch_one(db.pool())
            .await
            .unwrap();
        let name: String = row.try_get(0).unwrap();
        assert_eq!(name, "sightline");
    }

    #[tokio::test]
    async fn phase_2_tables_exist_and_are_empty() {
        let db = Db::open_in_memory().await.unwrap();
        db.migrate().await.unwrap();
        for table in ["streamers", "vods", "chapters", "poll_log"] {
            let count: i64 = sqlx::query_scalar(&format!("SELECT COUNT(*) FROM {table}"))
                .fetch_one(db.pool())
                .await
                .unwrap();
            assert_eq!(count, 0, "{table} should be empty after fresh migrate");
        }
    }

    #[tokio::test]
    async fn default_app_settings_seeded() {
        let db = Db::open_in_memory().await.unwrap();
        db.migrate().await.unwrap();
        let games: String =
            sqlx::query_scalar("SELECT enabled_game_ids_json FROM app_settings WHERE id = 1")
                .fetch_one(db.pool())
                .await
                .unwrap();
        assert!(games.contains("32982"), "expected GTA V in default filter");
    }

    #[tokio::test]
    async fn credentials_meta_starts_unconfigured() {
        let db = Db::open_in_memory().await.unwrap();
        db.migrate().await.unwrap();
        let configured: i64 =
            sqlx::query_scalar("SELECT configured FROM credentials_meta WHERE id = 1")
                .fetch_one(db.pool())
                .await
                .unwrap();
        assert_eq!(configured, 0);
    }
}
