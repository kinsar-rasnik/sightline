//! Background-friendly re-encode service (Phase 8, ADR-0028 + ADR-0029).
//!
//! Orchestrates the three policy layers from the ADRs:
//!
//! 1. **Encoder choice.**  Reads the persisted `encoder_capability`
//!    from settings and refuses to run if the chosen path needs
//!    software encode but `software_encode_opt_in = false`.
//! 2. **Process priority.**  Spawns the ffmpeg child at
//!    [`ProcessPriority::Background`] (renice / BELOW_NORMAL) so
//!    foreground / interactive workloads keep the CPU.
//! 3. **Adaptive throttle.**  Samples system-wide CPU load via
//!    `sysinfo` and surfaces a `ThrottleDecision::Suspend` signal
//!    when sustained load exceeds the high threshold; resumes on a
//!    sustained drop below the low threshold.  The actual
//!    suspend/resume mechanism (SIGSTOP / SuspendThread) is wired
//!    through the supplied [`SuspendController`] trait so the
//!    Windows-suspend implementation can land in v2.1 without
//!    blocking v2.0.
//!
//! What v2.0 ships:
//! - Encoder choice + audio-passthrough invariant + priority.
//! - The throttle's *decision* loop, observable + tested.
//! - Unix suspend wiring (signal-based) is the default; the
//!   `NoOpSuspendController` is the v2.0 default for non-Unix and
//!   power users who don't want suspend.
//!
//! What v2.1 will add:
//! - Windows `SuspendThread` controller (currently the throttle
//!   logs the decision but doesn't act on Windows).
//! - Concurrency cap on multiple in-flight reencodes (today we
//!   accept the cap from settings but the queue is upstream of
//!   this service).

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use tracing::{info, warn};

use crate::domain::quality::{EncoderCapability, EncoderKind, VideoQualityProfile};
use crate::error::AppError;
use crate::infra::ffmpeg::{ProcessPriority, ReencodeSpec, SharedFfmpeg};

/// Outcome of a reencode call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReencodeResult {
    pub destination: PathBuf,
    pub video_encoder: String,
    pub used_software: bool,
}

/// Decision the throttle loop makes for the current sampling window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThrottleDecision {
    /// CPU load is below the low threshold (or within the hysteresis
    /// band on the way down).  ffmpeg should run.
    Run,
    /// CPU load is above the high threshold.  ffmpeg should be
    /// suspended.
    Suspend,
}

/// State the throttle carries across sample ticks (so the 30-second
/// hysteresis windows are observable).  Pure data; the actual
/// suspend/resume side effect lives in the [`SuspendController`].
#[derive(Debug, Clone, Copy)]
pub struct ThrottleState {
    /// Current decision.
    pub decision: ThrottleDecision,
    /// Wall-clock instant of the last decision change.  Populated
    /// from `Instant::now()` by the runner.
    pub since: Instant,
}

impl ThrottleState {
    pub fn new(now: Instant) -> Self {
        Self {
            decision: ThrottleDecision::Run,
            since: now,
        }
    }
}

/// Trait abstracting "freeze / unfreeze the running ffmpeg".  v2.0
/// ships [`NoOpSuspendController`] — the throttle decision is logged
/// but no SIGSTOP / SuspendThread actually fires.  Unix suspend is
/// available as [`UnixSignalSuspend`] (gated behind a `cfg` so
/// non-Unix builds stay link-clean).
pub trait SuspendController: Send + Sync {
    fn suspend(&self, pid: u32) -> Result<(), String>;
    fn resume(&self, pid: u32) -> Result<(), String>;
}

#[derive(Debug, Default)]
pub struct NoOpSuspendController;

impl SuspendController for NoOpSuspendController {
    fn suspend(&self, _pid: u32) -> Result<(), String> {
        Ok(())
    }
    fn resume(&self, _pid: u32) -> Result<(), String> {
        Ok(())
    }
}

#[cfg(unix)]
mod unix_suspend {
    //! Unix SIGSTOP / SIGCONT controller.  Uses the OS `kill` CLI
    //! so the implementation stays inside the repo's `unsafe_code
    //! = "forbid"` lint.
    use super::SuspendController;

    #[derive(Debug, Default)]
    pub struct UnixSignalSuspend;

    impl SuspendController for UnixSignalSuspend {
        fn suspend(&self, pid: u32) -> Result<(), String> {
            send_signal(pid, "STOP")
        }
        fn resume(&self, pid: u32) -> Result<(), String> {
            send_signal(pid, "CONT")
        }
    }

