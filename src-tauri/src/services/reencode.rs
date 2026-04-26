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
//! What v2.0 / v2.0.1 ships:
//! - Encoder choice + audio-passthrough invariant + priority.
//! - The throttle's *decision* loop, observable + tested.
//! - Unix suspend wiring (`SIGSTOP` / `SIGCONT` via `kill`) and
//!   Windows suspend wiring (`NtSuspendProcess` / `NtResumeProcess`
//!   via PowerShell + `Add-Type` P/Invoke — the same
//!   `unsafe_code = "forbid"`-respecting shell-out pattern as
//!   `infra::process::priority`'s `wmic` path).
//! - Stale-PID guard on every suspend/resume (probes
//!   `infra::process::liveness::is_process_alive` first; silent
//!   no-op when the target PID is gone).
//!
//! What v2.1 will add:
//! - Concurrency cap on multiple in-flight reencodes (today we
//!   accept the cap from settings but the queue is upstream of
//!   this service).
//! - Hardware-aware quality-factor calibration.

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

/// Trait abstracting "freeze / unfreeze the running ffmpeg".  Every
/// platform-specific impl runs the stale-PID liveness probe
/// (`infra::process::liveness::is_process_alive`) before issuing the
/// suspend/resume primitive — a process that vanished between the
/// throttle decision and the controller invocation is a benign
/// no-op, not a crash.
///
/// The trait is intentionally PID-shaped (rather than handle-shaped)
/// so the controller can be constructed once at app startup and
/// reused across re-encode workers without threading process handles
/// through the call graph.  TOCTOU between the liveness probe and
/// the OS call is documented in
/// `infra::process::liveness` — both sides handle "process gone" as
/// `Ok(())` to keep the controller fail-open.
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
    //!
    //! The signal name is constrained to a closed enum at the type
    //! level (R-RC-03 hygiene) so a future caller can never widen
    //! this to user-supplied input without first changing the
    //! signature.
    use super::SuspendController;
    use crate::infra::process::liveness::is_process_alive;
    use tracing::debug;

    #[derive(Debug, Clone, Copy)]
    enum UnixSignal {
        Stop,
        Cont,
    }

    impl UnixSignal {
        fn flag(self) -> &'static str {
            match self {
                UnixSignal::Stop => "-STOP",
                UnixSignal::Cont => "-CONT",
            }
        }
    }

    #[derive(Debug, Default)]
    pub struct UnixSignalSuspend;

    impl SuspendController for UnixSignalSuspend {
        fn suspend(&self, pid: u32) -> Result<(), String> {
            send_signal_guarded(pid, UnixSignal::Stop)
        }
        fn resume(&self, pid: u32) -> Result<(), String> {
            send_signal_guarded(pid, UnixSignal::Cont)
        }
    }

    fn send_signal_guarded(pid: u32, sig: UnixSignal) -> Result<(), String> {
        if !is_process_alive(pid) {
            debug!(pid, signal = sig.flag(), "stale PID — suspend/resume no-op");
            return Ok(());
        }
        let flag = sig.flag();
        let output = std::process::Command::new("kill")
            .args([flag, &pid.to_string()])
            .output()
            .map_err(|e| format!("kill spawn: {e}"))?;
        if !output.status.success() {
            // ESRCH-equivalent: the process died between the liveness
            // probe and the kill call.  Match the dead-PID case from
            // the guard above and report success so the throttle
            // loop doesn't escalate.
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("No such process") {
                debug!(pid, signal = flag, "process exited mid-signal");
                return Ok(());
            }
            return Err(format!(
                "kill {flag} {pid} exit {:?}: {}",
                output.status.code(),
                stderr
            ));
        }
        Ok(())
    }
}
#[cfg(unix)]
pub use unix_suspend::UnixSignalSuspend;

