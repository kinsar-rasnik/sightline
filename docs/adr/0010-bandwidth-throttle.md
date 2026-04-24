# ADR-0010: Bandwidth throttle — per-worker `--limit-rate`, not a true global bucket

- **Status.** Accepted (2026-04-24).
- **Phase.** 3.
- **Deciders.** Senior Engineer (Claude), reviewing CTO.

## Context

Phase 3's download queue runs `max_concurrent_downloads` yt-dlp
subprocesses in parallel (default 2, max 5). The user can set a
global bandwidth cap in bytes per second, or leave it unlimited.

yt-dlp exposes `--limit-rate`, but this argument is **per-connection**
(arguably per-process, never fully documented). It does not know
about sibling processes on the same machine; two concurrent yt-dlp
workers each passed `--limit-rate 5M` will attempt to consume 10 MB/s
combined, not 5.

The obvious alternative — intercept the bytes at the syscall level
and meter them ourselves — is not viable. yt-dlp opens its own
sockets, HTTPS-decrypts inline, and pipes output to a file we don't
control. Writing our own HTTP layer would re-implement much of
yt-dlp.

## Decision

The queue ships a **fair-share authority**, not a token bucket that
yt-dlp respects.

- `infra::throttle::GlobalRate` holds the user's cap (`Option<u64>`
  bytes/sec) and the current number of active workers.
- Before each yt-dlp invocation, the worker asks
  `GlobalRate::per_worker_bps()` and passes the returned value as
  `--limit-rate`.
- When concurrency changes (worker starts / finishes, or user changes
  the cap) the authority recomputes. The in-flight worker picks the
  new value up on its **next** launch; already-running downloads keep
  their original limit until they finish or fail.
- A `MIN_WORKER_BPS = 512 KB/s` floor prevents a "5-worker pool × a
  1 MB/s cap" from giving each worker 200 KB/s — yt-dlp effectively
  stalls below that.

An in-process `TokenBucket` exists alongside the authority and is
wired for any future metered action we can actually intercept
(thumbnail HTTP fetches, VOD info probes). It is a conventional
bucket with refill rate + burst capacity.

## Consequences

### Positive

- Simple implementation (~60 lines of Rust + ~80 lines of tests).
- The user's cap is respected in aggregate **at steady state**: as
  downloads naturally churn, new workers take the updated share.
- No byte-level interception of yt-dlp, so no risk of breaking
  yt-dlp's transparent retry / segment-resume logic.

### Negative / accepted risks

- **Short-term overshoot.** For the seconds it takes an in-flight
  download to finish after the cap changes, total throughput can
  exceed the cap. In practice this is bounded by the number of
  active workers × their old per-worker rate, so ≤ 2× the new cap
  in a pathological case.
- **No fairness within a single worker.** yt-dlp opens multiple
  concurrent HTTPS connections for the video/audio format split. Our
  `--limit-rate` applies per-connection; a 1080p60 split could use
  2× the per-worker share. We accept this — it's bounded by the
  format's typical two-connection max, and the user-visible signal
  (MB/s they see) is still close.
- **Different from "real" QoS.** Users who want a hard cap at the
  network level should still use their router or OS-level shaping.
  We document this in the Settings help text.

### Neutral

- The TokenBucket remains useful for future metering — it costs
  nothing to ship alongside.

## Alternatives considered

1. **Patch yt-dlp.** Too invasive; we bundle the upstream binary and
   rely on the user's ability to self-update.
2. **Intercept network at OS level (NEKit, tc filter).** Requires
   root/Admin and is platform-specific; hostile to a cross-platform
   desktop app.
3. **Proxy yt-dlp through a local rate-limiting HTTP proxy.** Doable
   but implies shipping a second sidecar (and a TLS MITM, which
   doesn't work against HLS segments without cert-pinning gymnastics).
4. **Treat `max_concurrent_downloads = 1` as a constraint when a cap
   is set.** Simplifies the math but defeats the primary purpose of
   having a queue at all.

## References

- `src-tauri/src/infra/throttle.rs` — implementation + tests.
- `src-tauri/src/services/downloads.rs` — integration point.
- `docs/implementation-plan.md §Phase 3` — spec.
