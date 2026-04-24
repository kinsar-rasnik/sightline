//! Polling scheduler.
//!
//! Owns a long-lived Tokio task that:
//!
//! * Finds streamers with `next_poll_at <= now` (ignoring soft-deleted
//!   rows).
//! * Dispatches them to the ingest pipeline with a semaphore-backed
//!   concurrency cap (default 4).
//! * On success, advances `next_poll_at` using the adaptive rule from
//!   `domain::poll_schedule`.
//! * Writes one `poll_log` row per attempt with counts + outcome.
//! * Forwards `IngestEvent` to the Tauri event bus so the frontend can
//!   invalidate caches.
//!
//! Graceful shutdown: the task listens for `tokio::sync::broadcast::Receiver`
//! ticks from `PollerHandle::shutdown` and drains in-flight work.

use std::sync::Arc;

use tokio::sync::{Semaphore, broadcast, mpsc};
use tokio::task::JoinHandle;
use tokio::time::{Duration, Instant};
use tracing::{debug, info, warn};

use crate::domain::poll_schedule::{PollIntervals, StreamerState, next_poll_at};
use crate::error::AppError;
use crate::infra::clock::Clock;
use crate::infra::db::Db;
use crate::services::ingest::{IngestEvent, IngestOptions, IngestReport, IngestService};
use crate::services::settings::SettingsService;
use crate::services::streamers::StreamerService;

/// Minimum sleep between scheduler wake-ups when everything is idle —
/// keeps the hot loop from spinning while still letting the
/// manual-trigger channel wake us quickly.
const IDLE_TICK: Duration = Duration::from_secs(15);

#[derive(Debug, Clone)]
pub enum PollerCommand {
    /// Trigger a poll for one streamer immediately (subject to rate
    /// limits). If `twitch_user_id` is `None`, poll every due streamer
    /// in the next tick.
    Trigger { twitch_user_id: Option<String> },
}

/// Handle for external callers. Drop-safe; calling `shutdown` more
/// than once is allowed (broadcast + ignored-err pattern).
#[derive(Debug, Clone)]
pub struct PollerHandle {
    commands: mpsc::Sender<PollerCommand>,
    shutdown: broadcast::Sender<()>,
}

impl PollerHandle {
    pub async fn trigger(&self, twitch_user_id: Option<String>) -> Result<(), AppError> {
        self.commands
            .send(PollerCommand::Trigger { twitch_user_id })
            .await
            .map_err(|e| AppError::Internal {
                detail: format!("poller channel closed: {e}"),
            })
    }

    pub fn shutdown(&self) {
        let _ = self.shutdown.send(());
    }
}

/// Tauri-agnostic callback the `commands` layer hands in so the poller
/// can emit events without depending on `tauri::AppHandle` directly —
/// makes unit tests possible without a running runtime.
pub type EventSink = Arc<dyn Fn(IngestEvent) + Send + Sync>;

pub struct PollerSpawn {
    pub handle: PollerHandle,
    pub join: JoinHandle<()>,
}

#[derive(Debug)]
pub struct PollerService {
    db: Db,
    clock: Arc<dyn Clock>,
    settings: SettingsService,
    streamers: Arc<StreamerService>,
    ingest: Arc<IngestService>,
}

impl PollerService {
    pub fn new(
        db: Db,
        clock: Arc<dyn Clock>,
        settings: SettingsService,
        streamers: Arc<StreamerService>,
        ingest: Arc<IngestService>,
    ) -> Self {
        Self {
            db,
            clock,
            settings,
            streamers,
            ingest,
        }
    }

