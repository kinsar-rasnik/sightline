//! Bandwidth-throttle primitives.
//!
//! The mission brief calls for a "global token bucket across all
//! parallel downloads". We actually run two things side-by-side:
//!
//! 1. A **configuration authority** (`GlobalRate`) — holds the user's
//!    bytes-per-second setting and the current download concurrency,
//!    and returns a per-worker share. This is what gets handed to
//!    yt-dlp as `--limit-rate`. It is recomputed whenever concurrency
//!    or the setting changes; active downloads pick up the new value
//!    on their next progress tick without aborting.
//!
//! 2. A genuine in-process `TokenBucket` for anything we *can* meter
//!    ourselves (thumbnail downloads, metadata fetches, future
//!    features). Standard bucket with `refill_rate * 2` burst.
//!
//! See ADR-0010 for the rationale behind the split — yt-dlp's
//! `--limit-rate` is per-connection, not per-process, so a single
//! true global bucket is not actually enforceable against it.

use std::sync::Mutex;
use std::time::Duration;

/// A user-supplied setting: `Some(bytes/sec)` caps bandwidth, `None`
/// means unlimited.
pub type RateCapBps = Option<u64>;

/// Minimum per-worker share when a cap is set. A yt-dlp workers told
/// to "limit to 32KB/s" effectively cannot pick up its chunks fast
/// enough; we floor at 512 KB/s so the download finishes in finite
/// time. The `GlobalRate::per_worker_bps` documentation flags this.
pub const MIN_WORKER_BPS: u64 = 512 * 1024;

/// Holds the current cap + the number of active workers and returns
/// the per-worker yt-dlp `--limit-rate` value. Thread-safe via a
/// tokio-friendly `Mutex` — updates are rare (user drags a slider).
#[derive(Debug, Default)]
pub struct GlobalRate {
    inner: Mutex<Inner>,
}

#[derive(Debug, Default, Clone, Copy)]
struct Inner {
    cap_bps: RateCapBps,
    active_workers: usize,
}

impl GlobalRate {
    /// Build a new authority. Starts with no cap (unlimited) and zero
    /// active workers; callers call `set_cap` / `set_active_workers`
    /// once the queue service is up.
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(Inner::default()),
        }
    }

    pub fn set_cap(&self, cap: RateCapBps) {
        // std::sync::Mutex::lock cannot fail unless another thread
        // panicked while holding the guard; in that case, propagating
        // the panic is correct.
        #[allow(clippy::unwrap_used)]
        let mut i = self.inner.lock().unwrap();
        i.cap_bps = cap;
    }

    pub fn set_active_workers(&self, n: usize) {
        #[allow(clippy::unwrap_used)]
        let mut i = self.inner.lock().unwrap();
        i.active_workers = n;
    }

    pub fn active_workers(&self) -> usize {
        #[allow(clippy::unwrap_used)]
        self.inner.lock().unwrap().active_workers
    }

    pub fn cap(&self) -> RateCapBps {
        #[allow(clippy::unwrap_used)]
        self.inner.lock().unwrap().cap_bps
    }

    /// Per-worker `--limit-rate`. Returns `None` when the global cap
    /// is `None` (unlimited). When a cap is set, splits it evenly
    /// across active workers with a `MIN_WORKER_BPS` floor. If the
    /// worker-count is 0 the raw cap is returned — typical when a
    /// worker is about to start.
    pub fn per_worker_bps(&self) -> RateCapBps {
        #[allow(clippy::unwrap_used)]
        let i = *self.inner.lock().unwrap();
        let cap = i.cap_bps?;
        let share = if i.active_workers == 0 {
            cap
        } else {
            cap / (i.active_workers as u64)
        };
        Some(share.max(MIN_WORKER_BPS))
    }
}

/// Textbook token bucket (milliseconds granularity). Not used for
/// yt-dlp itself (see module docs); here for future in-process
/// metering.
#[derive(Debug)]
pub struct TokenBucket {
    state: Mutex<BucketState>,
    refill_per_sec: u64,
    capacity: u64,
}

#[derive(Debug, Clone, Copy)]
struct BucketState {
    tokens: f64,
    last_refill_ms: u64,
}

impl TokenBucket {
    /// `refill_per_sec` is the long-term budget (bytes / tokens per
    /// second). `burst_multiplier` sizes the bucket at
    /// `refill_per_sec * burst_multiplier`. A common reasonable
    /// default is 2.
    pub fn new(refill_per_sec: u64, burst_multiplier: u64) -> Self {
        let capacity = refill_per_sec.saturating_mul(burst_multiplier.max(1));
        Self {
            state: Mutex::new(BucketState {
                tokens: capacity as f64,
                last_refill_ms: 0,
            }),
            refill_per_sec,
            capacity,
        }
    }

    pub fn capacity(&self) -> u64 {
        self.capacity
    }

