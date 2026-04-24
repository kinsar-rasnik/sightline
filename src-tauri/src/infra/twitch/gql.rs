//! Twitch GraphQL client — scoped narrowly to fetching VOD chapter
//! moments. See ADR-0008 for the rationale and trade-offs.
//!
//! The module is deliberately limited:
//!
//! * Hardcoded endpoint + Client-Id (the public one used by the Twitch
//!   web frontend). No user-controlled URLs enter here.
//! * One operation: `fetch_video_moments(video_id)`.
//! * Two query names, the newer one tried first, the older as fallback.
//! * Defensive deserialization: unknown fields are ignored, missing
//!   arrays become empty, non-success statuses surface as
//!   `AppError::TwitchGql`.

use serde::Deserialize;
use tracing::{debug, warn};

use crate::domain::chapter::{Chapter, ChapterType};
use crate::error::AppError;

/// Public URL. Constants, not parameters — see the ADR's "no user-
/// controlled URLs" rule.
pub const GQL_ENDPOINT: &str = "https://gql.twitch.tv/gql";

/// Public Twitch web Client-Id. Documented in
/// docs/adr/0008-chapters-via-twitch-gql.md. Used by every open-source
/// VOD tool we're aware of (streamlink, twitch-dl, yt-dlp).
pub const PUBLIC_CLIENT_ID: &str = "kimne78kx3ncx6brgo4mv6wki5h1ko";

const QUERY_PRIMARY: &str = "VideoPlayerStreamMetadata";
const QUERY_FALLBACK: &str = "VideoPreviewOverlay_VideoMoments";

/// GQL client. Holds a `reqwest::Client` and, for tests, an overridable
/// endpoint.
#[derive(Debug, Clone)]
pub struct GqlClient {
    http: reqwest::Client,
    endpoint: String,
    client_id: String,
}

impl GqlClient {
    pub fn new(http: reqwest::Client) -> Self {
        Self {
            http,
            endpoint: GQL_ENDPOINT.to_owned(),
            client_id: PUBLIC_CLIENT_ID.to_owned(),
        }
    }

    pub fn with_endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.endpoint = endpoint.into();
        self
    }

    #[cfg(test)]
    pub fn with_client_id(mut self, id: impl Into<String>) -> Self {
        self.client_id = id.into();
        self
    }

    /// Fetch the chapter moments for a VOD. On a defensible failure
    /// (empty data, non-200, GQL errors) returns
    /// `Err(AppError::TwitchGql)`; the caller is expected to downgrade
    /// the VOD to `ingest_status = error` and retry next poll.
    pub async fn fetch_video_moments(&self, video_id: &str) -> Result<Vec<Chapter>, AppError> {
        match self.try_query(video_id, QUERY_PRIMARY).await {
            Ok(chapters) => Ok(chapters),
            Err(AppError::TwitchGql { detail })
                if detail.contains("unknown") || detail.contains("no such") =>
            {
                warn!(detail, "gql primary query rejected — trying fallback");
                self.try_query(video_id, QUERY_FALLBACK).await
            }
            Err(other) => Err(other),
        }
    }

    async fn try_query(
        &self,
        video_id: &str,
        operation_name: &str,
    ) -> Result<Vec<Chapter>, AppError> {
        let body = build_query_body(operation_name, video_id);

        debug!(operation_name, video_id, "gql request");
        let response = self
            .http
            .post(&self.endpoint)
            .header("Client-Id", &self.client_id)
            .json(&body)
            .send()
            .await
            .map_err(|e| AppError::TwitchGql {
                detail: format!("request: {e}"),
            })?;

        let status = response.status();
        if !status.is_success() {
            let text = response.text().await.unwrap_or_default();
            return Err(AppError::TwitchGql {
                detail: format!("{status}: {}", truncate(&text, 200)),
            });
        }

        let parsed: GqlResponse = response.json().await.map_err(|e| AppError::TwitchGql {
            detail: format!("body parse: {e}"),
        })?;

        if !parsed.errors.is_empty() {
            return Err(AppError::TwitchGql {
                detail: parsed
                    .errors
                    .into_iter()
                    .map(|e| e.message)
                    .collect::<Vec<_>>()
                    .join(" | "),
            });
        }

        let moments = parsed
            .data
            .and_then(|d| d.video)
            .and_then(|v| v.moments)
            .map(|m| m.edges)
            .unwrap_or_default();

        Ok(moments
            .into_iter()
            .filter_map(|edge| {
                let node = edge.node?;
                if node.kind.as_deref() != Some("GAME_CHANGE") {
                    return None;
                }
                let game = node.details.and_then(|d| d.game);
                Some(Chapter {
                    position_ms: node.position_milliseconds.unwrap_or(0),
                    duration_ms: node.duration_milliseconds.unwrap_or(0),
                    game_id: game.as_ref().and_then(|g| g.id.clone()),
                    game_name: game
                        .as_ref()
                        .and_then(|g| g.display_name.clone())
                        .unwrap_or_default(),
                    chapter_type: ChapterType::GameChange,
                })
            })
            .collect())
    }
}

