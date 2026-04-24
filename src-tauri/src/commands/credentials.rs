//! Credentials commands. The only command layer that writes to the
//! OS keyring — all other paths receive a status object derived from
//! the `credentials_meta` row.

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::AppState;
use crate::error::AppError;
use crate::services::credentials::CredentialsInput;
use crate::services::settings::CredentialsStatus;

/// Input payload for `set_twitch_credentials`. The IPC boundary is
/// the only place the Client Secret crosses from the webview into
/// Rust; any `{:?}` downstream of that would risk logging it, so
/// `Debug` is redacted.
#[derive(Clone, Deserialize, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SetTwitchCredentialsInput {
    pub client_id: String,
    pub client_secret: String,
}

impl std::fmt::Debug for SetTwitchCredentialsInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SetTwitchCredentialsInput")
            .field("client_id_len", &self.client_id.len())
            .field("client_secret", &"[REDACTED]")
            .finish()
    }
}

#[tauri::command]
#[specta::specta]
pub async fn set_twitch_credentials(
    state: tauri::State<'_, AppState>,
    input: SetTwitchCredentialsInput,
) -> Result<CredentialsStatus, AppError> {
    let status = state
        .credentials
        .set(CredentialsInput {
            client_id: input.client_id,
            client_secret: input.client_secret,
        })
        .await?;
    state.emit_credentials_changed(status.configured);
    Ok(status)
}

#[tauri::command]
#[specta::specta]
pub async fn get_twitch_credentials_status(
    state: tauri::State<'_, AppState>,
) -> Result<CredentialsStatus, AppError> {
    state.credentials.status().await
}

#[tauri::command]
#[specta::specta]
pub async fn clear_twitch_credentials(state: tauri::State<'_, AppState>) -> Result<(), AppError> {
    state.credentials.clear().await?;
    state.emit_credentials_changed(false);
    Ok(())
}
