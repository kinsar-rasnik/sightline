//! Persistent download queue.
//!
//! Responsibilities:
//!
//! * CRUD over the `downloads` table, including the state machine
//!   transitions defined in [`crate::domain::download_state`].
//! * A worker pool (sized by [`AppSettings::max_concurrent_downloads`])
//!   that picks up `queued` rows in priority + FIFO order, invokes
//!   [`YtDlp::download`] against the staging path, and runs the
//!   post-processing pipeline (ffmpeg remux → thumbnail → atomic move
//!   into the library → NFO sidecar when the layout calls for it).
//! * Event fan-out: [`DownloadEvent::StateChanged`] /
//!   [`DownloadEvent::Progress`] (throttled to ≤ 2 Hz per download) /
//!   [`DownloadEvent::Completed`] / [`DownloadEvent::Failed`].
//! * Crash recovery at startup: any row stuck in `downloading` is
//!   reset to `queued`. The (possibly-partial) staging file is
//!   abandoned — yt-dlp's resume flag is not reliable across the
//!   filesystems we target (Proton Drive, SMB shares).
//!
//! See [`crate::domain::download_state`] for the state-machine and
//! [`ADR-0010`] / [`ADR-0012`] for the throttle and atomic-move
//! rationales.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use specta::Type;
use sqlx::Row;
use tokio::sync::{Semaphore, broadcast, mpsc};
use tokio::task::JoinHandle;
use tracing::{debug, info, instrument, warn};

use crate::domain::download_state::{
    DownloadState, MAX_ATTEMPTS, Transition, apply as apply_transition, reason,
};
use crate::domain::library_layout::{LibraryLayoutKind, VodWithStreamer, layout as build_layout};
use crate::domain::nfo::{NfoInput, generate as generate_nfo};
use crate::domain::quality_preset::{QualityPreset, resolve as resolve_preset};
use crate::domain::sanitize::sanitize_component;
use crate::error::AppError;
use crate::infra::clock::Clock;
use crate::infra::db::Db;
use crate::infra::ffmpeg::{
    PREVIEW_FRAME_PERCENTS, PreviewFramesSpec, RemuxSpec, SharedFfmpeg, ThumbnailSpec, already_mp4,
};
use crate::infra::fs::move_::atomic_move;
use crate::infra::fs::space::{FreeSpaceProbe, check_preflight};
use crate::infra::throttle::GlobalRate;
use crate::infra::ytdlp::{
    DownloadProgress, DownloadSpec, SharedYtDlp, VodInfoRequest, size_estimate,
};
use crate::services::settings::SettingsService;
use crate::services::vods::VodReadService;

// -------------------------------------------------------------------
//  Public DTOs + events
// -------------------------------------------------------------------

/// A single row from the `downloads` table, joined with the owning
/// VOD + streamer so the frontend doesn't have to round-trip.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DownloadRow {
    pub vod_id: String,
    pub state: DownloadState,
    pub priority: i64,
    pub quality_preset: QualityPreset,
    pub quality_resolved: Option<String>,
    pub staging_path: Option<String>,
    pub final_path: Option<String>,
    pub bytes_total: Option<i64>,
    pub bytes_done: i64,
    pub speed_bps: Option<i64>,
    pub eta_seconds: Option<i64>,
    pub attempts: i64,
    pub last_error: Option<String>,
    pub last_error_at: Option<i64>,
    pub queued_at: i64,
    pub started_at: Option<i64>,
    pub finished_at: Option<i64>,
    pub pause_requested: bool,
    // Joined from `vods` + `streamers`:
    pub title: String,
    pub streamer_display_name: String,
    pub streamer_login: String,
    pub thumbnail_url: Option<String>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", default)]
pub struct DownloadFilters {
    #[specta(optional)]
    pub state: Option<DownloadState>,
    #[specta(optional)]
    pub streamer_id: Option<String>,
}

/// Aggregate counts used by the tray summary tooltip.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DownloadsSummary {
    pub active_count: i64,
    pub queued_count: i64,
    pub bandwidth_bps: i64,
}

/// Events the queue emits. The commands layer translates these into
/// Tauri event topics.
#[derive(Debug, Clone)]
pub enum DownloadEvent {
    StateChanged {
        vod_id: String,
        state: DownloadState,
    },
    Progress {
        vod_id: String,
        progress: DownloadProgress,
    },
    Completed {
        vod_id: String,
        final_path: PathBuf,
    },
    Failed {
        vod_id: String,
        reason: String,
    },
}

/// Callback the commands layer supplies — same pattern as the
/// existing [`crate::services::poller::EventSink`].
pub type DownloadEventSink = Arc<dyn Fn(DownloadEvent) + Send + Sync>;

/// External control plane commands. The queue owns an mpsc::Receiver
/// and processes them on its event loop. Today there is only one
/// variant — `WakeUp` — but we keep the enum so adding e.g. a
/// `RetryAllFailed` doesn't require a new channel.
#[derive(Debug, Clone)]
enum QueueCommand {
    WakeUp,
}

// -------------------------------------------------------------------
//  Service
// -------------------------------------------------------------------

/// Handle for external callers — send commands, stop the worker loop.
#[derive(Debug, Clone)]
pub struct DownloadQueueHandle {
    commands: mpsc::Sender<QueueCommand>,
    shutdown: broadcast::Sender<()>,
}

impl DownloadQueueHandle {
    pub async fn wake_up(&self) {
        let _ = self.commands.send(QueueCommand::WakeUp).await;
    }

    pub fn shutdown(&self) {
        let _ = self.shutdown.send(());
    }
}

pub struct DownloadQueueSpawn {
    pub handle: DownloadQueueHandle,
    pub join: JoinHandle<()>,
}

#[derive(Debug)]
pub struct DownloadQueueService {
    db: Db,
    clock: Arc<dyn Clock>,
    ytdlp: SharedYtDlp,
    ffmpeg: SharedFfmpeg,
    space_probe: Arc<dyn FreeSpaceProbe>,
    rate: Arc<GlobalRate>,
    settings: SettingsService,
    vods: Arc<VodReadService>,
    /// Default staging dir — used when `app_settings.staging_path`
    /// is NULL.
    default_staging: PathBuf,
}

