//! Health service — assembles a `HealthReport` by reading schema metadata
//! from the database and combining it with process-level facts.

use std::time::{SystemTime, UNIX_EPOCH};

use crate::domain::health::HealthReport;
use crate::error::AppError;
use crate::infra::db::Db;

pub struct HealthService<'a> {
    db: &'a Db,
}

impl<'a> HealthService<'a> {
    pub fn new(db: &'a Db) -> Self {
        Self { db }
    }

    pub async fn report(&self, started_at: i64) -> Result<HealthReport, AppError> {
        let schema_version = self.db.schema_version().await?;
        Ok(HealthReport {
            app_name: "sightline".to_string(),
            app_version: env!("CARGO_PKG_VERSION").to_string(),
            schema_version,
            started_at,
            checked_at: unix_now(),
        })
    }
}

fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn report_round_trips_app_version() {
        let db = Db::open_in_memory().await.unwrap();
        db.migrate().await.unwrap();
        let svc = HealthService::new(&db);
        let report = svc.report(0).await.unwrap();
        assert_eq!(report.app_name, "sightline");
        assert_eq!(report.app_version, env!("CARGO_PKG_VERSION"));
        assert_eq!(report.schema_version, 1);
        assert!(report.checked_at >= report.started_at);
    }
}
