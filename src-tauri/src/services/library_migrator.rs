//! Moves every completed download from the old layout to the new one
//! when the user switches `app_settings.library_layout`.
//!
//! The migrator is a one-shot: the user flips a toggle, we spawn a
//! task, we emit progress events, we stamp a row in
//! `library_migrations`. At most one migration can be `running` at a
//! time — enforced by the unique partial index on
//! `library_migrations(status) WHERE status = 'running'`.

use std::path::PathBuf;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use specta::Type;
use sqlx::Row;

use crate::domain::library_layout::{LibraryLayoutKind, VodWithStreamer, layout as build_layout};
use crate::error::AppError;
use crate::infra::clock::Clock;
use crate::infra::db::Db;
use crate::infra::fs::move_::atomic_move;
use crate::services::vods::VodReadService;

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MigrationRow {
    pub id: i64,
    pub started_at: i64,
    pub finished_at: Option<i64>,
    pub from_layout: LibraryLayoutKind,
    pub to_layout: LibraryLayoutKind,
    pub moved: i64,
    pub errors: i64,
    pub status: String,
}

/// Progress event emitted while a migration runs. The commands layer
/// fans this onto `library:migrating` / `library:migration_completed`
/// / `library:migration_failed`.
#[derive(Debug, Clone)]
pub enum LibraryMigrationEvent {
    Migrating { id: i64, moved: i64, total: i64 },
    Completed { id: i64, moved: i64, errors: i64 },
    Failed { id: i64, reason: String },
}

pub type MigrationSink = Arc<dyn Fn(LibraryMigrationEvent) + Send + Sync>;

pub struct LibraryMigratorService {
    db: Db,
    clock: Arc<dyn Clock>,
    vods: Arc<VodReadService>,
}

impl LibraryMigratorService {
    pub fn new(db: Db, clock: Arc<dyn Clock>, vods: Arc<VodReadService>) -> Self {
        Self { db, clock, vods }
    }

    /// Create a new `running` row and return its id. Fails if another
    /// migration is already running.
    pub async fn begin(
        &self,
        from: LibraryLayoutKind,
        to: LibraryLayoutKind,
    ) -> Result<i64, AppError> {
        let id: i64 = sqlx::query_scalar(
            "INSERT INTO library_migrations (started_at, from_layout, to_layout, status)
             VALUES (?, ?, ?, 'running')
             RETURNING id",
        )
        .bind(self.clock.unix_seconds())
        .bind(from.as_db_str())
        .bind(to.as_db_str())
        .fetch_one(self.db.pool())
        .await
        .map_err(|e| AppError::LibraryMigration {
            detail: format!("begin: {e}"),
        })?;
        Ok(id)
    }

    pub async fn get(&self, id: i64) -> Result<Option<MigrationRow>, AppError> {
        let row = sqlx::query(
            "SELECT id, started_at, finished_at, from_layout, to_layout,
                    moved, errors, status
             FROM library_migrations WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(self.db.pool())
        .await?;
        row.as_ref().map(row_to_migration).transpose()
    }

    pub async fn run(
        self: Arc<Self>,
        id: i64,
        library_root: PathBuf,
        from: LibraryLayoutKind,
        to: LibraryLayoutKind,
        sink: MigrationSink,
    ) -> Result<(), AppError> {
        if from == to {
            self.finish(id, 0, 0, "completed").await?;
            (sink)(LibraryMigrationEvent::Completed {
                id,
                moved: 0,
                errors: 0,
            });
            return Ok(());
        }

        // Every completed download is a candidate.
        let completed_vods = sqlx::query_scalar::<_, String>(
            "SELECT vod_id FROM downloads WHERE state = 'completed'",
        )
        .fetch_all(self.db.pool())
        .await?;
        let total = completed_vods.len() as i64;

        let layout_from = build_layout(from);
        let layout_to = build_layout(to);

        let mut moved = 0i64;
        let mut errors = 0i64;
        for vod_id in &completed_vods {
            let vod = match self.vods.get(vod_id).await {
                Ok(v) => v,
                Err(_) => {
                    errors += 1;
                    continue;
                }
            };
            let view = VodWithStreamer {
                vod: &vod.vod,
                streamer_display_name: &vod.streamer_display_name,
                streamer_login: &vod.streamer_login,
            };
            let src_rel = layout_from.path_for(&view);
            let dst_rel = layout_to.path_for(&view);
            let src = library_root.join(src_rel);
            let dst = library_root.join(&dst_rel);

            if !src.exists() {
                // Already moved (partial migration re-run) or file missing.
                continue;
            }
            match atomic_move(&src, &dst).await {
                Ok(_) => {
                    moved += 1;
                    let _ = sqlx::query("UPDATE downloads SET final_path = ? WHERE vod_id = ?")
                        .bind(dst.display().to_string())
                        .bind(vod_id)
                        .execute(self.db.pool())
                        .await;
                    (sink)(LibraryMigrationEvent::Migrating { id, moved, total });
                }
                Err(_) => errors += 1,
            }
        }

        // Errors are counted in the row but do not fail the overall
        // migration — users can re-run to retry per-file errors, and
        // the destination is either the new layout or untouched.
        self.finish(id, moved, errors, "completed").await?;
        (sink)(LibraryMigrationEvent::Completed { id, moved, errors });
        Ok(())
    }

    async fn finish(&self, id: i64, moved: i64, errors: i64, status: &str) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE library_migrations
             SET finished_at = ?, moved = ?, errors = ?, status = ?
             WHERE id = ?",
        )
        .bind(self.clock.unix_seconds())
        .bind(moved)
        .bind(errors)
        .bind(status)
        .bind(id)
        .execute(self.db.pool())
        .await
        .map_err(|e| AppError::LibraryMigration {
            detail: format!("finish: {e}"),
        })?;
        Ok(())
    }
}

