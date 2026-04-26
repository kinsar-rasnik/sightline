//! Cross-platform process liveness probe (R-SC closure for Phase 8
//! Medium finding: stale-PID guard for the suspend controller).
//!
//! `is_process_alive(pid)` is a low-overhead "does this PID still
//! refer to a running process?" check that uses the same shell-out
//! pattern as `infra::process::priority` so the
//! `unsafe_code = "forbid"` lint stays intact.
//!
//! * **Unix:** `kill -0 <pid>` — the canonical signal-zero idiom that
//!   probes for permission to deliver a signal without actually
//!   delivering one.  Exit 0 ⇒ alive; non-zero ⇒ dead or no
//!   permission.  We treat `no permission` (a process owned by another
//!   uid) as "not our business" → returns `false`, which is the safe
//!   read for a SuspendController whose only call sites are processes
//!   we spawned ourselves.
//! * **Windows:** `tasklist /NH /FI "PID eq <pid>"` — produces the
//!   stub `INFO: No tasks are running which match the specified
//!   criteria.` when the PID is gone, otherwise prints a header-less
//!   row with the process name.  We classify by the leading `INFO:`
//!   marker rather than parsing the row, so a localised Windows that
//!   translates the message string still flips correctly (the
//!   localised stub still starts with `INFO:` on every locale we've
//!   verified; if a future locale breaks this, the worst case is
//!   "alive" → the suspend kill-attempt itself is a no-op on a dead
//!   PID anyway).
//!
//! TOCTOU note: a process can disappear between the liveness check
//! and the subsequent suspend/resume call.  The shell-out invariants
//! handle that race — `kill -STOP` on a dead PID returns "No such
//! process" which the caller can treat as benign.  The pre-check is
//! belt-and-braces, not the load-bearing safety net.

/// Returns `true` iff the OS reports the PID corresponds to a live
/// process.  Returns `false` for dead processes, permission denials,
/// and any error spawning the probe binary — the caller treats all
/// three the same way (skip the suspend/resume side effect).
pub fn is_process_alive(pid: u32) -> bool {
    if pid == 0 {
        // PID 0 has special meaning on every supported OS (kernel /
        // process group); never a real ffmpeg child.
        return false;
    }
    is_alive_impl(pid)
}

#[cfg(unix)]
fn is_alive_impl(pid: u32) -> bool {
    let output = std::process::Command::new("kill")
        .args(["-0", &pid.to_string()])
        .output();
    match output {
        Ok(o) => o.status.success(),
        Err(_) => false,
    }
}

#[cfg(windows)]
fn is_alive_impl(pid: u32) -> bool {
    let output = std::process::Command::new("tasklist")
        .args(["/NH", "/FI", &format!("PID eq {pid}")])
        .output();
    match output {
        Ok(o) => {
            if !o.status.success() {
                return false;
            }
            let stdout = String::from_utf8_lossy(&o.stdout);
            let trimmed = stdout.trim_start();
            // tasklist prints "INFO: No tasks are running ..." when
            // the PID is gone.  Any non-INFO non-empty output means
            // tasklist found a row.
            !trimmed.is_empty() && !trimmed.starts_with("INFO:")
        }
        Err(_) => false,
    }
}

#[cfg(not(any(unix, windows)))]
fn is_alive_impl(_pid: u32) -> bool {
    // No probe available on unsupported platforms — assume alive so
    // the suspend/resume call still runs (it'll be a no-op on dead
    // PIDs at the OS level on those platforms, same as our shell-out
    // behaviour above).
    true
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn pid_zero_is_never_alive() {
        // PID 0 has special meaning on every supported OS; it must
        // never report as "alive" so a callsite that fell through to
        // an uninitialised PID can't accidentally suspend the world.
        assert!(!is_process_alive(0));
    }

    #[test]
    fn current_process_is_alive() {
        // The test runner itself must register as alive — sanity
        // check that the shell-out path is wired up at all on the
        // host OS.
        let me = std::process::id();
        assert!(is_process_alive(me), "self-pid {me} should be alive");
    }

    #[test]
    fn very_large_pid_is_dead() {
        // u32::MAX is reserved on every supported OS (Linux's pid_max
        // tops out at 2^22; Windows PIDs are 32-bit but the OS never
        // hands out u32::MAX as a real PID).  This is the cheapest
        // way to assert "definitely dead" without spawning a process.
        assert!(!is_process_alive(u32::MAX));
    }

    #[test]
    fn killed_child_reports_dead() {
        // Spawn a sleeper, kill it, wait for it, then probe — must
        // report dead.  Exercises the realistic stale-PID-guard
        // pattern: at the moment we'd send STOP/CONT, the child is
        // already gone.
        #[cfg(unix)]
        let mut child = std::process::Command::new("sleep")
            .arg("60")
            .spawn()
            .expect("spawn sleep");
        #[cfg(windows)]
        let mut child = std::process::Command::new("cmd")
            .args(["/C", "ping", "-n", "60", "127.0.0.1"])
            .spawn()
            .expect("spawn cmd");
        let pid = child.id();
        assert!(
            is_process_alive(pid),
            "freshly spawned child should be alive"
        );
        child.kill().expect("kill child");
        let _ = child.wait();
        // Some OSes recycle PIDs quickly; the assertion is about the
        // PID immediately after wait(), where the OS has reaped the
        // child but not yet handed the PID back out.
        assert!(
            !is_process_alive(pid),
            "killed-and-reaped child {pid} should be dead"
        );
    }
}