impl DownloadQueueService {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        db: Db,
        clock: Arc<dyn Clock>,
        ytdlp: SharedYtDlp,
        ffmpeg: SharedFfmpeg,
        space_probe: Arc<dyn FreeSpaceProbe>,
        rate: Arc<GlobalRate>,
        settings: SettingsService,
        vods: Arc<VodReadService>,
        default_staging: PathBuf,
    ) -> Self {
        Self {
            db,
            clock,
            ytdlp,
            ffmpeg,
            space_probe,
            rate,
            settings,
            vods,
            default_staging,
        }
    }

    // --------- CRUD / public API the commands layer calls ---------

    /// Insert a new row in `queued` state. Idempotent on the PK:
    /// a subsequent enqueue of the same VOD returns the existing row
    /// (so the UI's "Download" button is safe to double-click).
    pub async fn enqueue(
        &self,
        vod_id: &str,
        priority: Option<i64>,
    ) -> Result<DownloadRow, AppError> {
        let settings = self.settings.get().await?;
        let now = self.clock.unix_seconds();
        let priority = priority.unwrap_or(100);
        sqlx::query(
            "INSERT INTO downloads (vod_id, state, priority, quality_preset, queued_at)
             VALUES (?, 'queued', ?, ?, ?)
             ON CONFLICT(vod_id) DO NOTHING",
        )
        .bind(vod_id)
        .bind(priority)
        .bind(settings.quality_preset.as_db_str())
        .bind(now)
        .execute(self.db.pool())
        .await?;

        // S5 convergence: keep vods.status aligned with the
        // downloads queue from both entrypoints.  Pull-mode
        // (distribution.pick_vod) has already flipped vods.status
        // to 'queued'; auto-mode (legacy poller-driven enqueue)
        // hasn't.  This UPDATE handles both: idempotent on already-
        // queued rows, advances available/deleted rows, never
        // blows over ready/archived/downloading.
        sync_vod_status(
            self.db.pool(),
            vod_id,
            "queued",
            &["available", "deleted", "queued"],
        )
        .await?;

        self.get(vod_id).await?.ok_or(AppError::NotFound)
    }

    pub async fn get(&self, vod_id: &str) -> Result<Option<DownloadRow>, AppError> {
        let row = sqlx::query(DOWNLOAD_SELECT_SQL)
            .bind(vod_id)
            .fetch_optional(self.db.pool())
            .await?;
        row.as_ref().map(row_to_download).transpose()
    }

    pub async fn list(&self, filters: &DownloadFilters) -> Result<Vec<DownloadRow>, AppError> {
        let mut clauses: Vec<String> = vec![String::from("1 = 1")];
        if filters.state.is_some() {
            clauses.push(String::from("d.state = ?"));
        }
        if filters.streamer_id.is_some() {
            clauses.push(String::from(
                "d.vod_id IN (SELECT twitch_video_id FROM vods WHERE twitch_user_id = ?)",
            ));
        }
        let sql = format!(
            "{base} WHERE {clauses}
             ORDER BY d.priority DESC, d.queued_at ASC",
            base = DOWNLOAD_LIST_BASE,
            clauses = clauses.join(" AND ")
        );
        let mut q = sqlx::query(&sql);
        if let Some(state) = filters.state {
            q = q.bind(state.as_db_str());
        }
        if let Some(ref streamer_id) = filters.streamer_id {
            q = q.bind(streamer_id);
        }
        let rows = q.fetch_all(self.db.pool()).await?;
        rows.iter().map(row_to_download).collect()
    }

    pub async fn pause(&self, vod_id: &str) -> Result<DownloadRow, AppError> {
        self.transition(vod_id, Transition::Pause, |_| None).await
    }

    pub async fn resume(&self, vod_id: &str) -> Result<DownloadRow, AppError> {
        // Resume flips the state to `downloading` but the worker
        // won't pick it up again that way — easier: transition to
        // `queued` so the scheduler picks it up naturally.
        let now = self.clock.unix_seconds();
        sqlx::query(
            "UPDATE downloads
             SET state = 'queued', pause_requested = 0, started_at = NULL,
                 speed_bps = NULL, eta_seconds = NULL, last_error = NULL,
                 last_error_at = NULL, queued_at = ?
             WHERE vod_id = ? AND state = 'paused'",
        )
        .bind(now)
        .bind(vod_id)
        .execute(self.db.pool())
        .await?;
        self.get(vod_id).await?.ok_or(AppError::NotFound)
    }

    pub async fn cancel(&self, vod_id: &str) -> Result<DownloadRow, AppError> {
        self.transition(vod_id, Transition::Cancel, |_| {
            Some(reason::USER_CANCELLED.into())
        })
        .await
    }

    pub async fn retry(&self, vod_id: &str) -> Result<DownloadRow, AppError> {
        let now = self.clock.unix_seconds();
        sqlx::query(
            "UPDATE downloads
             SET state = 'queued', attempts = 0, pause_requested = 0,
                 started_at = NULL, finished_at = NULL, bytes_done = 0,
                 speed_bps = NULL, eta_seconds = NULL, last_error = NULL,
                 last_error_at = NULL, queued_at = ?
             WHERE vod_id = ? AND state IN ('failed_retryable', 'failed_permanent')",
        )
        .bind(now)
        .bind(vod_id)
        .execute(self.db.pool())
        .await?;
        self.get(vod_id).await?.ok_or(AppError::NotFound)
    }

    pub async fn reprioritize(&self, vod_id: &str, priority: i64) -> Result<DownloadRow, AppError> {
        sqlx::query("UPDATE downloads SET priority = ? WHERE vod_id = ?")
            .bind(priority)
            .bind(vod_id)
            .execute(self.db.pool())
            .await?;
        self.get(vod_id).await?.ok_or(AppError::NotFound)
    }

    /// Pause every row in `downloading` or `queued`. Used by the tray
    /// "Pause all" action.
    pub async fn pause_all(&self) -> Result<i64, AppError> {
        let now = self.clock.unix_seconds();
        let affected = sqlx::query(
            "UPDATE downloads
             SET state = 'paused',
                 pause_requested = 1,
                 last_error_at = ?
             WHERE state IN ('queued', 'downloading')",
        )
        .bind(now)
        .execute(self.db.pool())
        .await?;
        Ok(affected.rows_affected() as i64)
    }

    /// Resume every paused row. Counterpart of `pause_all`.
    pub async fn resume_all(&self) -> Result<i64, AppError> {
        let now = self.clock.unix_seconds();
        let affected = sqlx::query(
            "UPDATE downloads
             SET state = 'queued', pause_requested = 0, started_at = NULL,
                 speed_bps = NULL, eta_seconds = NULL, last_error = NULL,
                 last_error_at = NULL, queued_at = ?
             WHERE state = 'paused'",
        )
        .bind(now)
        .execute(self.db.pool())
        .await?;
        Ok(affected.rows_affected() as i64)
    }

    /// Tiny summary for the tray tooltip: counts + aggregate speed.
    pub async fn summary(&self) -> Result<DownloadsSummary, AppError> {
        let row = sqlx::query(
            "SELECT
               SUM(CASE WHEN state = 'downloading' THEN 1 ELSE 0 END) AS active_count,
               SUM(CASE WHEN state = 'queued' THEN 1 ELSE 0 END)       AS queued_count,
               COALESCE(SUM(CASE WHEN state = 'downloading' THEN speed_bps ELSE 0 END), 0) AS bandwidth_bps
             FROM downloads",
        )
        .fetch_one(self.db.pool())
        .await?;
        let active: Option<i64> = row.try_get("active_count").ok();
        let queued: Option<i64> = row.try_get("queued_count").ok();
        let bandwidth: Option<i64> = row.try_get("bandwidth_bps").ok();
        Ok(DownloadsSummary {
            active_count: active.unwrap_or(0),
            queued_count: queued.unwrap_or(0),
            bandwidth_bps: bandwidth.unwrap_or(0),
        })
    }

    /// State-machine-checked transition. The `new_error` closure
    /// receives the NEW state and returns an optional reason string
    /// to record in `last_error`.
    async fn transition(
        &self,
        vod_id: &str,
        t: Transition,
        new_error: impl FnOnce(DownloadState) -> Option<String>,
    ) -> Result<DownloadRow, AppError> {
        let current_state = self.state_of(vod_id).await?;
        let next_state = apply_transition(current_state, t).map_err(|e| AppError::Download {
            detail: e.to_string(),
        })?;
        let now = self.clock.unix_seconds();
        let err = new_error(next_state);
        sqlx::query(
            "UPDATE downloads
             SET state = ?,
                 pause_requested = CASE WHEN ? = 'paused' THEN 1 ELSE 0 END,
                 last_error = COALESCE(?, last_error),
                 last_error_at = CASE WHEN ? IS NOT NULL THEN ? ELSE last_error_at END,
                 finished_at = CASE WHEN ? IN ('completed', 'failed_permanent') THEN ?
                                    ELSE finished_at END
             WHERE vod_id = ?",
        )
        .bind(next_state.as_db_str())
        .bind(next_state.as_db_str())
        .bind(&err)
        .bind(&err)
        .bind(now)
        .bind(next_state.as_db_str())
        .bind(now)
        .bind(vod_id)
        .execute(self.db.pool())
        .await?;
        self.get(vod_id).await?.ok_or(AppError::NotFound)
    }

    async fn state_of(&self, vod_id: &str) -> Result<DownloadState, AppError> {
        let state_str: Option<String> =
            sqlx::query_scalar("SELECT state FROM downloads WHERE vod_id = ?")
                .bind(vod_id)
                .fetch_optional(self.db.pool())
                .await?;
        let s = state_str.ok_or(AppError::NotFound)?;
        DownloadState::from_db_str(&s).ok_or_else(|| AppError::Download {
            detail: format!("unknown state {s} in DB for {vod_id}"),
        })
    }

    /// Reset any `downloading` rows to `queued` — run once at startup.
    pub async fn crash_recover(&self) -> Result<u64, AppError> {
        let now = self.clock.unix_seconds();
        let result = sqlx::query(
            "UPDATE downloads
             SET state = 'queued', started_at = NULL,
                 bytes_done = 0, speed_bps = NULL, eta_seconds = NULL,
                 queued_at = ?
             WHERE state = 'downloading'",
        )
        .bind(now)
        .execute(self.db.pool())
        .await?;
        Ok(result.rows_affected())
    }

    // ---------------- Worker pool ----------------

    /// Spawn the orchestration loop. A single `manager` task scans
    /// `queued` rows at WAKEUP / interval, and hands each to a worker
    /// future gated by a [`Semaphore`] sized from
    /// `max_concurrent_downloads`.
    pub fn spawn(self: Arc<Self>, events: DownloadEventSink) -> DownloadQueueSpawn {
        let (tx_cmd, mut rx_cmd) = mpsc::channel::<QueueCommand>(32);
        let (tx_stop, mut rx_stop) = broadcast::channel::<()>(1);

        let join = tokio::spawn({
            let this = self;
            async move {
                let _ = this.crash_recover().await;
                let semaphore = Arc::new(Semaphore::new(current_concurrency(&this).await as usize));
                let tick_interval = Duration::from_secs(5);

                loop {
                    tokio::select! {
                        _ = rx_stop.recv() => {
                            info!("download queue shutting down");
                            break;
                        }
                        Some(_cmd) = rx_cmd.recv() => {
                            let _ = this.drain_once(&events, &semaphore).await;
                        }
                        _ = tokio::time::sleep(tick_interval) => {
                            let _ = this.drain_once(&events, &semaphore).await;
                        }
                    }
                }
            }
        });

        DownloadQueueSpawn {
            handle: DownloadQueueHandle {
                commands: tx_cmd,
                shutdown: tx_stop,
            },
            join,
        }
    }

    /// Drain every `queued` row that will fit under the semaphore right
    /// now; each download runs in its own spawned task so the manager
    /// loop stays responsive.
    async fn drain_once(
        self: &Arc<Self>,
        events: &DownloadEventSink,
        semaphore: &Arc<Semaphore>,
    ) -> Result<(), AppError> {
        loop {
            let permit = match semaphore.clone().try_acquire_owned() {
                Ok(p) => p,
                Err(_) => return Ok(()),
            };
            let row = self.pick_next_queued().await?;
            let Some(row) = row else {
                return Ok(());
            };
            self.rate
                .set_active_workers(semaphore.available_permits() + 1);
            let this = self.clone();
            let events = events.clone();
            tokio::spawn(async move {
                let _permit = permit;
                let vod_id = row.vod_id.clone();
                if let Err(e) = this.process_one(&events, &vod_id).await {
                    warn!(vod_id = %vod_id, error = %e, "download pipeline failed");
                }
            });
        }
    }

    async fn pick_next_queued(&self) -> Result<Option<DownloadRow>, AppError> {
        let row = sqlx::query(&format!(
            "{base} WHERE d.state = 'queued' ORDER BY d.priority DESC, d.queued_at ASC LIMIT 1",
            base = DOWNLOAD_LIST_BASE
        ))
        .fetch_optional(self.db.pool())
        .await?;
        row.as_ref().map(row_to_download).transpose()
    }

    #[instrument(skip(self, events), fields(vod_id = %vod_id))]
    async fn process_one(
        self: &Arc<Self>,
        events: &DownloadEventSink,
        vod_id: &str,
    ) -> Result<(), AppError> {
        // Transition to `downloading`.
        self.set_state(vod_id, DownloadState::Downloading, None)
            .await?;
        (events)(DownloadEvent::StateChanged {
            vod_id: vod_id.to_owned(),
            state: DownloadState::Downloading,
        });
        let started_at = self.clock.unix_seconds();
        sqlx::query("UPDATE downloads SET started_at = ? WHERE vod_id = ?")
            .bind(started_at)
            .bind(vod_id)
            .execute(self.db.pool())
            .await?;

        // Run the pipeline. `pipeline_inner` returns the success /
        // classification result; we map it onto state + events.
        let outcome = self.pipeline_inner(events, vod_id).await;
        match outcome {
            Ok(PipelineOk {
                final_path,
                quality_resolved,
            }) => {
                self.mark_completed(vod_id, &final_path, quality_resolved.as_deref())
                    .await?;
                (events)(DownloadEvent::Completed {
                    vod_id: vod_id.to_owned(),
                    final_path,
                });
                Ok(())
            }
            Err(PipelineErr { reason, retryable }) => {
                let attempts = self.bump_attempts(vod_id).await?;
                let next = if retryable && attempts < MAX_ATTEMPTS {
                    DownloadState::FailedRetryable
                } else {
                    DownloadState::FailedPermanent
                };
                self.set_state(vod_id, next, Some(reason.clone())).await?;
                (events)(DownloadEvent::StateChanged {
                    vod_id: vod_id.to_owned(),
                    state: next,
                });
                (events)(DownloadEvent::Failed {
                    vod_id: vod_id.to_owned(),
                    reason: reason.clone(),
                });
                Ok(())
            }
        }
    }

    /// The actual download → post-process pipeline.
    async fn pipeline_inner(
        self: &Arc<Self>,
        events: &DownloadEventSink,
        vod_id: &str,
    ) -> Result<PipelineOk, PipelineErr> {
        let settings = self.settings.get().await.map_err(|e| internal_err(&e))?;
        let library_root = match settings.library_root.as_deref() {
            Some(p) => PathBuf::from(p),
            None => {
                return Err(PipelineErr {
                    reason: "library root not configured".into(),
                    retryable: false,
                });
            }
        };
        let staging_root = settings
            .staging_path
            .as_deref()
            .map(PathBuf::from)
            .unwrap_or_else(|| self.default_staging.clone());

        let vod = self.vods.get(vod_id).await.map_err(|e| PipelineErr {
            reason: format!("load vod: {e}"),
            retryable: false,
        })?;
        if vod.vod.is_sub_only {
            return Err(PipelineErr {
                reason: reason::SUB_ONLY.into(),
                retryable: false,
            });
        }

        // Ask yt-dlp for source metadata so we can preflight + resolve quality.
        let info = self
            .ytdlp
            .fetch_info(&VodInfoRequest {
                url: vod.vod.url.clone(),
            })
            .await
            .map_err(|e| PipelineErr {
                reason: format!("fetch_info: {e}"),
                retryable: true,
            })?;
        let resolved = resolve_preset(
            settings.quality_preset,
            info.height.unwrap_or(0),
            info.fps.unwrap_or(0),
        );

        let estimated = size_estimate(&info).unwrap_or(512 * 1024 * 1024);
        check_preflight(
            self.space_probe.as_ref(),
            &staging_root,
            &library_root,
            estimated,
        )
        .await
        .map_err(|e| match e {
            AppError::DiskFull { .. } => PipelineErr {
                reason: reason::DISK_FULL.into(),
                retryable: false,
            },
            other => PipelineErr {
                reason: format!("preflight: {other}"),
                retryable: true,
            },
        })?;

        // Build the output spec.
        let stem = sanitize_component(vod_id);
        tokio::fs::create_dir_all(&staging_root)
            .await
            .map_err(|e| PipelineErr {
                reason: format!("staging mkdir: {e}"),
                retryable: true,
            })?;
        let no_part = path_is_on_sync_provider(&staging_root);
        let spec = DownloadSpec {
            url: vod.vod.url.clone(),
            output_dir: staging_root.clone(),
            output_stem: stem.clone(),
            format_selector: resolved.format_selector().into(),
            limit_rate_bps: self.rate.per_worker_bps(),
            no_part,
        };

        // Progress pipe — bounded, throttled at 2 Hz per download.
        let (tx_prog, mut rx_prog) = mpsc::channel::<DownloadProgress>(32);
        let vod_id_for_progress = vod_id.to_owned();
        let db = self.db.clone();
        let events_clone = events.clone();
        let progress_task = tokio::spawn(async move {
            let mut last_emit = Instant::now() - Duration::from_secs(1);
            while let Some(p) = rx_prog.recv().await {
                // Persist small — throttle DB writes too.
                if last_emit.elapsed() >= Duration::from_millis(500) {
                    let _ = sqlx::query(
                        "UPDATE downloads SET bytes_done = ?, bytes_total = COALESCE(?, bytes_total),
                              speed_bps = ?, eta_seconds = ?
                         WHERE vod_id = ?",
                    )
                    .bind(p.bytes_done as i64)
                    .bind(p.bytes_total.map(|b| b as i64))
                    .bind(p.speed_bps.map(|b| b as i64))
                    .bind(p.eta_seconds.map(|b| b as i64))
                    .bind(&vod_id_for_progress)
                    .execute(db.pool())
                    .await;
                    (events_clone)(DownloadEvent::Progress {
                        vod_id: vod_id_for_progress.clone(),
                        progress: p,
                    });
                    last_emit = Instant::now();
                }
            }
        });

        let result = self
            .ytdlp
            .download(&spec, tx_prog)
            .await
            .map_err(|e| PipelineErr {
                reason: format!("{}: {e}", reason::YTDLP_EXIT),
                retryable: true,
            })?;
        let _ = progress_task.await;

        // Post-process: remux if needed, thumbnail, atomic move.
        let staging_output = find_output_file(&staging_root, &stem)
            .await
            .unwrap_or(result.output_path.clone());
        let layout = build_layout(settings.library_layout);
        let view = VodWithStreamer {
            vod: &vod.vod,
            streamer_display_name: &vod.streamer_display_name,
            streamer_login: &vod.streamer_login,
        };
        let relative_final = layout.path_for(&view);
        let final_path = library_root.join(&relative_final);
        // Defence-in-depth: the sanitizer already strips path
        // separators and `..`, but assert the composed path cannot
        // escape the library root. A violation here is a bug in the
        // sanitizer or the layout impl, not attacker input.
        if !final_path.starts_with(&library_root) {
            return Err(PipelineErr {
                reason: format!(
                    "final_path {:?} escapes library_root {:?}",
                    final_path, library_root
                ),
                retryable: false,
            });
        }

        // Ensure .mp4 container.
        let mp4_staging = if already_mp4(&staging_output) {
            staging_output.clone()
        } else {
            let remuxed = staging_output.with_extension("mp4");
            self.ffmpeg
                .remux_to_mp4(&RemuxSpec {
                    source: staging_output.clone(),
                    destination: remuxed.clone(),
                })
                .await
                .map_err(|e| PipelineErr {
                    reason: format!("remux: {e}"),
                    retryable: true,
                })?;
            let _ = tokio::fs::remove_file(&staging_output).await;
            remuxed
        };

        // Thumbnail.
        let thumbnail_relative = layout.thumbnail_path(&view);
        let thumbnail_abs = library_root.join(&thumbnail_relative);
        let thumb_staging = staging_root.join(format!("{stem}-thumb.jpg"));
        if let Err(e) = self
            .ffmpeg
            .extract_thumbnail(&ThumbnailSpec {
                source: mp4_staging.clone(),
                destination: thumb_staging.clone(),
                duration_seconds: vod.vod.duration_seconds,
                percent: 10.0,
            })
            .await
        {
            debug!(error = %e, "thumbnail extraction failed; continuing without one");
        }

        // Preview frames for the library grid hover shimmer (Phase 5).
        // We extract to staging and move each file into the library
        // alongside the thumbnail. If any frame fails we delete the
        // partial set — the renderer falls back to the single thumb.
        let preview_abs_paths: Vec<std::path::PathBuf> = layout
            .preview_frame_paths(&view)
            .into_iter()
            .map(|p| library_root.join(p))
            .collect();
        let preview_staging: Vec<std::path::PathBuf> = (1..=PREVIEW_FRAME_PERCENTS.len())
            .map(|i| staging_root.join(format!("{stem}-preview-{i:02}.jpg")))
            .collect();
        let preview_frames: Vec<(f64, std::path::PathBuf)> = PREVIEW_FRAME_PERCENTS
            .iter()
            .copied()
            .zip(preview_staging.iter().cloned())
            .collect();
        if let Err(e) = self
            .ffmpeg
            .extract_preview_frames(&PreviewFramesSpec {
                source: mp4_staging.clone(),
                duration_seconds: vod.vod.duration_seconds,
                frames: preview_frames,
            })
            .await
        {
            debug!(error = %e, "preview frame extraction failed; falling back to single thumb");
            for p in &preview_staging {
                let _ = tokio::fs::remove_file(p).await;
            }
        }

        // Atomic move of the mp4 into the library.
        atomic_move(&mp4_staging, &final_path)
            .await
            .map_err(|e| PipelineErr {
                reason: format!("move: {e}"),
                retryable: true,
            })?;
        if thumb_staging.exists() {
            let _ = atomic_move(&thumb_staging, &thumbnail_abs).await;
        }
        // Move preview frames if all were produced; a partial set is
        // discarded to keep the rendering contract simple.
        let all_frames_written = preview_staging.iter().all(|p| p.exists());
        if all_frames_written && preview_staging.len() == preview_abs_paths.len() {
            for (src, dst) in preview_staging.iter().zip(preview_abs_paths.iter()) {
                let _ = atomic_move(src, dst).await;
            }
        } else {
            for p in &preview_staging {
                let _ = tokio::fs::remove_file(p).await;
            }
        }

        // Sidecars — NFO for the plex layout.
        if matches!(settings.library_layout, LibraryLayoutKind::Plex) {
            let nfo = generate_nfo(&NfoInput {
                vod: &vod.vod,
                chapters: &vod.chapters,
                streamer_display_name: &vod.streamer_display_name,
            });
            let nfo_path = final_path.with_extension("nfo");
            if let Err(e) = tokio::fs::write(&nfo_path, nfo.as_bytes()).await {
                debug!(error = %e, "nfo write failed; continuing");
            }
        }

        Ok(PipelineOk {
            final_path,
            quality_resolved: Some(resolved.as_db_str().into()),
        })
    }

    async fn mark_completed(
        &self,
        vod_id: &str,
        final_path: &Path,
        quality_resolved: Option<&str>,
    ) -> Result<(), AppError> {
        let now = self.clock.unix_seconds();
        sqlx::query(
            "UPDATE downloads
             SET state = 'completed', finished_at = ?,
                 final_path = ?, quality_resolved = COALESCE(?, quality_resolved),
                 last_error = NULL, last_error_at = NULL
             WHERE vod_id = ?",
        )
        .bind(now)
        .bind(final_path.display().to_string())
        .bind(quality_resolved)
        .bind(vod_id)
        .execute(self.db.pool())
        .await?;
        // S5 convergence: bring vods.status from 'downloading'
        // (or 'queued', if the worker raced past it) to 'ready'.
        // Idempotent — ready/archived stay where they are because
        // they are not in the WHERE filter.
        sync_vod_status(self.db.pool(), vod_id, "ready", &["downloading", "queued"]).await?;
        Ok(())
    }

    async fn bump_attempts(&self, vod_id: &str) -> Result<i64, AppError> {
        let attempts: i64 = sqlx::query_scalar(
            "UPDATE downloads SET attempts = attempts + 1 WHERE vod_id = ?
             RETURNING attempts",
        )
        .bind(vod_id)
        .fetch_one(self.db.pool())
        .await?;
        Ok(attempts)
    }

    async fn set_state(
        &self,
        vod_id: &str,
        state: DownloadState,
        last_error: Option<String>,
    ) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE downloads
             SET state = ?,
                 last_error = COALESCE(?, last_error),
                 last_error_at = CASE WHEN ? IS NOT NULL THEN ?
                                      ELSE last_error_at END
             WHERE vod_id = ?",
        )
        .bind(state.as_db_str())
        .bind(&last_error)
        .bind(&last_error)
        .bind(self.clock.unix_seconds())
        .bind(vod_id)
        .execute(self.db.pool())
        .await?;
        // S5 convergence: mirror selected downloads.state changes
        // onto vods.status so the pull-mode state machine and the
        // legacy auto-mode queue stay aligned.  See ADR-0030 for
        // the canonical machine.  We only touch the vods row when
        // the source state in vods is one we expect — never blow
        // over `archived`, `deleted`, or `available` rows.
        match state {
            DownloadState::Downloading => {
                sync_vod_status(
                    self.db.pool(),
                    vod_id,
                    "downloading",
                    &["queued", "downloading"],
                )
                .await?;
            }
            DownloadState::FailedPermanent => {
                // Pull-mode UX: a permanent failure rolls vods.status
                // back to 'queued' so the user's retry through the
                // Downloads UI lands cleanly without a state-machine
                // contradiction.  We don't go all the way back to
                // 'available' because the user still wants this VOD
                // — they just need a retry.
                sync_vod_status(self.db.pool(), vod_id, "queued", &["downloading", "queued"])
                    .await?;
            }
            DownloadState::Queued
            | DownloadState::Paused
            | DownloadState::Completed
            | DownloadState::FailedRetryable => {
                // Queued: handled by `enqueue` (which gets called via
                // the distribution sink).  Paused: doesn't change
                // vods.status — the file is still committed.
                // Completed: handled by `mark_completed`.  Retryable:
                // worker will pick it up on the next tick; vods.status
                // remains as-is (likely still 'downloading').
            }
        }
        Ok(())
    }
}

