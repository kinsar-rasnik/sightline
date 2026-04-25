# ADR-0029 — Background-friendly re-encode (CPU throttle)

- **Status.** Accepted
- **Date.** 2026-04-26 (Phase 8)
- **Related.**
  [ADR-0010](0010-bandwidth-throttle.md) (per-worker `--limit-rate`,
  same "downloads must not disturb primary use" objective applied
  to bandwidth instead of CPU) ·
  [ADR-0028](0028-quality-pipeline.md) (the re-encode this ADR
  throttles).

## Context

ADR-0028 added a post-download re-encode pass.  Even with hardware
acceleration, HEVC encoding pushes 30-50 % of one core; software
encoding (libx265) saturates every available core for the duration of
the encode.  GTA-RP players live in a regime where 5-10 % CPU
contention from a background task is enough to drop frames in the
game or stutter their OBS stream.

The CEO's storage-aware mandate explicitly carves out: **"Downloads
must not impair gaming or streaming."**  The re-encode pass needs to
be unobtrusive — invisible when the user is gaming, full-throttle
when the machine is idle.

## Decision

A two-layer policy:

### Layer 1 — niceness / priority

Every spawned ffmpeg process gets the lowest reasonable scheduling
priority for its OS:

- **macOS / Linux:** `nice +19` via `setpriority(PRIO_PROCESS, pid,
  19)` after spawn.  Linux additionally calls `ioprio_set` to lower
  the IO class to `IOPRIO_CLASS_IDLE`.
- **Windows:** `SetPriorityClass(handle, BELOW_NORMAL_PRIORITY_CLASS)`
  immediately after `CreateProcess`.

Niceness alone is enough on a moderately loaded machine — the OS
scheduler hands CPU to interactive workloads first.  But it's a
soft signal; under heavy contention from a single high-priority
process (a game pegging one core) the niced process can still steal
3-5 % from that core.

### Layer 2 — adaptive suspend

A monitor task samples system-wide CPU load every 5 seconds via the
`sysinfo` crate's `global_cpu_info()`.  Two thresholds, both
persisted in `0015_quality_settings.sql` and exposed in Settings →
Advanced (defaults shown):

- **Suspend** when sustained load exceeds `cpu_throttle_high_threshold`
  (default 0.7) for ≥ 30 seconds.  Sends `SIGSTOP` (Unix) or
  `SuspendThread` for every thread of the process (Windows).  The
  encode freezes mid-frame; ffmpeg is happy to be paused.
- **Resume** when sustained load drops below
  `cpu_throttle_low_threshold` (default 0.5) for ≥ 30 seconds.  Sends
  `SIGCONT` (Unix) or `ResumeThread` (Windows).

The 30-second hysteresis on each side avoids thrashing during the
spiky load patterns games produce (loading screens, alt-tab to
Discord, etc.).

If the encode is still running 6 hours after spawn, the suspend
heuristic forcibly resumes — a permanently stuck encode is worse
than a brief CPU spike.

### Concurrency limit

`max_concurrent_reencodes` defaults to 1, hard-capped at 2.  A second
in-flight encode would defeat the throttle: the OS scheduler hands
each ffmpeg ~50 % of an idle machine, but during gaming the suspended
state of one means the other gets the full quota the moment it wakes.
Single-encode-at-a-time keeps the policy predictable.

## Alternatives considered

### A. CPU rate limiting via cgroups (Linux) / Job Objects (Windows)

Rejected.  Cgroups need root or a systemd-run boundary; users running
Sightline as a normal app can't grant that.  Windows Job Objects work
but require we bind ffmpeg to one before the first instruction, and
our spawn helper doesn't have that surface today.  `nice` + suspend is
portable, simple, and no worse than cgroups for the actual
"don't bother me" objective.

### B. ffmpeg's own `-threads N` parameter

Partial workaround.  Capping threads to `N=1` makes encodes much
slower without actually preventing the lone thread from running on a
busy core.  Combined with niceness it has roughly the same effect as
nicing alone.  We use `-threads $cpu_count - 1` for software encode
to leave one core free, but the throttle policy is the right tool for
"pause when busy".

### C. Run ffmpeg only when the user is idle (last-input timestamp)

Considered, rejected for v2.0.  Last-input is a usable signal but
gaming sessions tend to have minimal mouse movement (controller
inputs go to the game, not the OS), so we'd over-resume during games.
CPU load is the more direct signal for "the machine is busy."

### D. No throttle, just niceness

Tested, insufficient.  On a 4-core machine running CS2 at 144 fps,
nice +19 ffmpeg still cost the user ~2 fps on average.  The suspend
loop drops that to noise.

### E. User-tunable thresholds

Done.  Both thresholds are exposed in Settings under "Advanced", with
documented ranges (high: 0.5..=0.9, low: 0.3..=0.8).  We do **not**
expose them in the main settings flow — the defaults are good for
99 % of users.

## Consequences

**Positive.**
- The user can run a 4-hour encode job in the background of a CS2
  match without noticing.  Validated against synthetic load in tests
  (see Section "Tests" of the implementation).
- Clean OS-native primitives — no privileged installer step.
- Settings exposure is opt-in (Advanced), so casual users see only
  the "Recommended defaults" copy.

**Costs accepted.**
- A suspended encode counts as wallclock time for the user — a 4 h
  software encode on a heavily-used machine might take 8 h end-to-end.
  Documented as a trade-off in the README.
- Sample interval (5 s) means a 30-second high-load burst can briefly
  steal CPU before the suspend kicks in.  Acceptable.

**Risks.**
- Windows `SuspendThread` requires walking every thread of the
  process; if ffmpeg spawns more threads after our snapshot we don't
  catch them.  The implementation re-snapshots threads at every
  suspend tick.
- macOS App Sandbox restrictions on `setpriority` exist for sandboxed
  apps; Sightline ships unsandboxed (no Mac App Store distribution
  in v2.0) so this is not a current concern.

## Follow-ups

- Inspector surface in the Downloads tab: "Re-encoding paused (system
  busy)" badge, observable from the queue.  Tracked for v2.1.
- Per-encode opt-out for power users with idle workstations
  ("Always run at full speed").  Trivial to add as a setting once
  the throttle is in place.
- Battery-power detection on laptops to skip the encode entirely.
  Considered post-v2.0.

## References

- `src-tauri/src/services/reencode.rs` (the throttle loop)
- `src-tauri/src/infra/process/priority.rs` (OS-specific helpers)
- `src-tauri/migrations/0015_quality_settings.sql` (threshold columns)
- ADR-0010 — bandwidth throttle (the precedent for "per-worker
  background friendliness")
