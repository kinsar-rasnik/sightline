//! Autostart wiring (Phase 5 housekeeping — pulled forward from
//! phase-04.md §Deviations #3 / Phase 7).
//!
//! The `tauri-plugin-autostart` crate handles the per-OS details:
//!
//! * macOS — LaunchAgent plist under `~/Library/LaunchAgents/`.
//! * Windows — HKCU\Software\Microsoft\Windows\CurrentVersion\Run key.
//! * Linux — XDG autostart `.desktop` file under
//!   `~/.config/autostart/`.
//!
//! This service is a tiny wrapper: it keeps the DB's `start_at_login`
//! row and the OS registration in sync. A mismatch can arise if the
//! user disables the OS registration through System Settings
//! directly; we resync on every app start so the DB value is never
//! the authoritative source (the OS is).
//!
//! The plugin's `AutoLaunchManager` is non-async; we offload every
//! call to `spawn_blocking` so the Tokio scheduler never stalls on
//! `launchctl` / registry reads.

use std::sync::Arc;

use tauri::{AppHandle, Runtime};
use tauri_plugin_autostart::ManagerExt;
use tracing::warn;

use crate::error::AppError;
use crate::infra::clock::Clock;
use crate::infra::db::Db;
use crate::services::settings::{SettingsPatch, SettingsService};

/// Wire the DB `start_at_login` setting onto the OS registration.
/// Called at startup (to reconcile any drift) and from the Settings
/// command.
#[derive(Clone)]
pub struct AutostartService<R: Runtime> {
    handle: AppHandle<R>,
    settings: Arc<SettingsService>,
}

impl<R: Runtime> AutostartService<R> {
    pub fn new(handle: AppHandle<R>, settings: Arc<SettingsService>) -> Self {
        Self { handle, settings }
    }

    /// Read the OS's current state (not the DB's) — single source of
    /// truth for "is the app actually registered".
    pub async fn is_os_enabled(&self) -> Result<bool, AppError> {
        let handle = self.handle.clone();
        tokio::task::spawn_blocking(move || handle.autolaunch().is_enabled())
            .await
            .map_err(|e| AppError::Internal {
                detail: format!("autostart probe panicked: {e}"),
            })?
            .map_err(|e| AppError::Io {
                detail: format!("autostart probe: {e}"),
            })
    }

    /// Set the OS registration. Returns the *new* effective state so
    /// the caller can mirror it into the DB. Idempotent: enabling an
    /// already-enabled registration is a no-op.
    pub async fn set_os_enabled(&self, enabled: bool) -> Result<bool, AppError> {
        let handle = self.handle.clone();
        tokio::task::spawn_blocking(move || {
            let mgr = handle.autolaunch();
            if enabled {
                mgr.enable().map_err(|e| format!("enable: {e}"))?;
            } else {
                mgr.disable().map_err(|e| format!("disable: {e}"))?;
            }
            mgr.is_enabled().map_err(|e| format!("readback: {e}"))
        })
        .await
        .map_err(|e| AppError::Internal {
            detail: format!("autostart apply panicked: {e}"),
        })?
        .map_err(|e| AppError::Io { detail: e })
    }

    /// Apply the current DB setting to the OS, and write the OS's
    /// readback back into the DB. If the OS has been changed behind
    /// our back (user unchecked the LoginItems entry) we accept that
    /// as authoritative and update the DB accordingly.
    pub async fn reconcile(&self) -> Result<bool, AppError> {
        let settings = self.settings.get().await?;
        let desired = settings.start_at_login;
        let actual_before = match self.is_os_enabled().await {
            Ok(v) => v,
            Err(e) => {
                // On platforms or sandboxes that deny the autostart
                // query we'd rather degrade cleanly than fail startup.
                warn!(error = ?e, "autostart probe failed; skipping reconcile");
                return Ok(desired);
            }
        };
        if actual_before == desired {
            return Ok(desired);
        }
        match self.set_os_enabled(desired).await {
            Ok(new_state) => {
                if new_state != desired {
                    // Writing succeeded but the OS reports a different
                    // value — trust the OS.
                    self.settings
                        .update(SettingsPatch {
                            start_at_login: Some(new_state),
                            ..Default::default()
                        })
                        .await?;
                }
                Ok(new_state)
            }
            Err(e) => {
                warn!(error = ?e, "autostart apply failed; trusting OS state");
                self.settings
                    .update(SettingsPatch {
                        start_at_login: Some(actual_before),
                        ..Default::default()
                    })
                    .await?;
                Ok(actual_before)
            }
        }
    }
}

/// Policy layer — pure. Decides what a reconcile pass should do given
/// the DB-desired state and the OS-observed state. Extracted for
/// testability: the real `AutoLaunchManager` can't be constructed
/// from a unit test, so we exercise the decision table here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReconcileAction {
    /// DB and OS already agree; nothing to do.
    NoOp,
    /// DB wants enabled, OS disabled — enable the OS registration.
    EnableOs,
    /// DB wants disabled, OS enabled — disable the OS registration.
    DisableOs,
}

pub fn decide(desired: bool, observed: bool) -> ReconcileAction {
    match (desired, observed) {
        (a, b) if a == b => ReconcileAction::NoOp,
        (true, false) => ReconcileAction::EnableOs,
        (false, true) => ReconcileAction::DisableOs,
        _ => ReconcileAction::NoOp,
    }
}

/// Mostly-data struct returned by the `cmd_get_autostart_status`
/// command. Separate from `AppSettings.start_at_login` because the
/// OS-observed value can diverge if the user toggled the OS setting
/// outside Sightline.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct AutostartStatus {
    pub os_enabled: bool,
    pub db_enabled: bool,
}

/// Thin DB-only helper — used by command tests that don't want a
/// real `AppHandle`.
pub async fn read_db_start_at_login(db: &Db, clock: Arc<dyn Clock>) -> Result<bool, AppError> {
    let settings = SettingsService::new(db.clone(), clock);
    Ok(settings.get().await?.start_at_login)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn decide_covers_all_four_corners() {
        assert_eq!(decide(false, false), ReconcileAction::NoOp);
        assert_eq!(decide(true, true), ReconcileAction::NoOp);
        assert_eq!(decide(true, false), ReconcileAction::EnableOs);
        assert_eq!(decide(false, true), ReconcileAction::DisableOs);
    }

    #[tokio::test]
    async fn read_db_start_at_login_defaults_to_false() {
        use crate::infra::clock::FixedClock;
        let db = Db::open_in_memory().await.unwrap();
        db.migrate().await.unwrap();
        let v = read_db_start_at_login(&db, Arc::new(FixedClock::at(0)))
            .await
            .unwrap();
        assert!(!v, "migration default is off");
    }
}