/// Update `vods.status` to `target` only when the row's current
/// status is in the allow-list `valid_from`.  Idempotent and safe
/// against missing vod rows (UPDATE WHERE returns 0 rows-affected).
/// Public crate-internal helper because both `enqueue` (which is
/// called from the IPC layer) and the worker transitions need it.
async fn sync_vod_status(
    pool: &sqlx::SqlitePool,
    vod_id: &str,
    target: &str,
    valid_from: &[&str],
) -> Result<(), AppError> {
    let placeholders = std::iter::repeat_n("?", valid_from.len())
        .collect::<Vec<_>>()
        .join(",");
    let sql = format!(
        "UPDATE vods SET status = ? WHERE twitch_video_id = ? AND status IN ({placeholders})"
    );
    let mut q = sqlx::query(&sql).bind(target).bind(vod_id);
    for s in valid_from {
        q = q.bind(*s);
    }
    q.execute(pool).await?;
    Ok(())
}

// -------------------------------------------------------------------
//  SQL + row-mapping helpers
// -------------------------------------------------------------------

/// Shared column list for reading a download row plus the denormalised
/// VOD + streamer fields the UI renders. Kept as a const so tests can
/// bind against the same layout without re-typing the column set.
const DOWNLOAD_COLUMNS: &str = "
    d.vod_id, d.state, d.priority, d.quality_preset, d.quality_resolved,
    d.staging_path, d.final_path, d.bytes_total, d.bytes_done,
    d.speed_bps, d.eta_seconds, d.attempts, d.last_error, d.last_error_at,
    d.queued_at, d.started_at, d.finished_at, d.pause_requested,
    v.title, s.display_name, s.login, v.thumbnail_url
