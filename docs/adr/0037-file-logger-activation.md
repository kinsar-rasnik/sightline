# ADR-0037 — File-logger activation

- **Status.** Accepted
- **Date.** 2026-04-26 (v2.0.3 hotfix)
- **Related.** None — this ADR backfills a Tech-Spec §8 promise
  that has been outstanding since v1.0.

## Context

Tech-Spec §8 ("Observability") has documented since v1.0:

> Application logs are written to `$LOG_DIR/sightline.log`, daily
> rotation, 20 MB cap per file, retain 7.

Reality up to v2.0.2:

```rust
fn init_tracing() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));
    let subscriber = fmt().with_env_filter(filter).with_target(false).finish();
    if tracing::subscriber::set_global_default(subscriber).is_err() {
        warn!("tracing subscriber already initialized");
    }
}
```

stderr only.  No file sink.  No `~/Library/Logs/dev.sightline.app/`
directory was ever created.  When the v2.0.2 download-engine bug hit
the CEO's install, the only way to triage was to manually re-run
the binary with `RUST_LOG=debug pnpm tauri dev > /tmp/sightline.log
2>&1`, which (a) requires a developer-grade workflow on the user's
machine, (b) loses the actual production runtime context, and (c)
doesn't help any user who isn't running from source.

The hotfix has to land the Tech-Spec promise: rolling daily file
log in the OS-native log directory, no developer-grade ceremony
required.

## Decision

Add `tracing-appender = "0.2"` and switch
`src-tauri/src/lib.rs::init_tracing` from a single `fmt`
subscriber to a `Registry` composing two layers:

- **stderr layer** — keeps the existing developer-facing output.
  Default level `warn` (so production stderr stays quiet).
- **file layer** — daily-rotating file under
  `default_log_dir() / "sightline.<date>.log"`.  Default level
  `info`.  Implementation:

```rust
tracing_appender::rolling::Builder::new()
    .rotation(Rotation::DAILY)
    .filename_prefix("sightline")
    .filename_suffix("log")
    .max_log_files(7)
    .build(&log_dir)
```

Both layers respect `RUST_LOG` when set; the override applies
identically to both sinks (no per-sink `RUST_LOG_FILE` /
`RUST_LOG_STDERR` vocabulary in this version).

The appender is wrapped with `tracing_appender::non_blocking(...)`
so writes happen on a dedicated thread; otherwise every log call
from a Tokio worker would block on file I/O.  The returned
`WorkerGuard` is stashed in a `once_cell::sync::OnceCell` named
`LOG_GUARD` that lives as long as the binary — dropping the guard
would silently shut down the writer thread.

### Log directory

The directory is computed manually because `init_tracing()` runs
before the Tauri runtime exists, and `tauri::path::PathResolver`
needs an `AppHandle`.  The algorithm matches Tauri 2's
`app_log_dir()` for bundle id `dev.sightline.app` so the file we
write is the same file the runtime would point at:

| OS | Path |
| --- | --- |
| macOS | `$HOME/Library/Logs/dev.sightline.app/sightline.<date>.log` |
| Windows | `%LOCALAPPDATA%\dev.sightline.app\logs\sightline.<date>.log` |
| Linux | `$XDG_CACHE_HOME/dev.sightline.app/logs/sightline.<date>.log` (or `$HOME/.cache/dev.sightline.app/logs/...` if XDG is unset) |

If the directory cannot be created (read-only home,
unset env vars on a stripped-down container), the file sink
silently degrades to stderr-only — startup is *never* aborted by
log-init failure.  An `eprintln!("[sightline] file logger
disabled — proceeding with stderr only")` surfaces the degradation
to a developer running with a console.

### 20-MB-per-file cap → deferred

Tech-Spec §8's "20 MB cap per file" is **not implemented in
v2.0.3**.  `tracing-appender`'s `RollingFileAppender` rotates by
time (Daily / Hourly / Minutely / Never) and supports
`max_log_files(N)` for retention count, but no per-file size cap.

For v2.0.3 we accept daily rotation + 7-day retention as the
working envelope.  Typical observed log volume in the dev workflow
is a few MB / day; the worst-case unbounded growth would require
a download-engine bug producing many MB/min of WARN+ output, which
is exactly the kind of problem the file logger is here to capture.
A pragmatic per-file cap would be a future enhancement that
either (a) switches to an hourly rotation when daily files exceed
20 MB, or (b) writes a custom `MakeWriter` that wraps the appender
with a size check.  Tracked as v2.1 backlog.