    /// Try to consume `amount` tokens at the caller-supplied wall
    /// clock in milliseconds. Returns `Ok(())` on success or
    /// `Err(Wait)` reporting how long to wait before retrying.
    ///
    /// Milliseconds-since-start time source is provided by the caller
    /// so tests can drive the bucket deterministically. Production
    /// callers use a `std::time::Instant` mapped to `Duration::as_millis`.
    pub fn try_consume(&self, amount: u64, now_ms: u64) -> Result<(), Wait> {
        #[allow(clippy::unwrap_used)]
        let mut state = self.state.lock().unwrap();
        self.refill(&mut state, now_ms);
        if state.tokens >= amount as f64 {
            state.tokens -= amount as f64;
            Ok(())
        } else {
            // Time in seconds to accrue the shortfall.
            let missing = amount as f64 - state.tokens;
            let seconds = missing / self.refill_per_sec.max(1) as f64;
            Err(Wait(Duration::from_secs_f64(seconds.max(0.001))))
        }
    }

    /// Read-only accessor — used by tests and observability.
    pub fn tokens(&self, now_ms: u64) -> f64 {
        #[allow(clippy::unwrap_used)]
        let mut state = self.state.lock().unwrap();
        self.refill(&mut state, now_ms);
        state.tokens
    }

    fn refill(&self, state: &mut BucketState, now_ms: u64) {
        let last = state.last_refill_ms;
        if now_ms <= last {
            state.last_refill_ms = now_ms;
            return;
        }
        let elapsed_ms = now_ms - last;
        let add = (self.refill_per_sec as f64) * (elapsed_ms as f64) / 1000.0;
        state.tokens = (state.tokens + add).min(self.capacity as f64);
        state.last_refill_ms = now_ms;
    }
}

/// Return value of `try_consume`: how long the caller should wait
/// before retrying.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Wait(pub Duration);

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn unlimited_when_no_cap_set() {
        let g = GlobalRate::new();
        g.set_active_workers(3);
        assert_eq!(g.per_worker_bps(), None);
    }

    #[test]
    fn cap_divides_across_active_workers() {
        let g = GlobalRate::new();
        g.set_cap(Some(10 * 1024 * 1024)); // 10 MB/s
        g.set_active_workers(2);
        assert_eq!(g.per_worker_bps(), Some(5 * 1024 * 1024));
    }

    #[test]
    fn min_floor_enforced_when_share_tiny() {
        let g = GlobalRate::new();
        // 100 KB/s total, 10 workers → 10 KB/s each < floor
        g.set_cap(Some(100 * 1024));
        g.set_active_workers(10);
        assert_eq!(g.per_worker_bps(), Some(MIN_WORKER_BPS));
    }

    #[test]
    fn zero_workers_returns_raw_cap() {
        let g = GlobalRate::new();
        g.set_cap(Some(4 * 1024 * 1024));
        assert_eq!(g.per_worker_bps(), Some(4 * 1024 * 1024));
    }

    #[test]
    fn updating_concurrency_updates_share() {
        let g = GlobalRate::new();
        g.set_cap(Some(6 * 1024 * 1024));
        g.set_active_workers(3);
        assert_eq!(g.per_worker_bps(), Some(2 * 1024 * 1024));
        g.set_active_workers(1);
        assert_eq!(g.per_worker_bps(), Some(6 * 1024 * 1024));
    }

    #[test]
    fn bucket_starts_full() {
        let b = TokenBucket::new(1000, 2);
        assert_eq!(b.capacity(), 2000);
        assert!((b.tokens(0) - 2000.0).abs() < 1e-6);
    }

    #[test]
    fn bucket_consumes_tokens_when_available() {
        let b = TokenBucket::new(1000, 2);
        assert!(b.try_consume(500, 0).is_ok());
        // 2000 - 500 = 1500 tokens remaining.
        assert!((b.tokens(0) - 1500.0).abs() < 1e-6);
    }

    #[test]
    fn bucket_rejects_over_capacity_and_reports_wait() {
        let b = TokenBucket::new(1000, 2);
        b.try_consume(2000, 0).unwrap();
        let wait = b.try_consume(500, 0).unwrap_err();
        // 500 missing tokens / 1000 per second = 0.5 seconds.
        assert!(wait.0.as_millis() >= 500);
    }

    #[test]
    fn bucket_refills_over_time() {
        let b = TokenBucket::new(1000, 2);
        // Drain.
        b.try_consume(2000, 0).unwrap();
        // 1 second later, 1000 tokens have re-accrued.
        assert!((b.tokens(1_000) - 1000.0).abs() < 1e-6);
        assert!(b.try_consume(800, 1_000).is_ok());
        assert!((b.tokens(1_000) - 200.0).abs() < 1e-6);
    }

    #[test]
    fn bucket_clamps_to_capacity() {
        let b = TokenBucket::new(1000, 2);
        // Already full; spending 500 then letting 10s pass should cap.
        b.try_consume(500, 0).unwrap();
        let tokens = b.tokens(10_000);
        assert!((tokens - 2000.0).abs() < 1.0);
    }

    #[test]
    fn bucket_never_goes_negative_under_concurrent_consume() {
        use std::sync::Arc;
        use std::thread;
        let b = Arc::new(TokenBucket::new(10_000, 1));
        let mut joins = Vec::new();
        for _ in 0..8 {
            let b = b.clone();
            joins.push(thread::spawn(move || {
                for _ in 0..1000 {
                    let _ = b.try_consume(1, 0);
                }
            }));
        }
        for j in joins {
            j.join().unwrap();
        }
        // 8 * 1000 = 8000 requests, each 1 token. Capacity is 10_000,
        // so all succeed and tokens can't go below 2_000.
        assert!(b.tokens(0) >= 1999.0);
    }
}
