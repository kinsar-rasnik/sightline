//! OS keyring wrapper for Twitch credentials.
//!
//! Backed by the `keyring` crate, which uses:
//!   * macOS: Security framework / Keychain (`apple-native` feature).
//!   * Windows: Credential Manager (`windows-native`).
//!   * Linux: `libsecret` via Secret Service (`sync-secret-service`).
//!
//! We keep the API abstract (`Credentials` trait) so services never
//! touch the OS keyring directly — they accept `&dyn Credentials`,
//! which lets tests substitute `InMemoryCredentials` without unsafe
//! keyring stubs.
//!
//! **Security posture.**
//! * Values are never logged. Only the presence bit + masked Client ID
//!   derived from the first 4 characters is exposed.
//! * The service name / user are compile-time constants: we do not
//!   accept user-controlled keys, preventing confused-deputy reads.
//! * Linux fallback when no Secret Service is running: the operation
//!   returns `AppError::Credentials`; the frontend is expected to
//!   render a setup warning. Sightline does not write a plaintext
//!   fallback file anywhere on disk.

use std::sync::Mutex;

use async_trait::async_trait;
use keyring::Entry;

use crate::error::AppError;

const SERVICE_NAME: &str = "sightline";
const ACCOUNT_CLIENT_ID: &str = "twitch_client_id";
const ACCOUNT_CLIENT_SECRET: &str = "twitch_client_secret";

/// Opaque bundle of Twitch App credentials. Stored in the keyring,
/// passed by value only within the auth pipeline.
#[derive(Debug, Clone)]
pub struct TwitchCredentials {
    pub client_id: String,
    pub client_secret: String,
}

impl TwitchCredentials {
    /// Produce a user-facing masked summary of the Client ID — first four
    /// characters + an asterisk tail. Never includes any of the secret.
    pub fn client_id_masked(&self) -> String {
        masked(&self.client_id)
    }
}

/// Mask a public identifier for display. Exposes at most 4 leading
/// characters, always pads the masked tail to at least 4 stars.
pub fn masked(s: &str) -> String {
    let visible: String = s.chars().take(4).collect();
    format!("{visible}••••")
}

/// Shared abstraction for reading/writing Twitch credentials.
#[async_trait]
pub trait Credentials: Send + Sync + std::fmt::Debug {
    async fn read(&self) -> Result<Option<TwitchCredentials>, AppError>;
    async fn write(&self, creds: &TwitchCredentials) -> Result<(), AppError>;
    async fn clear(&self) -> Result<(), AppError>;
}

/// Real OS-keyring-backed credentials store.
#[derive(Debug, Default)]
pub struct OsKeychainCredentials;

impl OsKeychainCredentials {
    fn entry(account: &str) -> Result<Entry, AppError> {
        Entry::new(SERVICE_NAME, account).map_err(AppError::from)
    }
}

#[async_trait]
impl Credentials for OsKeychainCredentials {
    async fn read(&self) -> Result<Option<TwitchCredentials>, AppError> {
        tokio::task::spawn_blocking(|| {
            let client_id = match Self::entry(ACCOUNT_CLIENT_ID)?.get_password() {
                Ok(v) => v,
                Err(keyring::Error::NoEntry) => return Ok(None),
                Err(e) => return Err(AppError::from(e)),
            };
            let client_secret = match Self::entry(ACCOUNT_CLIENT_SECRET)?.get_password() {
                Ok(v) => v,
                Err(keyring::Error::NoEntry) => return Ok(None),
                Err(e) => return Err(AppError::from(e)),
            };
            Ok(Some(TwitchCredentials {
                client_id,
                client_secret,
            }))
        })
        .await
        .map_err(|e| AppError::Credentials {
            detail: format!("keyring task join: {e}"),
        })?
    }

    async fn write(&self, creds: &TwitchCredentials) -> Result<(), AppError> {
        let id = creds.client_id.clone();
        let secret = creds.client_secret.clone();
        tokio::task::spawn_blocking(move || -> Result<(), AppError> {
            Self::entry(ACCOUNT_CLIENT_ID)?.set_password(&id)?;
            Self::entry(ACCOUNT_CLIENT_SECRET)?.set_password(&secret)?;
            Ok(())
        })
        .await
        .map_err(|e| AppError::Credentials {
            detail: format!("keyring task join: {e}"),
        })?
    }

    async fn clear(&self) -> Result<(), AppError> {
        tokio::task::spawn_blocking(|| -> Result<(), AppError> {
            for account in [ACCOUNT_CLIENT_ID, ACCOUNT_CLIENT_SECRET] {
                match Self::entry(account)?.delete_credential() {
                    Ok(()) => {}
                    Err(keyring::Error::NoEntry) => {}
                    Err(e) => return Err(AppError::from(e)),
                }
            }
            Ok(())
        })
        .await
        .map_err(|e| AppError::Credentials {
            detail: format!("keyring task join: {e}"),
        })?
    }
}

/// Test double. Holds the credentials in process memory; satisfies
/// the same contract as the OS keyring without platform dependencies.
#[derive(Debug, Default)]
pub struct InMemoryCredentials {
    inner: Mutex<Option<TwitchCredentials>>,
}

#[async_trait]
impl Credentials for InMemoryCredentials {
    async fn read(&self) -> Result<Option<TwitchCredentials>, AppError> {
        let guard = self.inner.lock().map_err(|_| AppError::Credentials {
            detail: "poisoned lock".to_owned(),
        })?;
        Ok(guard.clone())
    }

    async fn write(&self, creds: &TwitchCredentials) -> Result<(), AppError> {
        let mut guard = self.inner.lock().map_err(|_| AppError::Credentials {
            detail: "poisoned lock".to_owned(),
        })?;
        *guard = Some(creds.clone());
        Ok(())
    }

    async fn clear(&self) -> Result<(), AppError> {
        let mut guard = self.inner.lock().map_err(|_| AppError::Credentials {
            detail: "poisoned lock".to_owned(),
        })?;
        *guard = None;
        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn masked_truncates_and_pads() {
        assert_eq!(masked("abcdef"), "abcd••••");
        assert_eq!(masked("ab"), "ab••••");
        assert_eq!(masked(""), "••••");
    }

    #[tokio::test]
    async fn in_memory_write_read_clear() {
        let store = InMemoryCredentials::default();
        assert!(store.read().await.unwrap().is_none());
        store
            .write(&TwitchCredentials {
                client_id: "aaaabbbb".into(),
                client_secret: "supersecret".into(),
            })
            .await
            .unwrap();
        let got = store.read().await.unwrap().unwrap();
        assert_eq!(got.client_id, "aaaabbbb");
        assert_eq!(got.client_id_masked(), "aaaa••••");
        store.clear().await.unwrap();
        assert!(store.read().await.unwrap().is_none());
    }
}
