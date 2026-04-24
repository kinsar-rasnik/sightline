//! Credentials service — the only place the Twitch Client ID + Secret
//! round-trip between the user and the OS keyring.
//!
//! Security invariants (checked by the security reviewer):
//!
//! * Inputs are validated before they touch the keyring.
//! * Non-empty inputs only; trimmed; length-bounded to stop the user
//!   from pasting megabyte payloads.
//! * After write, the authenticator's cache is force-invalidated so any
//!   running flow picks up the new credentials on its next call.
//! * The masked preview is derived from the Client ID only — never
//!   from the secret.

use std::sync::Arc;

use crate::error::AppError;
use crate::infra::keychain::{Credentials, TwitchCredentials, masked};
use crate::infra::twitch::auth::TwitchAuthenticator;
use crate::services::settings::{CredentialsStatus, SettingsService};

/// User-facing input bundle. Trimmed and validated by `set()`.
#[derive(Debug)]
pub struct CredentialsInput {
    pub client_id: String,
    pub client_secret: String,
}

#[derive(Debug)]
pub struct CredentialsService {
    keychain: Arc<dyn Credentials>,
    auth: Arc<TwitchAuthenticator>,
    settings: SettingsService,
}

impl CredentialsService {
    pub fn new(
        keychain: Arc<dyn Credentials>,
        auth: Arc<TwitchAuthenticator>,
        settings: SettingsService,
    ) -> Self {
        Self {
            keychain,
            auth,
            settings,
        }
    }

    /// Save the credentials into the OS keyring. Writes the safe-to-
    /// persist summary (configured = true, masked Client ID) to the
    /// `credentials_meta` row.
    pub async fn set(&self, input: CredentialsInput) -> Result<CredentialsStatus, AppError> {
        let CredentialsInput {
            client_id,
            client_secret,
        } = input;
        let client_id = client_id.trim().to_owned();
        let client_secret = client_secret.trim().to_owned();
        validate(&client_id, &client_secret)?;

        self.keychain
            .write(&TwitchCredentials {
                client_id: client_id.clone(),
                client_secret,
            })
            .await?;

        // The stale token (if any) must not outlive the credential rotation.
        self.auth.force_refresh().await.ok();

        let status = CredentialsStatus {
            configured: true,
            client_id_masked: Some(masked(&client_id)),
            last_token_acquired_at: self.auth.last_acquired_at().await,
        };
        self.settings.set_credentials_meta(&status).await?;
        Ok(status)
    }

    pub async fn status(&self) -> Result<CredentialsStatus, AppError> {
        let settings = self.settings.get().await?;
        Ok(settings.credentials)
    }

    pub async fn clear(&self) -> Result<(), AppError> {
        self.keychain.clear().await?;
        self.settings
            .set_credentials_meta(&CredentialsStatus {
                configured: false,
                client_id_masked: None,
                last_token_acquired_at: None,
            })
            .await?;
        Ok(())
    }
}

fn validate(client_id: &str, client_secret: &str) -> Result<(), AppError> {
    if client_id.is_empty() || client_secret.is_empty() {
        return Err(AppError::InvalidInput {
            detail: "client_id and client_secret are required".to_owned(),
        });
    }
    // Twitch client IDs are 30-char alphanumeric; secrets are 30 char.
    // Bound generously to allow for any future variations, but reject
    // anything beyond 512 bytes — that is categorically not a Twitch
    // credential and would only happen if the user pasted something
    // else by mistake.
    if client_id.len() > 128 || client_secret.len() > 256 {
        return Err(AppError::InvalidInput {
            detail: "credentials exceed the maximum expected length".to_owned(),
        });
    }
    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::infra::clock::FixedClock;
    use crate::infra::db::Db;
    use crate::infra::keychain::InMemoryCredentials;

    async fn make_service() -> (CredentialsService, Arc<InMemoryCredentials>) {
        let db = Db::open_in_memory().await.unwrap();
        db.migrate().await.unwrap();
        let clock = Arc::new(FixedClock::at(1_000_000));
        let store: Arc<dyn Credentials> = Arc::new(InMemoryCredentials::default());
        // Keep a typed handle for assertions.
        let store_inmem = Arc::new(InMemoryCredentials::default());
        let store_inmem2: Arc<dyn Credentials> = store_inmem.clone();
        let auth = Arc::new(
            TwitchAuthenticator::new(reqwest::Client::new(), clock.clone(), store_inmem2)
                .with_endpoint("http://127.0.0.1:1/unused".to_owned()),
        );
        let settings = SettingsService::new(db, clock);
        (CredentialsService::new(store, auth, settings), store_inmem)
    }

    #[tokio::test]
    async fn set_persists_masked_client_id() {
        let (svc, _) = make_service().await;
        let status = svc
            .set(CredentialsInput {
                client_id: "gta5clientid0000".into(),
                client_secret: "superdupersecretvalue".into(),
            })
            .await
            .unwrap();
        assert!(status.configured);
        assert_eq!(status.client_id_masked.as_deref(), Some("gta5••••"));
    }

    #[tokio::test]
    async fn set_rejects_empty() {
        let (svc, _) = make_service().await;
        let err = svc
            .set(CredentialsInput {
                client_id: "   ".into(),
                client_secret: "nope".into(),
            })
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::InvalidInput { .. }));
    }

    #[tokio::test]
    async fn clear_resets_status() {
        let (svc, _) = make_service().await;
        svc.set(CredentialsInput {
            client_id: "abcd".into(),
            client_secret: "efgh".into(),
        })
        .await
        .unwrap();
        svc.clear().await.unwrap();
        let status = svc.status().await.unwrap();
        assert!(!status.configured);
        assert!(status.client_id_masked.is_none());
    }
}