";

const DOWNLOAD_LIST_BASE: &str = "
    SELECT
        d.vod_id, d.state, d.priority, d.quality_preset, d.quality_resolved,
        d.staging_path, d.final_path, d.bytes_total, d.bytes_done,
        d.speed_bps, d.eta_seconds, d.attempts, d.last_error, d.last_error_at,
        d.queued_at, d.started_at, d.finished_at, d.pause_requested,
        v.title, s.display_name, s.login, v.thumbnail_url
    FROM downloads d
    JOIN vods v ON v.twitch_video_id = d.vod_id
    JOIN streamers s ON s.twitch_user_id = v.twitch_user_id
";

const DOWNLOAD_SELECT_SQL: &str = "
    SELECT
        d.vod_id, d.state, d.priority, d.quality_preset, d.quality_resolved,
        d.staging_path, d.final_path, d.bytes_total, d.bytes_done,
        d.speed_bps, d.eta_seconds, d.attempts, d.last_error, d.last_error_at,
        d.queued_at, d.started_at, d.finished_at, d.pause_requested,
        v.title, s.display_name, s.login, v.thumbnail_url
    FROM downloads d
    JOIN vods v ON v.twitch_video_id = d.vod_id
    JOIN streamers s ON s.twitch_user_id = v.twitch_user_id
    WHERE d.vod_id = ?
