//! Storage + library-migration commands.

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::AppState;
use crate::domain::library_layout::LibraryLayoutKind;
use crate::error::AppError;
use crate::services::library_migrator::MigrationRow;
use crate::services::storage::{LibraryInfo, StagingInfo};

#[tauri::command]
#[specta::specta]
pub async fn get_staging_info(
    state: tauri::State<'_, AppState>,
) -> Result<StagingInfo, AppError> {
    let settings = state.settings.get().await?;
    state
        .storage
        .staging_info(settings.staging_path.as_deref())
        .await
}

#[tauri::command]
#[specta::specta]
pub async fn get_library_info(
    state: tauri::State<'_, AppState>,
) -> Result<LibraryInfo, AppError> {
    let settings = state.settings.get().await?;
    state
        .storage
        .library_info(settings.library_root.as_deref())
        .await
}

#[derive(Debug, Clone, Deserialize, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct MigrateLibraryInput {
    pub target_layout: LibraryLayoutKind,
}

#[derive(Debug, Clone, Deserialize, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct MigrateLibraryOutput {
    pub migration_id: i64,
}

#[tauri::command]
#[specta::specta]
pub async fn migrate_library(
    state: tauri::State<'_, AppState>,
    input: MigrateLibraryInput,
) -> Result<MigrateLibraryOutput, AppError> {
    let settings = state.settings.get().await?;
    let library_root = settings
        .library_root
        .clone()
        .ok_or(AppError::LibraryMigration {
            detail: "library_root not configured".into(),
        })?;
    let from = settings.library_layout;
    let to = input.target_layout;
    if from == to {
        return Err(AppError::LibraryMigration {
            detail: "target layout matches current layout".into(),
        });
    }
    let id = state.library_migrator.begin(from, to).await?;
    // Run in background — the UI subscribes to events for progress.
    let svc = state.library_migrator.clone();
    let sink = state.library_migration_sink.clone();
    let root = std::path::PathBuf::from(library_root);
    tokio::spawn(async move {
        let _ = svc.run(id, root, from, to, sink).await;
    });
    // Persist the layout choice immediately so subsequent enqueues
    // use the new layout.
    state
        .settings
        .update(crate::services::settings::SettingsPatch {
            library_layout: Some(to),
            ..Default::default()
        })
        .await?;
    Ok(MigrateLibraryOutput { migration_id: id })
}

#[derive(Debug, Clone, Deserialize, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct MigrationIdInput {
    pub migration_id: i64,
}

#[tauri::command]
#[specta::specta]
pub async fn get_migration_status(
    state: tauri::State<'_, AppState>,
    input: MigrationIdInput,
) -> Result<MigrationRow, AppError> {
    state
        .library_migrator
        .get(input.migration_id)
        .await?
        .ok_or(AppError::NotFound)
}