### Smoke test

`src-tauri/src/lib.rs::tests::rolling_file_appender_writes_to_disk`
constructs the same `Builder` chain with `tempfile::tempdir()` as
the destination, writes a marker line, and asserts the marker is
present in a file in the temp dir.  Catches a regression where an
upstream rename (e.g., `filename_suffix` becoming required)
silently disables the file sink.

`default_log_dir_resolves_on_supported_platforms` asserts the
function returns `Some` on the platforms the test runs on (macOS /
Linux for CI; Windows path is constructed from `LOCALAPPDATA`
which CI sets).  Catches a future cfg-arm regression that lands
in the `None` fallthrough.

## Alternatives considered

### A. `tauri-plugin-log`

The official Tauri plugin handles file logging out of the box,
including platform paths and rotation.  Rejected for v2.0.3
because (a) adding a plugin pulls in a permission/capability
surface to audit, (b) the plugin's API is centred on
`AppHandle`-based init and would require restructuring our
two-stage startup, and (c) `tracing-appender` is already on the
shortest path to "file in OS log dir".  Tracked as v2.1 candidate
if the plugin's rotation policy proves more flexible than ours.

### B. Initialise tracing inside `setup` (post-AppHandle)

Move `init_tracing` from the top of `pub fn run()` into the Tauri
`setup` callback so we can call `app.path().app_log_dir()`
directly.  Rejected because `tracing::subscriber::set_global_default`
can be called only once; pre-`setup` `warn!` calls (e.g., the
`tauri-specta export skipped` line in debug builds) would either
go unlogged or require a complex `Reload` layer.  The manual
log-dir computation is a one-time write and matches Tauri's
algorithm exactly, so the cost of duplicating it is essentially
zero.

### C. Synchronous (blocking) file writes

Drop `non_blocking` and let the `RollingFileAppender` write
inline.  Rejected as a foot-gun — every tracing event from a Tokio
worker would stall on file I/O (slow disks, network mounts, log
rotation latency).  The `WorkerGuard` ceremony to keep the
non-blocking thread alive is small in exchange.

## Consequences

**Positive.**
- A user filing a bug can attach the log file from the documented
  OS path without running the app from source or under a
  developer environment.
- The file is the same path the future
  `app_log_dir()`-based code would resolve, so `tauri::Manager`
  can be wired in later without users having to look in two
  places.
- Per-sink defaults (stderr=warn, file=info) keep the developer
  console quiet during ordinary operation while still capturing
  useful state for support.

**Costs accepted.**
- One additional dependency (`tracing-appender`).  Tokio
  ecosystem standard, low risk.
- 7-day rotation without a size cap.  Worst-case footprint for a
  pathological log-explosion is bounded only by 7 days × disk I/O
  rate.  Trip-wire: if any user reports the log dir consuming
  hundreds of MB, switch to hourly rotation in a v2.0.4 hotfix.
- Pre-Tauri-runtime log lines (a handful of `warn!` calls before
  the `setup` callback) are bound by the same registry, so they
  appear in the file too.

**Risks.**
- A canonical `RUST_LOG=debug` invocation now floods both stderr
  AND the file.  Users running with debug logging should expect
  the file size to balloon for that session.  Acceptable —
  documented in [README.md](../../README.md) §"Logs".
- The `default_log_dir` algorithm reproduces Tauri's
  internal logic; if Tauri changes its algorithm in a future
  version bump, our path drifts.  Tracked as v2.1 candidate to
  swap in `tauri-plugin-log` or a `Reload`-based two-stage init.

## Follow-ups

- **v2.1 candidate:** per-file 20 MB cap (custom `MakeWriter`
  wrapper, or hourly rotation when daily files cross the cap).
- **v2.1 candidate:** `tauri-plugin-log` adoption (alternative A)
  if the plugin's rotation policy lines up.
- **v2.1 candidate:** Settings UI button "Open log folder" that
  shells out to the OS file manager pointed at
  `default_log_dir()`.

## References

- `src-tauri/src/lib.rs::init_tracing`
- `src-tauri/src/lib.rs::init_rolling_file_appender`
- `src-tauri/src/lib.rs::default_log_dir`
- `src-tauri/src/lib.rs::tests::rolling_file_appender_writes_to_disk`
- `src-tauri/src/lib.rs::tests::default_log_dir_resolves_on_supported_platforms`
- `README.md` §"Logs"
- Tech-Spec §8 — "Observability"
