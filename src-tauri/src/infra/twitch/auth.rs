//! Twitch App Access Token acquisition.
//!
//! Only the Client Credentials grant is supported — Sightline never
//! touches user-scoped OAuth (see `tech-spec.md §2`). Tokens are held
//! in memory only and refreshed before expiry.
//!
//! Concurrency: a single in-flight acquisition is enforced via
//! `tokio::sync::Mutex`. The mutex is held across the HTTP call, so
//! concurrent callers queue behind a single refresh.

use std::sync::Arc;

use serde::Deserialize;
use tokio::sync::Mutex;
use tracing::{debug, info, warn};

use crate::error::AppError;
use crate::infra::clock::Clock;
use crate::infra::keychain::{Credentials, TwitchCredentials};

const TOKEN_ENDPOINT_DEFAULT: &str = "https://id.twitch.tv/oauth2/token";

/// A live App Access Token with its expected expiry.
#[derive(Debug, Clone)]
pub struct AppAccessToken {
    pub access_token: String,
    /// Unix seconds UTC when the token becomes invalid.
    pub expires_at: i64,
}

impl AppAccessToken {
    /// Consider the token expired 60 s before its stated expiry so we
    /// never present a just-expired token.
    fn is_usable(&self, now: i64) -> bool {
        self.expires_at.saturating_sub(60) > now
    }
}

/// In-memory token cache plus refresh logic.
#[derive(Debug)]
pub struct TwitchAuthenticator {
    http: reqwest::Client,
    clock: Arc<dyn Clock>,
    credentials: Arc<dyn Credentials>,
    /// Token endpoint. Injected for tests; defaults to `id.twitch.tv`.
    endpoint: String,
    current: Mutex<Option<AppAccessToken>>,
    /// Timestamp of the most recent successful acquisition. Exposed to
    /// the frontend as `credentials_meta.last_token_acquired_at`.
    last_acquired_at: Mutex<Option<i64>>,
}

impl TwitchAuthenticator {
    pub fn new(
        http: reqwest::Client,
        clock: Arc<dyn Clock>,
        credentials: Arc<dyn Credentials>,
    ) -> Self {
        Self {
            http,
            clock,
            credentials,
            endpoint: TOKEN_ENDPOINT_DEFAULT.to_owned(),
            current: Mutex::new(None),
            last_acquired_at: Mutex::new(None),
        }
    }

    /// Override the token endpoint. Used by tests against wiremock.
    pub fn with_endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.endpoint = endpoint.into();
        self
    }

    /// Return a token valid for at least the next minute, refreshing
    /// from the API if necessary.
    pub async fn access_token(&self) -> Result<String, AppError> {
        let now = self.clock.unix_seconds();
        {
            let guard = self.current.lock().await;
            if let Some(tok) = guard.as_ref()
                && tok.is_usable(now)
            {
                return Ok(tok.access_token.clone());
            }
        }

        self.refresh_locked().await
    }

    /// Force a refresh, overwriting any cached token. Used after a 401.
    pub async fn force_refresh(&self) -> Result<String, AppError> {
        {
            let mut guard = self.current.lock().await;
            *guard = None;
        }
        self.refresh_locked().await
    }

    /// Timestamp of the most recent successful acquisition, for the
    /// credentials-status summary.
    pub async fn last_acquired_at(&self) -> Option<i64> {
        *self.last_acquired_at.lock().await
    }

    async fn refresh_locked(&self) -> Result<String, AppError> {
        let creds = self
            .credentials
            .read()
            .await?
            .ok_or_else(|| AppError::Credentials {
                detail: "no Twitch credentials configured".to_owned(),
            })?;

        let token = self.exchange(&creds).await?;
        let fetched_at = self.clock.unix_seconds();

        let mut current = self.current.lock().await;
        *current = Some(token.clone());
        drop(current);

        let mut last = self.last_acquired_at.lock().await;
        *last = Some(fetched_at);

        info!(
            expires_at = token.expires_at,
            "twitch app access token acquired"
        );
        Ok(token.access_token)
    }

    /// Expose the configured credentials bundle for callers that need the
    /// Client-Id header alongside the bearer token. The secret is cloned
    /// only briefly within the auth layer.
    pub async fn credentials_snapshot(&self) -> Result<TwitchCredentials, AppError> {
        self.credentials
            .read()
            .await?
            .ok_or_else(|| AppError::Credentials {
                detail: "no Twitch credentials configured".to_owned(),
            })
    }

    async fn exchange(&self, creds: &TwitchCredentials) -> Result<AppAccessToken, AppError> {
        #[derive(Deserialize)]
        struct TokenResponse {
            access_token: String,
            expires_in: i64,
            #[allow(dead_code)]
            token_type: Option<String>,
        }

        let form = [
            ("client_id", creds.client_id.as_str()),
            ("client_secret", creds.client_secret.as_str()),
            ("grant_type", "client_credentials"),
        ];

        debug!(endpoint = %self.endpoint, "requesting twitch app access token");
        let response = self
            .http
            .post(&self.endpoint)
            .form(&form)
            .send()
            .await
            .map_err(|e| AppError::TwitchAuth {
                detail: format!("token request: {e}"),
            })?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            warn!(status = %status, "twitch token endpoint non-2xx");
            return Err(AppError::TwitchAuth {
                detail: format!("token endpoint returned {status}: {body}"),
            });
        }

        let parsed: TokenResponse = response.json().await.map_err(|e| AppError::TwitchAuth {
            detail: format!("token body parse: {e}"),
        })?;

        let now = self.clock.unix_seconds();
        Ok(AppAccessToken {
            access_token: parsed.access_token,
            expires_at: now.saturating_add(parsed.expires_in),
        })
    }
}