";

fn row_to_download(r: &sqlx::sqlite::SqliteRow) -> Result<DownloadRow, AppError> {
    let state_str: String = r.try_get(1)?;
    let state = DownloadState::from_db_str(&state_str).ok_or_else(|| AppError::Download {
        detail: format!("unknown state {state_str}"),
    })?;
    let quality_preset_str: String = r.try_get(3)?;
    let quality_preset =
        QualityPreset::from_db_str(&quality_preset_str).ok_or_else(|| AppError::Download {
            detail: format!("unknown preset {quality_preset_str}"),
        })?;
    let pause_requested: i64 = r.try_get(17)?;
    Ok(DownloadRow {
        vod_id: r.try_get(0)?,
        state,
        priority: r.try_get(2)?,
        quality_preset,
        quality_resolved: r.try_get(4)?,
        staging_path: r.try_get(5)?,
        final_path: r.try_get(6)?,
        bytes_total: r.try_get(7)?,
        bytes_done: r.try_get(8)?,
        speed_bps: r.try_get(9)?,
        eta_seconds: r.try_get(10)?,
        attempts: r.try_get(11)?,
        last_error: r.try_get(12)?,
        last_error_at: r.try_get(13)?,
        queued_at: r.try_get(14)?,
        started_at: r.try_get(15)?,
        finished_at: r.try_get(16)?,
        pause_requested: pause_requested != 0,
        title: r.try_get(18)?,
        streamer_display_name: r.try_get(19)?,
        streamer_login: r.try_get(20)?,
        thumbnail_url: r.try_get(21)?,
    })
}