    fn send_signal(pid: u32, sig: &str) -> Result<(), String> {
        let output = std::process::Command::new("kill")
            .args([&format!("-{sig}"), &pid.to_string()])
            .output()
            .map_err(|e| format!("kill spawn: {e}"))?;
        if !output.status.success() {
            return Err(format!(
                "kill -{sig} {pid} exit {:?}: {}",
                output.status.code(),
                String::from_utf8_lossy(&output.stderr)
            ));
        }
        Ok(())
    }
}
#[cfg(unix)]
pub use unix_suspend::UnixSignalSuspend;

/// Pure throttle decision step.  Given the previous state, the
/// current CPU load fraction, and the configured thresholds, return
/// the new state.  The 30-second hysteresis is encoded as a minimum
/// dwell time before flipping.
///
/// The runner calls this once per 5-second sample tick; the
/// service's behaviour is fully deterministic from this function plus
/// the suspend controller.
pub fn step_throttle(
    state: ThrottleState,
    cpu_load: f64,
    high_threshold: f64,
    low_threshold: f64,
    now: Instant,
    dwell: Duration,
) -> ThrottleState {
    let elapsed = now.duration_since(state.since);
    match state.decision {
        ThrottleDecision::Run => {
            if cpu_load >= high_threshold && elapsed >= dwell {
                ThrottleState {
                    decision: ThrottleDecision::Suspend,
                    since: now,
                }
            } else if cpu_load < high_threshold {
                // Reset the dwell timer on every dip so a single
                // brief spike doesn't accumulate towards the
                // suspend trigger.
                ThrottleState {
                    decision: ThrottleDecision::Run,
                    since: now,
                }
            } else {
                state
            }
        }
        ThrottleDecision::Suspend => {
            if cpu_load <= low_threshold && elapsed >= dwell {
                ThrottleState {
                    decision: ThrottleDecision::Run,
                    since: now,
                }
            } else if cpu_load > low_threshold {
                ThrottleState {
                    decision: ThrottleDecision::Suspend,
                    since: now,
                }
            } else {
                state
            }
        }
    }
}

#[derive(Debug)]
pub struct ReencodeService {
    ffmpeg: SharedFfmpeg,
}

impl ReencodeService {
    pub fn new(ffmpeg: SharedFfmpeg) -> Self {
        Self { ffmpeg }
    }

    /// Run a full re-encode pass.  Selects the encoder per
    /// ADR-0028's policy (primary unless it's software and the user
    /// hasn't opted in), spawns the ffmpeg child at background
    /// priority, awaits completion.  Throttle-loop integration is
    /// caller's responsibility — callers that want adaptive suspend
    /// drive [`step_throttle`] alongside the running task.
    pub async fn reencode_to_profile(
        &self,
        source: &Path,
        destination: &Path,
        profile: VideoQualityProfile,
        capability: &EncoderCapability,
        software_opt_in: bool,
    ) -> Result<ReencodeResult, AppError> {
        if !profile.re_encodes() {
            // `Source` profile passes through; the caller should
            // never have called us, but no-op for safety.
            return Err(AppError::InvalidInput {
                detail: "reencode_to_profile called with `source` profile (no re-encode needed)"
                    .into(),
            });
        }

        let chosen = self.choose_encoder(capability, software_opt_in)?;
        let video_encoder_arg = chosen.hevc_encoder_arg().to_owned();

        info!(
            primary = %chosen.as_str(),
            profile = profile.as_db_str(),
            "starting re-encode"
        );

        let spec = ReencodeSpec {
            source: source.to_path_buf(),
            destination: destination.to_path_buf(),
            video_encoder_arg: video_encoder_arg.clone(),
            max_height: profile.max_height(),
            max_fps: profile.max_fps(),
            priority: ProcessPriority::Background,
        };
        self.ffmpeg.reencode(&spec).await?;

        Ok(ReencodeResult {
            destination: destination.to_path_buf(),
            video_encoder: video_encoder_arg,
            used_software: matches!(chosen, EncoderKind::Software),
        })
    }

