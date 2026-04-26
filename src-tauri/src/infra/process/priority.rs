//! Cross-platform process scheduling priority helpers (ADR-0029).
//!
//! Layer 1 of the background-friendly re-encode policy: lower the
//! priority of the spawned ffmpeg child so the OS scheduler hands
//! foreground / interactive workloads (the user's game, streaming
//! software, browser, etc.) the CPU first.
//!
//! The repo's `unsafe_code = "forbid"` lint blocks raw FFI calls, so
//! we shell out to the OS-native CLI primitives instead:
//!
//! * **macOS / Linux:** `renice -n 19 -p <pid>`.  Always available
//!   on a desktop install; `nice +19` is the lowest reasonable
//!   priority that still lets the process make progress.
//! * **Windows:** `wmic process where ProcessId=<pid> CALL
//!   SetPriority "Below Normal"`.  WMIC is deprecated but ships
//!   with every Win10/11 image we care about.  The fallback path
//!   if WMIC is removed (Win12+) is documented as a v2.1
//!   follow-up.
//!
//! Adaptive suspend/resume (Layer 2) is handled separately in
//! `services::reencode`; this module only handles the priority
//! lowering done at spawn time.

use crate::infra::ffmpeg::ProcessPriority;

/// Apply the requested priority to the process with the given PID.
/// Returns immediately on `ProcessPriority::Normal`.
///
/// Errors are surfaced as `Err(detail_string)` so the caller (which
/// has logging context) can decide whether to log-and-continue or
/// fail the operation.  In practice, callers log and continue —
/// failure to lower priority is a degraded-mode condition, not a
/// fatal one.
pub fn apply_priority(pid: u32, priority: ProcessPriority) -> Result<(), String> {
    match priority {
        ProcessPriority::Normal => Ok(()),
        ProcessPriority::Background => apply_background(pid),
    }
}

#[cfg(unix)]
fn apply_background(pid: u32) -> Result<(), String> {
    let output = std::process::Command::new("renice")
        .args(["-n", "19", "-p", &pid.to_string()])
        .output()
        .map_err(|e| format!("renice spawn: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "renice exit {:?}: {}",
            output.status.code(),
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(())
}

#[cfg(windows)]
fn apply_background(pid: u32) -> Result<(), String> {
    // wmic CALL SetPriority takes a numeric priority class.
    // 16384 = BELOW_NORMAL_PRIORITY_CLASS.  Constants documented
    // at https://learn.microsoft.com/en-us/windows/win32/cimwin32prov/setpriority-method-in-class-win32-process
    let output = std::process::Command::new("wmic")
        .args([
            "process",
            "where",
            &format!("ProcessId={pid}"),
            "CALL",
            "SetPriority",
            "16384",
        ])
        .output()
        .map_err(|e| format!("wmic spawn: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "wmic exit {:?}: {}",
            output.status.code(),
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(())
}

#[cfg(not(any(unix, windows)))]
fn apply_background(_pid: u32) -> Result<(), String> {
    // No-op on unsupported platforms.  The encode runs at the
    // default priority and the user can rely on OS-native task
    // managers if they need to throttle manually.
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normal_priority_is_a_noop() {
        // Should not invoke any OS command and should return Ok(())
        // for any (even invalid) PID.
        assert!(apply_priority(0, ProcessPriority::Normal).is_ok());
        assert!(apply_priority(u32::MAX, ProcessPriority::Normal).is_ok());
    }
}