#[allow(dead_code)]
fn columns_unused_reference() -> &'static str {
    DOWNLOAD_COLUMNS
}

async fn current_concurrency(svc: &DownloadQueueService) -> i64 {
    svc.settings
        .get()
        .await
        .map(|s| s.max_concurrent_downloads.clamp(1, 5))
        .unwrap_or(2)
}

/// Tiny heuristic: Proton Drive / Dropbox / iCloud paths contain well-
/// known directory names. The queue passes `--no-part` so yt-dlp
/// doesn't rely on rename semantics that the provider intercepts.
fn path_is_on_sync_provider(p: &Path) -> bool {
    let s = p.to_string_lossy().to_lowercase();
    s.contains("/cloudstorage/") || s.contains("dropbox") || s.contains("icloud")
}

async fn find_output_file(dir: &Path, stem: &str) -> Option<PathBuf> {
    let mut rd = tokio::fs::read_dir(dir).await.ok()?;
    let mut best: Option<(PathBuf, std::time::SystemTime)> = None;
    while let Ok(Some(entry)) = rd.next_entry().await {
        let path = entry.path();
        if path
            .file_stem()
            .and_then(|n| n.to_str())
            .is_some_and(|s| s == stem)
        {
            let modified = entry
                .metadata()
                .await
                .and_then(|m| m.modified())
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
            match &best {
                None => best = Some((path, modified)),
                Some((_, prev)) if modified > *prev => best = Some((path, modified)),
                _ => {}
            }
        }
    }
    best.map(|(p, _)| p)
}