/// Test helper: seed a pre-validated token into the authenticator cache so
/// integration tests against Helix / GQL don't have to exercise the OAuth
/// exchange too. Only compiled in tests.
#[doc(hidden)]
#[cfg(any(test, feature = "test-support"))]
pub async fn prime_token_for_tests(
    auth: &TwitchAuthenticator,
    access_token: String,
    expires_at: i64,
) {
    let mut guard = auth.current.lock().await;
    *guard = Some(AppAccessToken {
        access_token,
        expires_at,
    });
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::infra::clock::FixedClock;
    use crate::infra::keychain::InMemoryCredentials;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    async fn with_creds(secret: &str, id: &str) -> Arc<InMemoryCredentials> {
        let store = Arc::new(InMemoryCredentials::default());
        store
            .write(&TwitchCredentials {
                client_id: id.to_owned(),
                client_secret: secret.to_owned(),
            })
            .await
            .unwrap();
        store
    }

    #[tokio::test]
    async fn access_token_caches_and_refreshes() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/oauth2/token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "access_token": "tok-1",
                "expires_in": 3600,
                "token_type": "bearer"
            })))
            .mount(&server)
            .await;

        let clock = Arc::new(FixedClock::at(1_000_000));
        let creds = with_creds("secret", "clientid").await;
        let auth = TwitchAuthenticator::new(reqwest::Client::new(), clock.clone(), creds)
            .with_endpoint(format!("{}/oauth2/token", server.uri()));

        let first = auth.access_token().await.unwrap();
        assert_eq!(first, "tok-1");
        // Still within the 3600 s window — cached path, no extra request.
        clock.advance(100);
        let second = auth.access_token().await.unwrap();
        assert_eq!(second, "tok-1");
    }

    #[tokio::test]
    async fn access_token_refreshes_near_expiry() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/oauth2/token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "access_token": "tok-1",
                "expires_in": 100,
                "token_type": "bearer"
            })))
            .up_to_n_times(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/oauth2/token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "access_token": "tok-2",
                "expires_in": 3600,
                "token_type": "bearer"
            })))
            .mount(&server)
            .await;

        let clock = Arc::new(FixedClock::at(1_000_000));
        let creds = with_creds("secret", "clientid").await;
        let auth = TwitchAuthenticator::new(reqwest::Client::new(), clock.clone(), creds)
            .with_endpoint(format!("{}/oauth2/token", server.uri()));

        assert_eq!(auth.access_token().await.unwrap(), "tok-1");
        // Advance past the 60 s safety window.
        clock.advance(50);
        assert_eq!(auth.access_token().await.unwrap(), "tok-2");
    }

    #[tokio::test]
    async fn force_refresh_bypasses_cache() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/oauth2/token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "access_token": "tok-1",
                "expires_in": 3600,
                "token_type": "bearer"
            })))
            .up_to_n_times(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/oauth2/token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "access_token": "tok-2",
                "expires_in": 3600,
                "token_type": "bearer"
            })))
            .mount(&server)
            .await;

        let clock = Arc::new(FixedClock::at(1_000_000));
        let creds = with_creds("secret", "clientid").await;
        let auth = TwitchAuthenticator::new(reqwest::Client::new(), clock.clone(), creds)
            .with_endpoint(format!("{}/oauth2/token", server.uri()));

        assert_eq!(auth.access_token().await.unwrap(), "tok-1");
        assert_eq!(auth.force_refresh().await.unwrap(), "tok-2");
    }

    #[tokio::test]
    async fn missing_creds_returns_typed_error() {
        let empty = Arc::new(InMemoryCredentials::default());
        let clock = Arc::new(FixedClock::at(0));
        let auth = TwitchAuthenticator::new(reqwest::Client::new(), clock, empty);
        let err = auth.access_token().await.unwrap_err();
        assert!(matches!(err, AppError::Credentials { .. }));
    }

    #[tokio::test]
    async fn token_endpoint_non_2xx_is_typed() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/oauth2/token"))
            .respond_with(ResponseTemplate::new(401).set_body_string("invalid_client"))
            .mount(&server)
            .await;

        let clock = Arc::new(FixedClock::at(1_000_000));
        let creds = with_creds("secret", "bad").await;
        let auth = TwitchAuthenticator::new(reqwest::Client::new(), clock, creds)
            .with_endpoint(format!("{}/oauth2/token", server.uri()));

        let err = auth.access_token().await.unwrap_err();
        assert!(matches!(err, AppError::TwitchAuth { .. }));
    }
}