    /// Spawn the scheduler task. The returned handle can be used to
    /// trigger on-demand polls or shut the task down.
    pub fn spawn(self: Arc<Self>, events: EventSink) -> PollerSpawn {
        let (tx_cmd, mut rx_cmd) = mpsc::channel::<PollerCommand>(32);
        let (tx_stop, mut rx_stop) = broadcast::channel::<()>(1);

        let join = tokio::spawn({
            let this = self;
            async move {
                let mut tick =
                    tokio::time::interval_at(Instant::now() + Duration::from_secs(2), IDLE_TICK);
                tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

                loop {
                    tokio::select! {
                        _ = rx_stop.recv() => {
                            info!("poller shutting down");
                            break;
                        }
                        Some(cmd) = rx_cmd.recv() => {
                            match cmd {
                                PollerCommand::Trigger { twitch_user_id } => {
                                    if let Err(e) = this.tick_with_target(&events, twitch_user_id.as_deref()).await {
                                        warn!(error = %e, "manual poll trigger failed");
                                    }
                                }
                            }
                        }
                        _ = tick.tick() => {
                            if let Err(e) = this.tick_with_target(&events, None).await {
                                warn!(error = %e, "scheduler tick failed");
                            }
                        }
                    }
                }
            }
        });

        PollerSpawn {
            handle: PollerHandle {
                commands: tx_cmd,
                shutdown: tx_stop,
            },
            join,
        }
    }

    /// Exposed for tests and manual triggers. Polls every streamer due
    /// right now, or just the one targeted, respecting the concurrency
    /// cap from settings.
    pub async fn tick_with_target(
        &self,
        events: &EventSink,
        target: Option<&str>,
    ) -> Result<(), AppError> {
        let settings = self.settings.get().await?;
        let intervals = SettingsService::intervals_from(&settings);
        let cap = settings.concurrency_cap.clamp(1, 16) as usize;

        let due = match target {
            Some(id) => vec![id.to_owned()],
            None => self.streamers.due_for_poll(cap as i64 * 4).await?,
        };
        if due.is_empty() {
            return Ok(());
        }
        debug!(count = due.len(), "poll tick");

        let sem = Arc::new(Semaphore::new(cap));
        let mut joins = Vec::with_capacity(due.len());
        for streamer_id in due {
            let permit = sem
                .clone()
                .acquire_owned()
                .await
                .map_err(|e| AppError::Internal {
                    detail: format!("semaphore closed: {e}"),
                })?;
            let this = self.clone_state();
            let events = events.clone();
            let intervals = intervals.clone();
            joins.push(tokio::spawn(async move {
                let _permit = permit;
                this.poll_one(&events, &streamer_id, &intervals).await
            }));
        }
        for j in joins {
            match j.await {
                Ok(Ok(())) => {}
                Ok(Err(e)) => warn!(error = %e, "poll_one returned error"),
                Err(e) => warn!(error = %e, "poll_one task panicked"),
            }
        }
        Ok(())
    }

    fn clone_state(&self) -> SharedPollerState {
        SharedPollerState {
            db: self.db.clone(),
            clock: self.clock.clone(),
            streamers: self.streamers.clone(),
            ingest: self.ingest.clone(),
        }
    }
}

/// Cheap-to-clone bundle of the state the poll tasks actually need.
#[derive(Clone)]
struct SharedPollerState {
    db: Db,
    clock: Arc<dyn Clock>,
    streamers: Arc<StreamerService>,
    ingest: Arc<IngestService>,
}

impl SharedPollerState {
    async fn poll_one(
        &self,
        events: &EventSink,
        streamer_id: &str,
        intervals: &PollIntervals,
    ) -> Result<(), AppError> {
        let started_at = self.clock.unix_seconds();
        let log_id = self.begin_poll_log(streamer_id, started_at).await?;

        let report_result = self.ingest.run(streamer_id, IngestOptions::default()).await;

        let finished_at = self.clock.unix_seconds();
        let (status, report_for_log) = match report_result {
            Ok((report, ingest_events)) => {
                for ev in ingest_events {
                    (events)(ev);
                }
                let status = derive_status(&report);
                self.schedule_next(streamer_id, intervals, &report).await?;
                (status, report)
            }
            Err(e) => {
                warn!(error = %e, "ingest run failed");
                let report = IngestReport {
                    errors: vec![e.to_string()],
                    ..IngestReport::default()
                };
                // Still push `next_poll_at` out so we don't hot-loop on a
                // broken run.
                self.schedule_next(streamer_id, intervals, &report).await?;
                ("error".to_owned(), report)
            }
        };

        self.finalize_poll_log(log_id, finished_at, &status, &report_for_log)
            .await?;
        self.streamers
            .mark_polled(streamer_id, finished_at + intervals.floor_seconds)
            .await?;
        Ok(())
    }