fn internal_err(e: &AppError) -> PipelineErr {
    PipelineErr {
        reason: e.to_string(),
        retryable: true,
    }
}

struct PipelineOk {
    final_path: PathBuf,
    quality_resolved: Option<String>,
}

struct PipelineErr {
    reason: String,
    retryable: bool,
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::infra::clock::FixedClock;
    use crate::infra::ffmpeg::fake::{FfmpegFake, FfmpegScript};
    use crate::infra::fs::space::FakeFreeSpace;
    use crate::infra::ytdlp::fake::{FakeScript, YtDlpFake};
    use crate::services::settings::SettingsService;
    use crate::services::vods::VodReadService;

    async fn setup_service() -> (Arc<DownloadQueueService>, Db) {
        let db = Db::open_in_memory().await.unwrap();
        db.migrate().await.unwrap();
        let clock: Arc<dyn Clock> = Arc::new(FixedClock::at(1_000_000));
        let settings = SettingsService::new(db.clone(), clock.clone());
        let svc = DownloadQueueService::new(
            db.clone(),
            clock.clone(),
            Arc::new(YtDlpFake::new(FakeScript::default())),
            Arc::new(FfmpegFake::new(FfmpegScript::default())),
            Arc::new(FakeFreeSpace(u64::MAX)),
            Arc::new(GlobalRate::new()),
            settings,
            Arc::new(VodReadService::new(db.clone())),
            std::env::temp_dir().join("sightline-test-staging"),
        );
        (Arc::new(svc), db)
    }

