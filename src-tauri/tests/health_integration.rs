//! Integration test: exercise the health service end-to-end against a
//! temp-file SQLite database. This covers what a unit test cannot: the
//! migration runner + the service composition.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use sightline_lib::Db;
use sightline_lib::services::health::HealthService;
use tempfile::tempdir;

#[tokio::test]
async fn health_report_against_tempfile_db() {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("sightline.sqlite");

    let db = Db::open(&path).await.expect("open db");
    db.migrate().await.expect("migrate");

    let svc = HealthService::new(&db);
    let started = 1_700_000_000;
    let report = svc.report(started).await.expect("report");

    assert_eq!(report.app_name, "sightline");
    assert!(
        report.schema_version >= 1,
        "schema version should be monotonic"
    );
    assert_eq!(report.started_at, started);
    assert!(report.checked_at >= started);
}