#[cfg(windows)]
mod windows_suspend {
    //! Windows `NtSuspendProcess` / `NtResumeProcess` controller.
    //!
    //! Implementation choice: shell out to `powershell.exe` with an
    //! `Add-Type` P/Invoke shim that calls the two `ntdll.dll`
    //! primitives.  This matches the `infra::process::priority`
    //! pattern (`wmic CALL SetPriority`) — same shell-out idiom,
    //! same `unsafe_code = "forbid"` compatibility.  An FFI-direct
    //! implementation via the `windows` crate would require flipping
    //! the workspace lint, which is out of scope for v2.0.1.
    //!
    //! Cost: roughly 0.3-0.8s per call due to PowerShell startup +
    //! C# JIT.  The throttle samples at 5-second intervals so this
    //! is invisible to the user.  A native FFI rewrite is a v2.1
    //! follow-up if support data shows the latency matters.
    use super::SuspendController;
    use crate::infra::process::liveness::is_process_alive;
    use tracing::debug;

    /// Operation requested of the embedded P/Invoke shim.  Closed
    /// enum at the type level so a caller can never widen this to a
    /// user-supplied string (the same R-RC-03 hygiene applied on the
    /// Unix side).
    #[derive(Debug, Clone, Copy)]
    enum NtOp {
        Suspend,
        Resume,
    }