fn row_to_migration(r: &sqlx::sqlite::SqliteRow) -> Result<MigrationRow, AppError> {
    let from: String = r.try_get(3)?;
    let to: String = r.try_get(4)?;
    Ok(MigrationRow {
        id: r.try_get(0)?,
        started_at: r.try_get(1)?,
        finished_at: r.try_get(2)?,
        from_layout: LibraryLayoutKind::from_db_str(&from).unwrap_or(LibraryLayoutKind::Plex),
        to_layout: LibraryLayoutKind::from_db_str(&to).unwrap_or(LibraryLayoutKind::Plex),
        moved: r.try_get(5)?,
        errors: r.try_get(6)?,
        status: r.try_get(7)?,
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::infra::clock::FixedClock;

    async fn svc() -> (Arc<LibraryMigratorService>, Db) {
        let db = Db::open_in_memory().await.unwrap();
        db.migrate().await.unwrap();
        let clock: Arc<dyn Clock> = Arc::new(FixedClock::at(1_000));
        let vods = Arc::new(VodReadService::new(db.clone()));
        (
            Arc::new(LibraryMigratorService::new(db.clone(), clock, vods)),
            db,
        )
    }

    #[tokio::test]
    async fn begin_creates_running_row() {
        let (svc, db) = svc().await;
        let id = svc
            .begin(LibraryLayoutKind::Plex, LibraryLayoutKind::Flat)
            .await
            .unwrap();
        let status: String =
            sqlx::query_scalar("SELECT status FROM library_migrations WHERE id = ?")
                .bind(id)
                .fetch_one(db.pool())
                .await
                .unwrap();
        assert_eq!(status, "running");
    }

    #[tokio::test]
    async fn cannot_begin_two_running_migrations_at_once() {
        let (svc, _db) = svc().await;
        let _ = svc
            .begin(LibraryLayoutKind::Plex, LibraryLayoutKind::Flat)
            .await
            .unwrap();
        let err = svc
            .begin(LibraryLayoutKind::Plex, LibraryLayoutKind::Flat)
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::LibraryMigration { .. }));
    }

    #[tokio::test]
    async fn run_no_op_when_from_equals_to() {
        let (svc, _db) = svc().await;
        let id = svc
            .begin(LibraryLayoutKind::Plex, LibraryLayoutKind::Plex)
            .await
            .unwrap();
        let sink: MigrationSink = Arc::new(|_| {});
        svc.clone()
            .run(
                id,
                PathBuf::from("/tmp"),
                LibraryLayoutKind::Plex,
                LibraryLayoutKind::Plex,
                sink,
            )
            .await
            .unwrap();
        let row = svc.get(id).await.unwrap().unwrap();
        assert_eq!(row.status, "completed");
        assert_eq!(row.moved, 0);
        assert_eq!(row.errors, 0);
        // Unique index now permits a new run.
        let _ = svc
            .begin(LibraryLayoutKind::Plex, LibraryLayoutKind::Flat)
            .await
            .unwrap();
    }
}