    async fn seed_streamer_and_vod(db: &Db) {
        sqlx::query(
            "INSERT INTO streamers (twitch_user_id, login, display_name,
                 broadcaster_type, twitch_created_at, added_at)
             VALUES ('100', 'sampler', 'Sampler', '', 0, 0)",
        )
        .execute(db.pool())
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO vods (twitch_video_id, twitch_user_id, title, stream_started_at,
                 published_at, url, duration_seconds, ingest_status, first_seen_at, last_seen_at)
             VALUES ('v1', '100', 'title', 1, 1, 'https://twitch.tv/videos/v1', 1800, 'eligible', 0, 0)",
        )
        .execute(db.pool())
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn enqueue_then_get_roundtrips() {
        let (svc, db) = setup_service().await;
        seed_streamer_and_vod(&db).await;
        let row = svc.enqueue("v1", None).await.unwrap();
        assert_eq!(row.state, DownloadState::Queued);
        assert_eq!(row.priority, 100);
        let fetched = svc.get("v1").await.unwrap().unwrap();
        assert_eq!(fetched.vod_id, "v1");
    }

    /// Helper for the S5 convergence tests: read the current
    /// vods.status string for a given vod_id.
    async fn vod_status(db: &Db, vod_id: &str) -> String {
        sqlx::query_scalar::<_, String>("SELECT status FROM vods WHERE twitch_video_id = ?")
            .bind(vod_id)
            .fetch_one(db.pool())
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn enqueue_promotes_vods_status_from_available_to_queued() {
        // Pure auto-mode path: poller seeds an 'available' row, the
        // legacy enqueue path (no distribution.pick_vod step) should
        // still drag vods.status forward so the Library UI's status
        // badges agree with reality.
        let (svc, db) = setup_service().await;
        seed_streamer_and_vod(&db).await;
        assert_eq!(vod_status(&db, "v1").await, "available");
        svc.enqueue("v1", None).await.unwrap();
        assert_eq!(vod_status(&db, "v1").await, "queued");
    }

    #[tokio::test]
    async fn enqueue_preserves_terminal_vods_status() {
        // If a vod row is already 'archived' (user watched it
        // post-cleanup-cycle, then somehow re-enqueued via the
        // legacy path), we must not blow over that signal.
        let (svc, db) = setup_service().await;
        seed_streamer_and_vod(&db).await;
        sqlx::query("UPDATE vods SET status = 'archived' WHERE twitch_video_id = 'v1'")
            .execute(db.pool())
            .await
            .unwrap();
        svc.enqueue("v1", None).await.unwrap();
        assert_eq!(vod_status(&db, "v1").await, "archived");
    }

    #[tokio::test]
    async fn set_state_downloading_promotes_vods_status() {
        let (svc, db) = setup_service().await;
        seed_streamer_and_vod(&db).await;
        svc.enqueue("v1", None).await.unwrap();
        assert_eq!(vod_status(&db, "v1").await, "queued");
        svc.set_state("v1", DownloadState::Downloading, None)
            .await
            .unwrap();
        assert_eq!(vod_status(&db, "v1").await, "downloading");
    }

    #[tokio::test]
    async fn mark_completed_advances_vods_status_to_ready() {
        let (svc, db) = setup_service().await;
        seed_streamer_and_vod(&db).await;
        svc.enqueue("v1", None).await.unwrap();
        svc.set_state("v1", DownloadState::Downloading, None)
            .await
            .unwrap();
        svc.mark_completed("v1", Path::new("/tmp/v1.mp4"), Some("720p30"))
            .await
            .unwrap();
        assert_eq!(vod_status(&db, "v1").await, "ready");
    }

    #[tokio::test]
    async fn failed_permanent_rolls_vods_status_back_to_queued() {
        // ADR-0030 risk mitigation: a permanent failure must leave
        // vods.status in a state the user can act on (retry).  We
        // roll back to 'queued' so a retry from the Downloads UI
        // doesn't see a stale 'downloading' row.
        let (svc, db) = setup_service().await;
        seed_streamer_and_vod(&db).await;
        svc.enqueue("v1", None).await.unwrap();
        svc.set_state("v1", DownloadState::Downloading, None)
            .await
            .unwrap();
        assert_eq!(vod_status(&db, "v1").await, "downloading");
        svc.set_state(
            "v1",
            DownloadState::FailedPermanent,
            Some("disk full".into()),
        )
        .await
        .unwrap();
        assert_eq!(vod_status(&db, "v1").await, "queued");
    }

    #[tokio::test]
    async fn paused_state_does_not_touch_vods_status() {
        // Paused leaves the file on disk; the worker can resume it
        // later via the existing transition.  The vods row should
        // not flicker on the Library UI as a result.
        let (svc, db) = setup_service().await;
        seed_streamer_and_vod(&db).await;
        svc.enqueue("v1", None).await.unwrap();
        svc.set_state("v1", DownloadState::Downloading, None)
            .await
            .unwrap();
        svc.set_state("v1", DownloadState::Paused, None)
            .await
            .unwrap();
        // Should remain 'downloading' — the file exists, the user
        // has paused but not abandoned.
        assert_eq!(vod_status(&db, "v1").await, "downloading");
    }

    #[tokio::test]
    async fn enqueue_is_idempotent() {
        let (svc, db) = setup_service().await;
        seed_streamer_and_vod(&db).await;
        let first = svc.enqueue("v1", Some(50)).await.unwrap();
        let second = svc.enqueue("v1", Some(200)).await.unwrap();
        assert_eq!(
            first.priority, second.priority,
            "second enqueue should not change priority"
        );
    }

    #[tokio::test]
    async fn transitions_respect_state_machine() {
        let (svc, db) = setup_service().await;
        seed_streamer_and_vod(&db).await;
        svc.enqueue("v1", None).await.unwrap();
        // Can't pause a queued row.
        let err = svc.pause("v1").await.unwrap_err();
        assert!(matches!(err, AppError::Download { .. }));
        // Cancelling a queued row lands in FailedPermanent.
        let row = svc.cancel("v1").await.unwrap();
        assert_eq!(row.state, DownloadState::FailedPermanent);
        assert_eq!(row.last_error.as_deref(), Some(reason::USER_CANCELLED));
    }

    #[tokio::test]
    async fn retry_resets_attempts_and_requeues() {
        let (svc, db) = setup_service().await;
        seed_streamer_and_vod(&db).await;
        svc.enqueue("v1", None).await.unwrap();
        svc.cancel("v1").await.unwrap();
        let row = svc.retry("v1").await.unwrap();
        assert_eq!(row.state, DownloadState::Queued);
        assert_eq!(row.attempts, 0);
        assert_eq!(row.bytes_done, 0);
    }

    #[tokio::test]
    async fn list_filters_by_state() {
        let (svc, db) = setup_service().await;
        seed_streamer_and_vod(&db).await;
        svc.enqueue("v1", None).await.unwrap();
        let all = svc.list(&DownloadFilters::default()).await.unwrap();
        assert_eq!(all.len(), 1);
        let queued = svc
            .list(&DownloadFilters {
                state: Some(DownloadState::Queued),
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(queued.len(), 1);
        let done = svc
            .list(&DownloadFilters {
                state: Some(DownloadState::Completed),
                ..Default::default()
            })
            .await
            .unwrap();
        assert!(done.is_empty());
    }

    #[tokio::test]
    async fn crash_recover_resets_downloading_rows() {
        let (svc, db) = setup_service().await;
        seed_streamer_and_vod(&db).await;
        svc.enqueue("v1", None).await.unwrap();
        sqlx::query("UPDATE downloads SET state = 'downloading' WHERE vod_id = 'v1'")
            .execute(db.pool())
            .await
            .unwrap();
        let moved = svc.crash_recover().await.unwrap();
        assert_eq!(moved, 1);
        let row = svc.get("v1").await.unwrap().unwrap();
        assert_eq!(row.state, DownloadState::Queued);
    }

    #[tokio::test]
    async fn reprioritize_updates_priority() {
        let (svc, db) = setup_service().await;
        seed_streamer_and_vod(&db).await;
        svc.enqueue("v1", Some(100)).await.unwrap();
        let row = svc.reprioritize("v1", 500).await.unwrap();
        assert_eq!(row.priority, 500);
    }
}