fn build_query_body(operation_name: &str, video_id: &str) -> serde_json::Value {
    // We ship a small inline query that both known query names accept.
    // Twitch's real frontend uses persisted-query hashes, but those hashes
    // change and are themselves undocumented; the explicit query body is
    // more stable across schema migrations.
    const QUERY: &str = r#"
        query VideoMoments($videoID: ID!) {
            video(id: $videoID) {
                moments(momentRequestType: VIDEO_CHAPTER_MARKERS) {
                    edges {
                        node {
                            type
                            positionMilliseconds
                            durationMilliseconds
                            details {
                                ... on GameChangeMomentDetails {
                                    game {
                                        id
                                        displayName
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    "#;
    serde_json::json!({
        "operationName": operation_name,
        "query": QUERY,
        "variables": { "videoID": video_id },
    })
}

fn truncate(s: &str, max_chars: usize) -> String {
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

#[derive(Debug, Deserialize, Default)]
struct GqlResponse {
    #[serde(default)]
    data: Option<GqlData>,
    #[serde(default)]
    errors: Vec<GqlError>,
}

#[derive(Debug, Deserialize)]
struct GqlData {
    #[serde(default)]
    video: Option<GqlVideo>,
}

#[derive(Debug, Deserialize)]
struct GqlVideo {
    #[serde(default)]
    moments: Option<GqlMoments>,
}

#[derive(Debug, Deserialize)]
struct GqlMoments {
    #[serde(default)]
    edges: Vec<GqlEdge>,
}

#[derive(Debug, Deserialize, Default)]
struct GqlEdge {
    #[serde(default)]
    node: Option<GqlNode>,
}

#[derive(Debug, Deserialize)]
struct GqlNode {
    #[serde(rename = "type", default)]
    kind: Option<String>,
    #[serde(rename = "positionMilliseconds", default)]
    position_milliseconds: Option<i64>,
    #[serde(rename = "durationMilliseconds", default)]
    duration_milliseconds: Option<i64>,
    #[serde(default)]
    details: Option<GqlDetails>,
}

#[derive(Debug, Deserialize)]
struct GqlDetails {
    #[serde(default)]
    game: Option<GqlGame>,
}

#[derive(Debug, Deserialize)]
struct GqlGame {
    #[serde(default)]
    id: Option<String>,
    #[serde(rename = "displayName", default)]
    display_name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GqlError {
    #[serde(default)]
    message: String,
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn fixture_two_chapters() -> serde_json::Value {
        serde_json::json!({
            "data": {
                "video": {
                    "moments": {
                        "edges": [
                            {
                                "node": {
                                    "type": "GAME_CHANGE",
                                    "positionMilliseconds": 0,
                                    "durationMilliseconds": 1_800_000,
                                    "details": {
                                        "game": {
                                            "id": "32982",
                                            "displayName": "Grand Theft Auto V"
                                        }
                                    }
                                }
                            },
                            {
                                "node": {
                                    "type": "GAME_CHANGE",
                                    "positionMilliseconds": 1_800_000,
                                    "durationMilliseconds": 600_000,
                                    "details": {
                                        "game": {
                                            "id": "509658",
                                            "displayName": "Just Chatting"
                                        }
                                    }
                                }
                            }
                        ]
                    }
                }
            },
            "errors": []
        })
    }

    #[tokio::test]
    async fn parses_valid_moments() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/gql"))
            .and(header("Client-Id", PUBLIC_CLIENT_ID))
            .respond_with(ResponseTemplate::new(200).set_body_json(fixture_two_chapters()))
            .mount(&server)
            .await;

        let client =
            GqlClient::new(reqwest::Client::new()).with_endpoint(format!("{}/gql", server.uri()));
        let chapters = client.fetch_video_moments("abc123").await.unwrap();
        assert_eq!(chapters.len(), 2);
        assert_eq!(chapters[0].game_id.as_deref(), Some("32982"));
        assert_eq!(chapters[1].game_name, "Just Chatting");
        assert_eq!(chapters[0].chapter_type, ChapterType::GameChange);
    }

    #[tokio::test]
    async fn ignores_non_game_change_nodes() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/gql"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": {
                    "video": {
                        "moments": {
                            "edges": [
                                { "node": { "type": "OTHER" } },
                                { "node": { "type": "GAME_CHANGE", "positionMilliseconds": 0, "durationMilliseconds": 1, "details": { "game": { "id": "1", "displayName": "g" } } } }
                            ]
                        }
                    }
                }
            })))
            .mount(&server)
            .await;

        let client =
            GqlClient::new(reqwest::Client::new()).with_endpoint(format!("{}/gql", server.uri()));
        let out = client.fetch_video_moments("abc").await.unwrap();
        assert_eq!(out.len(), 1);
    }

    #[tokio::test]
    async fn empty_moments_returns_empty_vec() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/gql"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": { "video": { "moments": { "edges": [] } } },
                "errors": []
            })))
            .mount(&server)
            .await;
        let client =
            GqlClient::new(reqwest::Client::new()).with_endpoint(format!("{}/gql", server.uri()));
        let out = client.fetch_video_moments("abc").await.unwrap();
        assert!(out.is_empty());
    }

    #[tokio::test]
    async fn gql_errors_array_is_surfaced() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/gql"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": null,
                "errors": [{ "message": "persisted query not found" }]
            })))
            .mount(&server)
            .await;
        let client =
            GqlClient::new(reqwest::Client::new()).with_endpoint(format!("{}/gql", server.uri()));
        let err = client.fetch_video_moments("abc").await.unwrap_err();
        assert!(matches!(err, AppError::TwitchGql { .. }));
    }

    #[tokio::test]
    async fn non_2xx_is_typed_gql_error() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/gql"))
            .respond_with(ResponseTemplate::new(500).set_body_string("boom"))
            .mount(&server)
            .await;
        let client =
            GqlClient::new(reqwest::Client::new()).with_endpoint(format!("{}/gql", server.uri()));
        let err = client.fetch_video_moments("abc").await.unwrap_err();
        match err {
            AppError::TwitchGql { detail } => assert!(detail.contains("500")),
            other => panic!("expected TwitchGql, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn unknown_fields_are_ignored() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/gql"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": {
                    "video": {
                        "moments": {
                            "edges": [{ "node": { "type": "GAME_CHANGE", "positionMilliseconds": 10, "durationMilliseconds": 20, "unexpected": "ignored", "details": { "game": { "id": "a", "displayName": "A", "extra": true } } } }]
                        }
                    }
                }
            })))
            .mount(&server)
            .await;
        let client =
            GqlClient::new(reqwest::Client::new()).with_endpoint(format!("{}/gql", server.uri()));
        let out = client.fetch_video_moments("abc").await.unwrap();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].position_ms, 10);
    }

    #[test]
    fn query_body_names_the_operation() {
        let body = build_query_body("VideoPlayerStreamMetadata", "abc");
        assert_eq!(body["operationName"], "VideoPlayerStreamMetadata");
        assert_eq!(body["variables"]["videoID"], "abc");
    }
}