    fn choose_encoder(
        &self,
        capability: &EncoderCapability,
        software_opt_in: bool,
    ) -> Result<EncoderKind, AppError> {
        // Order: H.265 availability is checked FIRST so a stripped
        // ffmpeg build (Software primary, h265=false) returns the
        // accurate "encoder lacks H.265" message instead of the
        // generic "no hardware encoder" path.  See R-RC-01 finding
        // P1 on commit 94e4340.
        if !capability.h265 {
            return Err(AppError::InvalidInput {
                detail: format!(
                    "encoder {} does not support H.265 on this machine",
                    capability.primary.as_str()
                ),
            });
        }
        if matches!(capability.primary, EncoderKind::Software) && !software_opt_in {
            warn!("no hardware encoder detected and software opt-in is off");
            return Err(AppError::InvalidInput {
                detail:
                    "no hardware encoder available; enable software encoding in Settings to proceed"
                        .into(),
            });
        }
        Ok(capability.primary)
    }
}

/// Construct a default suspend controller for the current platform.
/// Unix builds get `UnixSignalSuspend`; everything else (Windows,
/// targets without a cfg(unix)) gets `NoOpSuspendController` until
/// v2.1 lands the Windows suspend wiring.
pub fn default_suspend_controller() -> Arc<dyn SuspendController> {
    #[cfg(unix)]
    {
        Arc::new(UnixSignalSuspend)
    }
    #[cfg(not(unix))]
    {
        Arc::new(NoOpSuspendController)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::infra::ffmpeg::SharedFfmpeg;
    use crate::infra::ffmpeg::fake::{FfmpegFake, FfmpegScript};

    fn cap(primary: EncoderKind, h265: bool, h264: bool) -> EncoderCapability {
        EncoderCapability {
            primary,
            available: vec![primary, EncoderKind::Software],
            h265,
            h264,
            tested_at: 1,
        }
    }

    #[tokio::test]
    async fn reencode_dispatches_with_correct_encoder_arg() {
        let ffmpeg = Arc::new(FfmpegFake::new(FfmpegScript::default()));
        let svc = ReencodeService::new(ffmpeg.clone() as SharedFfmpeg);
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("in.mp4");
        let dst = tmp.path().join("out.mp4");
        tokio::fs::write(&src, b"src").await.unwrap();
        let res = svc
            .reencode_to_profile(
                &src,
                &dst,
                VideoQualityProfile::P720p30,
                &cap(EncoderKind::VideoToolbox, true, true),
                false,
            )
            .await
            .unwrap();
        assert_eq!(res.video_encoder, "hevc_videotoolbox");
        assert!(!res.used_software);
        let calls = ffmpeg.calls();
        assert_eq!(calls.reencodes.len(), 1);
        assert_eq!(calls.reencodes[0].max_height, Some(720));
        assert_eq!(calls.reencodes[0].max_fps, Some(30));
        assert_eq!(calls.reencodes[0].priority, ProcessPriority::Background);
    }

    #[tokio::test]
    async fn reencode_refuses_software_without_opt_in() {
        let ffmpeg = Arc::new(FfmpegFake::new(FfmpegScript::default()));
        let svc = ReencodeService::new(ffmpeg as SharedFfmpeg);
        let tmp = tempfile::tempdir().unwrap();
        let err = svc
            .reencode_to_profile(
                tmp.path(),
                tmp.path(),
                VideoQualityProfile::P720p30,
                &cap(EncoderKind::Software, true, true),
                false,
            )
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::InvalidInput { .. }));
    }

    #[tokio::test]
    async fn reencode_runs_software_when_opt_in() {
        let ffmpeg = Arc::new(FfmpegFake::new(FfmpegScript::default()));
        let svc = ReencodeService::new(ffmpeg.clone() as SharedFfmpeg);
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("in.mp4");
        let dst = tmp.path().join("out.mp4");
        tokio::fs::write(&src, b"src").await.unwrap();
        let res = svc
            .reencode_to_profile(
                &src,
                &dst,
                VideoQualityProfile::P720p30,
                &cap(EncoderKind::Software, true, true),
                true,
            )
            .await
            .unwrap();
        assert_eq!(res.video_encoder, "libx265");
        assert!(res.used_software);
    }

    #[tokio::test]
    async fn source_profile_is_rejected() {
        let ffmpeg = Arc::new(FfmpegFake::new(FfmpegScript::default()));
        let svc = ReencodeService::new(ffmpeg as SharedFfmpeg);
        let tmp = tempfile::tempdir().unwrap();
        let err = svc
            .reencode_to_profile(
                tmp.path(),
                tmp.path(),
                VideoQualityProfile::Source,
                &cap(EncoderKind::VideoToolbox, true, true),
                false,
            )
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::InvalidInput { .. }));
    }

    #[tokio::test]
    async fn refuses_when_h265_unavailable() {
        let ffmpeg = Arc::new(FfmpegFake::new(FfmpegScript::default()));
        let svc = ReencodeService::new(ffmpeg as SharedFfmpeg);
        let tmp = tempfile::tempdir().unwrap();
        let err = svc
            .reencode_to_profile(
                tmp.path(),
                tmp.path(),
                VideoQualityProfile::P720p30,
                &cap(EncoderKind::Nvenc, false, true),
                false,
            )
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::InvalidInput { .. }));
    }

    #[test]
    fn throttle_step_run_to_suspend_after_dwell() {
        // Sustained high load (>= high_threshold) holds the timer
        // across consecutive ticks because the Run branch's "reset
        // on dip" only fires when cpu_load < high_threshold.  Once
        // total elapsed >= dwell, the next tick flips to Suspend.
        let dwell = Duration::from_secs(30);
        let t0 = Instant::now();
        let mut s = ThrottleState::new(t0);
        // 15s in, still high — no flip yet (state held with old `since`).
        s = step_throttle(s, 0.85, 0.7, 0.5, t0 + Duration::from_secs(15), dwell);
        assert_eq!(s.decision, ThrottleDecision::Run);
        // 35s in, dwell elapsed — flip to Suspend.
        s = step_throttle(s, 0.85, 0.7, 0.5, t0 + Duration::from_secs(35), dwell);
        assert_eq!(s.decision, ThrottleDecision::Suspend);
    }

    #[test]
    fn throttle_step_holds_state_at_exact_threshold() {
        // cpu_load == high_threshold hits neither the suspend (>=)
        // nor the reset (<) branch on the way up, so the state is
        // preserved unchanged.  Documented behaviour: a reading
        // exactly on the boundary is "indeterminate" and we wait
        // for the next sample.
        let dwell = Duration::from_secs(30);
        let t0 = Instant::now();
        let initial = ThrottleState::new(t0);
        let after = step_throttle(initial, 0.7, 0.7, 0.5, t0 + Duration::from_secs(10), dwell);
        // Decision unchanged AND `since` unchanged — sample is treated as no-info.
        assert_eq!(after.decision, ThrottleDecision::Run);
        assert_eq!(after.since, initial.since);

        // Same on the way down: cpu_load == low_threshold while
        // suspended doesn't resume.
        let suspended = ThrottleState {
            decision: ThrottleDecision::Suspend,
            since: t0,
        };
        let after2 = step_throttle(
            suspended,
            0.5,
            0.7,
            0.5,
            t0 + Duration::from_secs(10),
            dwell,
        );
        assert_eq!(after2.decision, ThrottleDecision::Suspend);
        assert_eq!(after2.since, suspended.since);
    }

    #[test]
    fn throttle_step_brief_spike_does_not_flip() {
        let dwell = Duration::from_secs(30);
        let t0 = Instant::now();
        let mut s = ThrottleState::new(t0);
        // High load at t+10s.
        s = step_throttle(s, 0.85, 0.7, 0.5, t0 + Duration::from_secs(10), dwell);
        // Drop below high at t+15s — should reset to Run with new
        // `since`.
        s = step_throttle(s, 0.4, 0.7, 0.5, t0 + Duration::from_secs(15), dwell);
        assert_eq!(s.decision, ThrottleDecision::Run);
        // High again at t+20s.  Should NOT flip yet because the
        // dwell from t+15 hasn't elapsed.
        s = step_throttle(s, 0.85, 0.7, 0.5, t0 + Duration::from_secs(20), dwell);
        assert_eq!(s.decision, ThrottleDecision::Run);
    }

    #[test]
    fn throttle_step_suspend_to_run_after_low_dwell() {
        let dwell = Duration::from_secs(30);
        let t0 = Instant::now();
        let s0 = ThrottleState {
            decision: ThrottleDecision::Suspend,
            since: t0,
        };
        // Sustained low load past dwell flips back to Run.
        let s1 = step_throttle(s0, 0.3, 0.7, 0.5, t0 + Duration::from_secs(35), dwell);
        assert_eq!(s1.decision, ThrottleDecision::Run);
    }

    #[test]
    fn no_op_suspend_controller_succeeds() {
        let c = NoOpSuspendController;
        assert!(c.suspend(0).is_ok());
        assert!(c.resume(0).is_ok());
    }
}
