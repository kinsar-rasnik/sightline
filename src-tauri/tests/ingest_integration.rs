//! End-to-end ingest integration test.
//!
//! Stands up:
//!   * An in-memory SQLite pool with the full migration set.
//!   * A wiremock for Helix (`/users`, `/videos`, `/streams`).
//!   * A wiremock for the GraphQL endpoint.
//!   * A fixed clock and an in-memory credential store.
//!
//! Then drives the full flow: add a streamer, run an ingest, classify
//! VODs, persist chapters, and assert the database state matches the
//! Phase 2 state-machine invariants.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::{Arc, Mutex};

use sightline_lib::infra::clock::{Clock, FixedClock};
use sightline_lib::infra::db::Db;
use sightline_lib::infra::keychain::{Credentials, InMemoryCredentials, TwitchCredentials};
use sightline_lib::infra::twitch::auth::{TwitchAuthenticator, prime_token_for_tests};
use sightline_lib::infra::twitch::gql::GqlClient;
use sightline_lib::infra::twitch::helix::HelixClient;
use sightline_lib::services::ingest::{IngestOptions, IngestService};
use sightline_lib::services::poller::{EventSink, PollerEvent, PollerService};
use sightline_lib::services::settings::SettingsService;
use sightline_lib::services::streamers::StreamerService;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

struct Harness {
    db: Db,
    ingest: IngestService,
    streamers: Arc<StreamerService>,
    helix: Arc<HelixClient>,
    gql: Arc<GqlClient>,
    clock: Arc<dyn Clock>,
}

async fn harness(helix_server: &MockServer, gql_server: &MockServer) -> Harness {
    let db = Db::open_in_memory().await.unwrap();
    db.migrate().await.unwrap();

    let clock = Arc::new(FixedClock::at(1_000_000));
    let credentials: Arc<dyn Credentials> = {
        let store = Arc::new(InMemoryCredentials::default());
        store
            .write(&TwitchCredentials {
                client_id: "clientid000000000000000000000".into(),
                client_secret: "secret".into(),
            })
            .await
            .unwrap();
        store
    };

    let auth = Arc::new(
        TwitchAuthenticator::new(reqwest::Client::new(), clock.clone(), credentials.clone())
            .with_endpoint("http://127.0.0.1:1/unused".to_owned()),
    );
    prime_token_for_tests(&auth, "test-token".to_owned(), clock.unix_seconds() + 3600).await;

    let helix = Arc::new(
        HelixClient::new(reqwest::Client::new(), auth.clone(), clock.clone())
            .with_base(helix_server.uri()),
    );
    let gql = Arc::new(GqlClient::new(reqwest::Client::new()).with_endpoint(gql_server.uri()));
    let settings = SettingsService::new(db.clone(), clock.clone());
    let streamers = Arc::new(StreamerService::new(
        db.clone(),
        helix.clone(),
        clock.clone(),
    ));
    let ingest = IngestService::new(
        db.clone(),
        helix.clone(),
        gql.clone(),
        clock.clone(),
        settings,
        streamers.clone(),
    );

    Harness {
        db,
        ingest,
        streamers,
        helix,
        gql,
        clock,
    }
}

fn video_json(id: &str, started: &str, viewable: &str, kind: &str) -> serde_json::Value {
    serde_json::json!({
        "id": id,
        "user_id": "100",
        "stream_id": null,
        "title": format!("sample {id}"),
        "description": "",
        "url": format!("https://twitch.tv/videos/{id}"),
        "thumbnail_url": "",
        "created_at": started,
        "published_at": started,
        "duration": "1h30m0s",
        "view_count": 10,
        "language": "en",
        "type": kind,
        "viewable": viewable
    })
}