    impl NtOp {
        fn entrypoint(self) -> &'static str {
            match self {
                NtOp::Suspend => "NtSuspendProcess",
                NtOp::Resume => "NtResumeProcess",
            }
        }
    }

    #[derive(Debug, Default)]
    pub struct WindowsSuspend;

    impl SuspendController for WindowsSuspend {
        fn suspend(&self, pid: u32) -> Result<(), String> {
            run_nt_op_guarded(pid, NtOp::Suspend)
        }
        fn resume(&self, pid: u32) -> Result<(), String> {
            run_nt_op_guarded(pid, NtOp::Resume)
        }
    }

    fn run_nt_op_guarded(pid: u32, op: NtOp) -> Result<(), String> {
        if !is_process_alive(pid) {
            debug!(
                pid,
                op = op.entrypoint(),
                "stale PID — suspend/resume no-op"
            );
            return Ok(());
        }
        let script = build_script(pid, op);
        let output = std::process::Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", &script])
            .output()
            .map_err(|e| format!("powershell spawn: {e}"))?;
        if !output.status.success() {
            return Err(format!(
                "powershell {} pid={pid} exit {:?}: {}",
                op.entrypoint(),
                output.status.code(),
                String::from_utf8_lossy(&output.stderr)
            ));
        }
        Ok(())
    }

    /// Construct the PowerShell command that performs the
    /// suspend/resume.  PROCESS_SUSPEND_RESUME = 0x0800 is the
    /// least-privilege access mask sufficient for both NT calls.
    /// The class name (`SightlineSus`) is fresh per PowerShell
    /// process so `Add-Type` won't collide with a pre-existing type
    /// in the AppDomain.
    fn build_script(pid: u32, op: NtOp) -> String {
        format!(
            "$ErrorActionPreference='Stop';\
             $src='[DllImport(\"ntdll.dll\")]public static extern uint NtSuspendProcess(System.IntPtr h);\
             [DllImport(\"ntdll.dll\")]public static extern uint NtResumeProcess(System.IntPtr h);\
             [DllImport(\"kernel32.dll\")]public static extern System.IntPtr OpenProcess(uint a, bool b, int c);\
             [DllImport(\"kernel32.dll\")]public static extern bool CloseHandle(System.IntPtr h);';\
             $p=Add-Type -PassThru -Name SightlineSus -Namespace Sightline -MemberDefinition $src;\
             $h=$p::OpenProcess(0x800, $false, {pid});\
             if($h -eq [System.IntPtr]::Zero){{exit 2}};\
             $rc=$p::{entry}($h);\
             $p::CloseHandle($h)|Out-Null;\
             if($rc -ne 0){{exit 3}};\
             exit 0",
            pid = pid,
            entry = op.entrypoint(),
        )
    }

    #[cfg(test)]
    mod script_tests {
        use super::*;

        #[test]
        fn suspend_script_includes_pid_and_entrypoint() {
            let s = build_script(4242, NtOp::Suspend);
            assert!(
                s.contains(", 4242)"),
                "PID must appear in OpenProcess call: {s}"
            );
            assert!(
                s.contains("NtSuspendProcess($h)"),
                "entrypoint must invoke NtSuspendProcess: {s}"
            );
        }

        #[test]
        fn resume_script_includes_pid_and_entrypoint() {
            let s = build_script(99, NtOp::Resume);
            assert!(s.contains(", 99)"));
            assert!(s.contains("NtResumeProcess($h)"));
        }

        #[test]
        fn script_uses_minimum_access_mask() {
            // 0x800 = PROCESS_SUSPEND_RESUME — least-privilege mask.
            // A regression that broadened this to 0x1F0FFF
            // (PROCESS_ALL_ACCESS) would still work but represents a
            // privilege escalation we don't need.
            let s = build_script(1, NtOp::Suspend);
            assert!(
                s.contains("OpenProcess(0x800,"),
                "must use PROCESS_SUSPEND_RESUME = 0x800: {s}"
            );
        }
    }
}
#[cfg(windows)]
pub use windows_suspend::WindowsSuspend;

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
/// Unix builds get [`UnixSignalSuspend`] (SIGSTOP / SIGCONT via
/// `kill`).  Windows builds get [`WindowsSuspend`] (NtSuspendProcess /
/// NtResumeProcess via PowerShell + Add-Type P/Invoke).  Targets
/// without an OS-specific impl fall back to [`NoOpSuspendController`].
pub fn default_suspend_controller() -> Arc<dyn SuspendController> {
    #[cfg(unix)]
    {
        Arc::new(UnixSignalSuspend)
    }
    #[cfg(windows)]
    {
        Arc::new(WindowsSuspend)
    }
    #[cfg(not(any(unix, windows)))]
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

    /// Stale-PID guard: a controller asked to suspend a PID that
    /// definitely doesn't exist must return Ok(()) — the throttle
    /// loop should never escalate "process already gone" to a
    /// hard error.  Uses `u32::MAX` as the "guaranteed dead" PID
    /// (Linux pid_max ≤ 2^22; Windows never hands out u32::MAX).
    #[cfg(unix)]
    #[test]
    fn unix_signal_suspend_is_silent_on_dead_pid() {
        let c = UnixSignalSuspend;
        assert!(c.suspend(u32::MAX).is_ok());
        assert!(c.resume(u32::MAX).is_ok());
    }

    /// PID 0 is reserved on every supported OS.  The guard must
    /// classify it as dead and skip the OS call entirely.
    #[cfg(unix)]
    #[test]
    fn unix_signal_suspend_pid_zero_is_noop() {
        let c = UnixSignalSuspend;
        assert!(c.suspend(0).is_ok());
        assert!(c.resume(0).is_ok());
    }

    /// Windows side of the stale-PID guard.  Same contract: a dead
    /// PID must never produce an error.  The Windows controller
    /// shells out to `tasklist` for the liveness probe and to
    /// `powershell` for the suspend op; for a known-dead PID only
    /// the probe runs and the function returns early without
    /// touching PowerShell.
    #[cfg(windows)]
    #[test]
    fn windows_suspend_is_silent_on_dead_pid() {
        let c = WindowsSuspend;
        assert!(c.suspend(u32::MAX).is_ok());
        assert!(c.resume(u32::MAX).is_ok());
    }

    #[cfg(windows)]
    #[test]
    fn windows_suspend_pid_zero_is_noop() {
        let c = WindowsSuspend;
        assert!(c.suspend(0).is_ok());
        assert!(c.resume(0).is_ok());
    }

    /// Spawn a real child, kill it, then prove suspend/resume on
    /// the now-dead PID is benign.  This is the realistic
    /// stale-PID scenario: the throttle loop sampled CPU, decided
    /// to suspend, and by the time the controller fires the encode
    /// has already completed and the PID is gone.
    #[cfg(unix)]
    #[test]
    fn unix_signal_suspend_handles_killed_child() {
        let mut child = std::process::Command::new("sleep")
            .arg("60")
            .spawn()
            .unwrap();
        let pid = child.id();
        child.kill().unwrap();
        let _ = child.wait();
        let c = UnixSignalSuspend;
        // Both the liveness probe AND the kill-call ESRCH branch are
        // tolerant of the dead PID.
        assert!(c.suspend(pid).is_ok());
        assert!(c.resume(pid).is_ok());
    }
}