    async fn schedule_next(
        &self,
        streamer_id: &str,
        intervals: &PollIntervals,
        report: &IngestReport,
    ) -> Result<(), AppError> {
        let state = self
            .load_streamer_state(streamer_id, report.live_now)
            .await?;
        let seed = seed_from_id(streamer_id);
        let target = next_poll_at(intervals, &state, seed, PollIntervals::DEFAULT_JITTER_BPS);
        self.streamers.mark_polled(streamer_id, target).await
    }

    async fn load_streamer_state(
        &self,
        streamer_id: &str,
        live_now: bool,
    ) -> Result<StreamerState, AppError> {
        let row = sqlx::query_as::<_, (Option<i64>, Option<i64>)>(
            "SELECT last_live_at, last_polled_at FROM streamers WHERE twitch_user_id = ?",
        )
        .bind(streamer_id)
        .fetch_one(self.db.pool())
        .await?;
        Ok(StreamerState {
            now_unix: self.clock.unix_seconds(),
            last_live_at: row.0,
            last_polled_at: row.1,
            live_now,
        })
    }

    async fn begin_poll_log(&self, streamer_id: &str, started_at: i64) -> Result<i64, AppError> {
        let id: i64 = sqlx::query_scalar(
            "INSERT INTO poll_log (twitch_user_id, started_at, status) VALUES (?, ?, 'running')
             RETURNING id",
        )
        .bind(streamer_id)
        .bind(started_at)
        .fetch_one(self.db.pool())
        .await?;
        Ok(id)
    }

    async fn finalize_poll_log(
        &self,
        log_id: i64,
        finished_at: i64,
        status: &str,
        report: &IngestReport,
    ) -> Result<(), AppError> {
        let errors_json = serde_json::to_string(&report.errors).map_err(AppError::from)?;
        sqlx::query(
            "UPDATE poll_log
             SET finished_at = ?, vods_seen = ?, vods_new = ?, vods_updated = ?,
                 chapters_fetched = ?, errors_json = ?, status = ?
             WHERE id = ?",
        )
        .bind(finished_at)
        .bind(report.vods_seen)
        .bind(report.vods_new)
        .bind(report.vods_updated)
        .bind(report.chapters_fetched)
        .bind(&errors_json)
        .bind(status)
        .bind(log_id)
        .execute(self.db.pool())
        .await?;
        Ok(())
    }
}

fn derive_status(report: &IngestReport) -> String {
    if report.rate_limited {
        return "rate_limited".to_owned();
    }
    if !report.errors.is_empty() && report.vods_new == 0 && report.vods_updated == 0 {
        return "error".to_owned();
    }
    if !report.errors.is_empty() {
        return "partial".to_owned();
    }
    "ok".to_owned()
}

fn seed_from_id(streamer_id: &str) -> u64 {
    let mut seed: u64 = 0xdeadbeef_u64;
    for b in streamer_id.bytes() {
        seed = seed.wrapping_mul(131).wrapping_add(u64::from(b));
    }
    seed | 1
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn seed_is_stable_for_same_id() {
        assert_eq!(seed_from_id("abc"), seed_from_id("abc"));
        assert_ne!(seed_from_id("abc"), seed_from_id("abd"));
    }

    #[test]
    fn derive_status_labels_paths() {
        assert_eq!(derive_status(&IngestReport::default()), "ok");
        assert_eq!(
            derive_status(&IngestReport {
                rate_limited: true,
                ..Default::default()
            }),
            "rate_limited"
        );
        assert_eq!(
            derive_status(&IngestReport {
                errors: vec!["x".into()],
                vods_new: 1,
                ..Default::default()
            }),
            "partial"
        );
        assert_eq!(
            derive_status(&IngestReport {
                errors: vec!["x".into()],
                ..Default::default()
            }),
            "error"
        );
    }
}