fn gql_two_chapters(game: &str) -> serde_json::Value {
    serde_json::json!({
        "data": {
            "video": {
                "moments": {
                    "edges": [
                        {
                            "node": {
                                "type": "GAME_CHANGE",
                                "positionMilliseconds": 0,
                                "durationMilliseconds": 1800000,
                                "details": { "game": { "id": game, "displayName": "Game" } }
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
async fn happy_path_ingests_and_classifies_eligible() {
    let helix = MockServer::start().await;
    let gql = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/users"))
        .and(query_param("login", "sampler"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [{
                "id": "100",
                "login": "sampler",
                "display_name": "Sampler",
                "profile_image_url": "",
                "broadcaster_type": "",
                "created_at": "2020-01-01T00:00:00Z"
            }],
            "pagination": {}
        })))
        .mount(&helix)
        .await;
    Mock::given(method("GET"))
        .and(path("/streams"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [], "pagination": {}
        })))
        .mount(&helix)
        .await;
    Mock::given(method("GET"))
        .and(path("/videos"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [ video_json("v1", "2026-04-01T00:00:00Z", "public", "archive") ],
            "pagination": {}
        })))
        .mount(&helix)
        .await;
    Mock::given(method("POST"))
        .and(path(""))
        .respond_with(ResponseTemplate::new(200).set_body_json(gql_two_chapters("32982")))
        .mount(&gql)
        .await;

    let h = harness(&helix, &gql).await;
    h.streamers.add("sampler").await.unwrap();
    let (report, events) = h.ingest.run("100", IngestOptions::default()).await.unwrap();
    assert_eq!(report.vods_new, 1);
    assert_eq!(events.len(), 1);

    let status: String =
        sqlx::query_scalar("SELECT ingest_status FROM vods WHERE twitch_video_id = 'v1'")
            .fetch_one(h.db.pool())
            .await
            .unwrap();
    assert_eq!(status, "eligible");
}

#[tokio::test]
async fn live_streamer_vods_are_deferred() {
    let helix = MockServer::start().await;
    let gql = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/users"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [{
                "id": "100", "login": "live", "display_name": "Live",
                "profile_image_url": "", "broadcaster_type": "",
                "created_at": "2020-01-01T00:00:00Z"
            }],
            "pagination": {}
        })))
        .mount(&helix)
        .await;
    Mock::given(method("GET"))
        .and(path("/streams"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [{ "id": "s1" }], "pagination": {}
        })))
        .mount(&helix)
        .await;
    Mock::given(method("GET"))
        .and(path("/videos"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [ video_json("vL", "2026-04-01T00:00:00Z", "public", "archive") ],
            "pagination": {}
        })))
        .mount(&helix)
        .await;
    Mock::given(method("POST"))
        .and(path(""))
        .respond_with(ResponseTemplate::new(200).set_body_json(gql_two_chapters("32982")))
        .mount(&gql)
        .await;

    let h = harness(&helix, &gql).await;
    h.streamers.add("live").await.unwrap();
    let (report, _) = h.ingest.run("100", IngestOptions::default()).await.unwrap();
    assert!(report.live_now);
    let status: String =
        sqlx::query_scalar("SELECT ingest_status FROM vods WHERE twitch_video_id = 'vL'")
            .fetch_one(h.db.pool())
            .await
            .unwrap();
    assert_eq!(status, "skipped_live");
}

#[tokio::test]
async fn non_gta_vod_is_skipped_game() {
    let helix = MockServer::start().await;
    let gql = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/users"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [{
                "id": "100", "login": "variety", "display_name": "Variety",
                "profile_image_url": "", "broadcaster_type": "",
                "created_at": "2020-01-01T00:00:00Z"
            }], "pagination": {}
        })))
        .mount(&helix)
        .await;
    Mock::given(method("GET"))
        .and(path("/streams"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [], "pagination": {}
        })))
        .mount(&helix)
        .await;
    Mock::given(method("GET"))
        .and(path("/videos"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [ video_json("vS", "2026-04-01T00:00:00Z", "public", "archive") ],
            "pagination": {}
        })))
        .mount(&helix)
        .await;
    Mock::given(method("POST"))
        .and(path(""))
        .respond_with(ResponseTemplate::new(200).set_body_json(gql_two_chapters("509658"))) // Just Chatting
        .mount(&gql)
        .await;

    let h = harness(&helix, &gql).await;
    h.streamers.add("variety").await.unwrap();
    h.ingest.run("100", IngestOptions::default()).await.unwrap();
    let status: String =
        sqlx::query_scalar("SELECT ingest_status FROM vods WHERE twitch_video_id = 'vS'")
            .fetch_one(h.db.pool())
            .await
            .unwrap();
    assert_eq!(status, "skipped_game");
}

#[tokio::test]
async fn sub_only_vod_is_flagged() {
    let helix = MockServer::start().await;
    let gql = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/users"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [{
                "id": "100", "login": "subonly", "display_name": "SubOnly",
                "profile_image_url": "", "broadcaster_type": "",
                "created_at": "2020-01-01T00:00:00Z"
            }], "pagination": {}
        })))
        .mount(&helix)
        .await;
    Mock::given(method("GET"))
        .and(path("/streams"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [], "pagination": {}
        })))
        .mount(&helix)
        .await;
    Mock::given(method("GET"))
        .and(path("/videos"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [ video_json("vP", "2026-04-01T00:00:00Z", "private", "archive") ],
            "pagination": {}
        })))
        .mount(&helix)
        .await;
    Mock::given(method("POST"))
        .and(path(""))
        .respond_with(ResponseTemplate::new(200).set_body_json(gql_two_chapters("32982")))
        .mount(&gql)
        .await;

    let h = harness(&helix, &gql).await;
    h.streamers.add("subonly").await.unwrap();
    h.ingest.run("100", IngestOptions::default()).await.unwrap();
    let (status, is_sub_only): (String, i64) =
        sqlx::query_as("SELECT ingest_status, is_sub_only FROM vods WHERE twitch_video_id = 'vP'")
            .fetch_one(h.db.pool())
            .await
            .unwrap();
    assert_eq!(status, "skipped_sub_only");
    assert_eq!(is_sub_only, 1);
}

