//! Twitch Helix REST client.
//!
//! Scope: the `videos`, `users`, and `streams` endpoints. Every Phase 2
//! ingest call goes through the same `request_with_auth` helper so the
//! retry/backoff behaviour is written once.
//!
//! **Rate-limiting.** Helix gives 800 points/min per client. We budget
//! conservatively at 600/min with a `governor::RateLimiter` keyed on
//! the client itself (one limiter per process).
//!
//! **Retries.**
//! * 401 → force a token refresh, then retry once.
//! * 429 → respect `Ratelimit-Reset` (unix seconds, header is authoritative).
//!   Returns `AppError::TwitchRateLimit { retry_after_seconds }` so the
//!   scheduler can re-queue the poll.
//! * 5xx → single retry with a short linear delay, then surface.

use std::num::NonZeroU32;
use std::sync::Arc;
use std::time::Duration;

use governor::clock::DefaultClock;
use governor::state::{InMemoryState, NotKeyed};
use governor::{Quota, RateLimiter};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use crate::error::AppError;
use crate::infra::clock::Clock;
use crate::infra::twitch::auth::TwitchAuthenticator;

const HELIX_BASE_DEFAULT: &str = "https://api.twitch.tv/helix";
/// Conservative cap (Helix allows 800/min).
const RATE_LIMIT_PER_MINUTE: u32 = 600;

/// Subset of the Helix `User` object we store.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct HelixUser {
    pub id: String,
    pub login: String,
    pub display_name: String,
    #[serde(default)]
    pub profile_image_url: Option<String>,
    #[serde(default)]
    pub broadcaster_type: String,
    /// ISO 8601 string from Helix. The service layer parses to unix seconds.
    pub created_at: String,
}

