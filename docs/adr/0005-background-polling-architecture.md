# ADR-0005 — Background polling architecture

- **Status.** Accepted
- **Date.** 2026-04-24
- **Phase.** 1
- **Deciders.** CTO, Senior Engineer

## Context

Sightline's core value is discovering new VODs for followed streamers without the user having to do anything. That requires polling the Twitch Helix API on a cadence, even when no window is visible. The implementation must satisfy:

- **Runs with the window closed.** Menu-bar / tray mode on all three OSes.
- **Survives sleep / wake.** A laptop that sleeps for six hours should resume cleanly without hammering the API.
- **Respects rate limits.** Twitch Helix has a global 800 points/minute bucket; we use 1 point per Videos call.
- **Observable.** The user can see the last poll time, the next scheduled time, and the most recent error per streamer.
- **Testable.** The scheduler must be injectable so tests do not sleep in real time.

Options considered:

1. **Spawn a hidden webview that runs the poll loop in JS.** Works but ties lifecycle to a window handle and obscures CPU usage.
2. **Run a separate OS service (launchd / systemd / Task Scheduler).** Overkill for v1.
3. **Run the poll loop as a tokio task owned by the main Rust process, independent of any window.**

## Decision

We implement a **Tokio-based background Poller** owned by the main Rust process.

Shape:

```rust
pub struct PollerHandle {
    tx: mpsc::Sender<PollerCommand>,
}

pub enum PollerCommand {
    PollNow,
    Reconfigure(PollerConfig),
    Shutdown(oneshot::Sender<()>),
}

pub struct PollerService { /* owns clients, config, clock */ }

impl PollerService {
    pub async fn run(mut self, mut rx: mpsc::Receiver<PollerCommand>) { /* ... */ }
}
```

Rules:

- `PollerService::run` selects on a `tokio::time::interval`, the command channel, and a shutdown signal. Single `loop { select! { ... } }`.
- The **clock is an injected trait** (`trait Clock { fn now(&self) -> Instant; fn sleep(&self, d: Duration) -> ...; }`). Tests use a fake clock; production uses a tokio-backed clock.
- One task handles one streamer per tick (round-robin), with concurrency configurable (default 4).
- Rate-limit awareness: the HelixClient layer returns remaining budget; the poller throttles accordingly.
- On sleep/wake: the interval is paced by wall clock, not monotonic time. If the process wakes with more than one missed tick, we coalesce into a single catch-up poll.
- The tray / menu-bar lifecycle owns the process. Closing the main window does not terminate the runtime.

## Consequences

Positive:

- **Testable.** Clock injection + message-based API means tests run in milliseconds.
- **Unified process.** One runtime, one log, one place to look when something misbehaves.
- **Graceful shutdown.** The `Shutdown` variant carries a `oneshot` so the UI can wait for drain.

Negative:

- **OS-specific tray handling.** WebKitGTK and platform differences require guards (see `docs/tech-spec.md §9`).
- **User education.** Close-to-tray can surprise users; we surface a first-run dialog and a setting to change it.
- **No multi-process isolation.** A crash in the poll path risks the whole app. We accept this for v1 and mitigate with a panic hook that restarts the poller task without tearing down the runtime.

## Mitigations

- A supervisor wraps `PollerService::run`: on panic (caught via `tokio::task::JoinHandle::await`), the supervisor logs and restarts after a backoff.
- Integration tests exercise panic recovery by injecting a faulting clock.
- Sentry / local diagnostic bundle captures the last 200 log lines on crash for post-mortem.

## Alternatives considered and rejected

- **Hidden renderer.** Rejected — ties work to window lifecycle, obscures CPU usage, and doubles memory.
- **Separate OS service.** Rejected — installation, permission, and packaging burden.
- **Cron-driven external process.** Rejected — cross-platform scheduling is a bespoke project on its own.

## Supersedes / superseded by

- None.