#[tokio::test]
async fn empty_videos_list_is_fine() {
    let helix = MockServer::start().await;
    let gql = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/users"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [{ "id": "100", "login": "fresh", "display_name": "Fresh",
                       "profile_image_url": "", "broadcaster_type": "", "created_at": "2020-01-01T00:00:00Z" }],
            "pagination": {}
        })))
        .mount(&helix)
        .await;
    Mock::given(method("GET"))
        .and(path("/streams"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [], "pagination": {}
        })))
        .mount(&helix)
        .await;
    Mock::given(method("GET"))
        .and(path("/videos"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [], "pagination": {}
        })))
        .mount(&helix)
        .await;

    let h = harness(&helix, &gql).await;
    h.streamers.add("fresh").await.unwrap();
    let (report, _) = h.ingest.run("100", IngestOptions::default()).await.unwrap();
    assert_eq!(report.vods_seen, 0);
    assert_eq!(report.vods_new, 0);
}

// ---------------------------------------------------------------------
// Poller emit path — confirms `poll:started` / `poll:finished` and the
// wrapped ingest events all flow through the same `EventSink` for a
// single streamer cycle.
// ---------------------------------------------------------------------
#[tokio::test]
async fn poller_emits_start_ingest_finish_events_in_order() {
    let helix_server = MockServer::start().await;
    let gql_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/users"))
        .and(query_param("login", "emitter"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [{
                "id": "100", "login": "emitter", "display_name": "Emitter",
                "profile_image_url": "", "broadcaster_type": "",
                "created_at": "2020-01-01T00:00:00Z"
            }], "pagination": {}
        })))
        .mount(&helix_server)
        .await;
    Mock::given(method("GET"))
        .and(path("/streams"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [], "pagination": {}
        })))
        .mount(&helix_server)
        .await;
    Mock::given(method("GET"))
        .and(path("/videos"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [ video_json("vE", "2026-04-01T00:00:00Z", "public", "archive") ],
            "pagination": {}
        })))
        .mount(&helix_server)
        .await;
    Mock::given(method("POST"))
        .and(path(""))
        .respond_with(ResponseTemplate::new(200).set_body_json(gql_two_chapters("32982")))
        .mount(&gql_server)
        .await;

    let h = harness(&helix_server, &gql_server).await;
    h.streamers.add("emitter").await.unwrap();

    let poller = Arc::new(PollerService::new(
        h.db.clone(),
        h.clock.clone(),
        SettingsService::new(h.db.clone(), h.clock.clone()),
        h.streamers.clone(),
        Arc::new(IngestService::new(
            h.db.clone(),
            h.helix.clone(),
            h.gql.clone(),
            h.clock.clone(),
            SettingsService::new(h.db.clone(), h.clock.clone()),
            h.streamers.clone(),
        )),
    ));

    let captured: Arc<Mutex<Vec<PollerEvent>>> = Arc::new(Mutex::new(Vec::new()));
    let captured_for_sink = captured.clone();
    let sink: EventSink = Arc::new(move |ev| {
        captured_for_sink.lock().unwrap().push(ev);
    });

    poller.tick_with_target(&sink, Some("100")).await.unwrap();

    let events = captured.lock().unwrap();
    assert!(
        events.len() >= 3,
        "expected start + ingest + finish, got {:?}",
        events.len()
    );
    assert!(matches!(
        events.first(),
        Some(PollerEvent::PollStarted { .. })
    ));
    assert!(
        matches!(events.last(), Some(PollerEvent::PollFinished { status, .. }) if status == "ok")
    );
    assert!(
        events.iter().any(|e| matches!(e, PollerEvent::Ingest(_))),
        "expected at least one ingest event in the middle: {events:?}"
    );

    // The finished event should carry the report counts the UI uses.
    match events.last().expect("poll finished event") {
        PollerEvent::PollFinished {
            twitch_user_id,
            vods_new,
            ..
        } => {
            assert_eq!(twitch_user_id, "100");
            assert_eq!(*vods_new, 1);
        }
        other => panic!("unexpected last event: {other:?}"),
    }
}