/// Subset of the Helix `Video` object we store.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct HelixVideo {
    pub id: String,
    pub user_id: String,
    #[serde(default)]
    pub stream_id: Option<String>,
    pub title: String,
    #[serde(default)]
    pub description: String,
    pub url: String,
    #[serde(default)]
    pub thumbnail_url: Option<String>,
    /// Wall-clock stream start time (ISO 8601).
    pub created_at: String,
    pub published_at: String,
    pub duration: String,
    #[serde(default)]
    pub view_count: i64,
    #[serde(default)]
    pub language: String,
    /// `archive` | `highlight` | `upload`.
    #[serde(rename = "type")]
    pub kind: String,
    /// `public` | `private`.
    pub viewable: String,
    #[serde(default)]
    pub muted_segments: Option<Vec<HelixMutedSegment>>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct HelixMutedSegment {
    pub duration: i64,
    pub offset: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Paginated<T> {
    pub data: Vec<T>,
    #[serde(default)]
    pub pagination: Pagination,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct Pagination {
    #[serde(default)]
    pub cursor: Option<String>,
}

#[derive(Debug)]
pub struct HelixClient {
    http: reqwest::Client,
    auth: Arc<TwitchAuthenticator>,
    clock: Arc<dyn Clock>,
    base: String,
    limiter: Arc<RateLimiter<NotKeyed, InMemoryState, DefaultClock>>,
}

impl HelixClient {
    pub fn new(
        http: reqwest::Client,
        auth: Arc<TwitchAuthenticator>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        #[allow(clippy::unwrap_used)] // compile-time constant
        let quota = Quota::per_minute(NonZeroU32::new(RATE_LIMIT_PER_MINUTE).unwrap());
        Self {
            http,
            auth,
            clock,
            base: HELIX_BASE_DEFAULT.to_owned(),
            limiter: Arc::new(RateLimiter::direct(quota)),
        }
    }

    pub fn with_base(mut self, base: impl Into<String>) -> Self {
        self.base = base.into();
        self
    }

    /// Look up a user by login. Returns `None` if the user doesn't exist.
    pub async fn get_user_by_login(&self, login: &str) -> Result<Option<HelixUser>, AppError> {
        let url = format!("{}/users?login={}", self.base, urlencoded(login));
        let paged: Paginated<HelixUser> = self.request_with_auth(&url, None).await?;
        Ok(paged.data.into_iter().next())
    }

    /// Fetch archive VODs for a user in one page. Caller is expected to
    /// iterate with cursor-based pagination; the service layer stops on
    /// the first already-seen ID during incremental polls.
    pub async fn list_videos_archive(
        &self,
        user_id: &str,
        cursor: Option<&str>,
        page_size: u32,
    ) -> Result<Paginated<HelixVideo>, AppError> {
        let mut url = format!(
            "{}/videos?user_id={}&type=archive&first={}",
            self.base,
            urlencoded(user_id),
            page_size.clamp(1, 100)
        );
        if let Some(c) = cursor {
            url.push_str(&format!("&after={}", urlencoded(c)));
        }
        self.request_with_auth(&url, None).await
    }

    /// Is the streamer currently broadcasting? We only care about the
    /// presence/absence of a row for the live gate.
    pub async fn is_streamer_live(&self, user_id: &str) -> Result<bool, AppError> {
        let url = format!("{}/streams?user_id={}", self.base, urlencoded(user_id));
        let paged: Paginated<serde_json::Value> = self.request_with_auth(&url, None).await?;
        Ok(!paged.data.is_empty())
    }

    async fn request_with_auth<T>(
        &self,
        url: &str,
        attempt_hint: Option<u32>,
    ) -> Result<T, AppError>
    where
        T: for<'de> Deserialize<'de>,
    {
        self.limiter.until_ready().await;
        let attempt = attempt_hint.unwrap_or(0);

        let token = self.auth.access_token().await?;
        let client_id = self.auth.credentials_snapshot().await?.client_id;

        debug!(%url, attempt, "helix request");
        let resp = self
            .http
            .get(url)
            .bearer_auth(&token)
            .header("Client-Id", &client_id)
            .send()
            .await?;

        let status = resp.status();
        match status {
            s if s.is_success() => {
                let value = resp.json::<T>().await?;
                Ok(value)
            }
            StatusCode::UNAUTHORIZED if attempt == 0 => {
                info!("helix 401 — refreshing token and retrying once");
                self.auth.force_refresh().await?;
                Box::pin(self.request_with_auth::<T>(url, Some(1))).await
            }
            StatusCode::TOO_MANY_REQUESTS => {
                let retry_after =
                    parse_retry_after(resp.headers(), self.clock.unix_seconds()).unwrap_or(30);
                warn!(retry_after, "helix 429 — surfacing to caller");
                Err(AppError::TwitchRateLimit {
                    retry_after_seconds: retry_after,
                })
            }
            StatusCode::NOT_FOUND => Err(AppError::TwitchNotFound {
                detail: url.to_owned(),
            }),
            s if s.is_server_error() && attempt == 0 => {
                warn!(status = %s, "helix 5xx — retrying once after short delay");
                tokio::time::sleep(Duration::from_millis(250)).await;
                Box::pin(self.request_with_auth::<T>(url, Some(1))).await
            }
            other => {
                let body = resp.text().await.unwrap_or_default();
                Err(AppError::TwitchApi {
                    status: other.as_u16(),
                    detail: truncate_for_logs(&body, 500),
                })
            }
        }
    }
}

fn urlencoded(value: &str) -> String {
    // Only a narrow character set (alphanumerics + underscores) comes in for
    // logins and user IDs, so we do a minimal encode here rather than pulling
    // a full querystring crate. The regex in `domain::streamer` already
    // rejects anything that would need escaping.
    value
        .chars()
        .map(|c| match c {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '_' | '-' | '.' | '~' => c.to_string(),
            other => format!("%{:02X}", other as u32),
        })
        .collect()
}

fn truncate_for_logs(s: &str, max_chars: usize) -> String {
    if s.len() <= max_chars {
        s.to_owned()
    } else {
        let mut out = String::with_capacity(max_chars + 3);
        for (i, ch) in s.chars().enumerate() {
            if i >= max_chars {
                out.push_str("...");
                break;
            }
            out.push(ch);
        }
        out
    }
}

fn parse_retry_after(headers: &reqwest::header::HeaderMap, now: i64) -> Option<u32> {
    if let Some(reset) = headers.get("Ratelimit-Reset").and_then(|v| v.to_str().ok())
        && let Ok(ts) = reset.parse::<i64>()
    {
        let delta = (ts - now).max(1);
        return Some(delta.clamp(1, 600) as u32);
    }
    if let Some(retry) = headers
        .get(reqwest::header::RETRY_AFTER)
        .and_then(|v| v.to_str().ok())
        && let Ok(seconds) = retry.parse::<i64>()
    {
        return Some(seconds.clamp(1, 600) as u32);
    }
    None
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::infra::clock::FixedClock;
    use crate::infra::keychain::{Credentials, InMemoryCredentials, TwitchCredentials};
    use wiremock::matchers::{method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    async fn make_client(base: String) -> HelixClient {
        let creds = Arc::new(InMemoryCredentials::default());
        creds
            .write(&TwitchCredentials {
                client_id: "test-client".into(),
                client_secret: "secret".into(),
            })
            .await
            .unwrap();
        let clock = Arc::new(FixedClock::at(1_000_000));
        let auth = Arc::new(
            TwitchAuthenticator::new(reqwest::Client::new(), clock.clone(), creds)
                .with_endpoint("http://127.0.0.1:1/unused".to_owned()),
        );
        // Pre-load a cached token so we never hit the auth endpoint.
        crate::infra::twitch::auth::prime_token_for_tests(
            &auth,
            "test-token".to_owned(),
            clock.unix_seconds() + 3600,
        )
        .await;
        HelixClient::new(reqwest::Client::new(), auth, clock).with_base(base)
    }

    #[tokio::test]
    async fn get_user_returns_parsed_row() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/users"))
            .and(query_param("login", "samplevod"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": [
                    {
                        "id": "123",
                        "login": "samplevod",
                        "display_name": "SampleVOD",
                        "broadcaster_type": "affiliate",
                        "profile_image_url": "https://cdn.example/p.png",
                        "created_at": "2021-02-03T04:05:06Z"
                    }
                ],
                "pagination": {}
            })))
            .mount(&server)
            .await;

        let client = make_client(server.uri()).await;
        let got = client
            .get_user_by_login("samplevod")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(got.id, "123");
        assert_eq!(got.display_name, "SampleVOD");
    }

    #[tokio::test]
    async fn get_user_returns_none_for_unknown() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/users"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": [],
                "pagination": {}
            })))
            .mount(&server)
            .await;

        let client = make_client(server.uri()).await;
        assert!(client.get_user_by_login("ghost").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn list_videos_paginates() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/videos"))
            .and(query_param("user_id", "1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": [sample_video("a"), sample_video("b")],
                "pagination": { "cursor": "next-page" }
            })))
            .mount(&server)
            .await;

        let client = make_client(server.uri()).await;
        let page = client.list_videos_archive("1", None, 20).await.unwrap();
        assert_eq!(page.data.len(), 2);
        assert_eq!(page.pagination.cursor.as_deref(), Some("next-page"));
    }

    #[tokio::test]
    async fn is_streamer_live_detects_running_stream() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/streams"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": [{ "id": "s1" }],
                "pagination": {}
            })))
            .mount(&server)
            .await;

        let client = make_client(server.uri()).await;
        assert!(client.is_streamer_live("1").await.unwrap());
    }

    #[tokio::test]
    async fn rate_limit_is_propagated_as_typed_error() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/users"))
            .respond_with(
                ResponseTemplate::new(429)
                    .insert_header("Ratelimit-Reset", "1000060")
                    .set_body_string("rate limited"),
            )
            .mount(&server)
            .await;

        let client = make_client(server.uri()).await;
        let err = client.get_user_by_login("any").await.unwrap_err();
        match err {
            AppError::TwitchRateLimit {
                retry_after_seconds,
            } => {
                assert!(retry_after_seconds > 0 && retry_after_seconds <= 600);
            }
            other => panic!("expected TwitchRateLimit, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn unauthorized_triggers_one_refresh_then_surfaces() {
        let server = MockServer::start().await;
        // First attempt: 401 → client forces refresh → still 401 (force path)
        Mock::given(method("GET"))
            .and(path("/users"))
            .respond_with(ResponseTemplate::new(401).set_body_string("expired"))
            .mount(&server)
            .await;
        // Force refresh re-asks the auth endpoint — but our auth is primed
        // and "force_refresh" will hit the unused endpoint and fail. We
        // expect the typed error propagation to be TwitchAuth.
        let client = make_client(server.uri()).await;
        let err = client.get_user_by_login("any").await.unwrap_err();
        assert!(matches!(err, AppError::TwitchAuth { .. }));
    }

    #[tokio::test]
    async fn malformed_body_returns_parse_error() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/users"))
            .respond_with(ResponseTemplate::new(200).set_body_string("not-json"))
            .mount(&server)
            .await;

        let client = make_client(server.uri()).await;
        let err = client.get_user_by_login("any").await.unwrap_err();
        // reqwest maps body-parse as `status=None`, so From<reqwest::Error>
        // yields TwitchAuth — that's still typed.
        assert!(matches!(
            err,
            AppError::TwitchAuth { .. } | AppError::TwitchApi { .. }
        ));
    }

    fn sample_video(id: &str) -> serde_json::Value {
        serde_json::json!({
            "id": id,
            "user_id": "1",
            "stream_id": null,
            "title": format!("sample {id}"),
            "description": "",
            "url": format!("https://twitch.tv/videos/{id}"),
            "thumbnail_url": "",
            "created_at": "2026-04-23T22:00:00Z",
            "published_at": "2026-04-24T02:00:00Z",
            "duration": "1h23m45s",
            "view_count": 100,
            "language": "en",
            "type": "archive",
            "viewable": "public"
        })
    }
}
