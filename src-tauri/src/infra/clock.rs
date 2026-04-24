//! Clock abstraction — lets tests substitute a fake clock for the system
//! time. Services that schedule work take `&dyn Clock`; domain code never
//! reads wall-clock time directly.

use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

/// Read unix seconds UTC. Infallible in production — `SystemTime::now`
/// can only fail if the system clock is before 1970, which we treat as
/// an impossible configuration and map to 0.
pub trait Clock: Send + Sync + std::fmt::Debug {
    fn unix_seconds(&self) -> i64;
}

/// Production clock. A unit struct that delegates to `SystemTime::now`.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn unix_seconds(&self) -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0)
    }
}

/// Deterministic clock for tests. Internally an `AtomicI64` seconds value
/// so it can be shared across tasks without a mutex.
#[derive(Debug, Clone)]
pub struct FixedClock {
    inner: Arc<AtomicI64>,
}

impl FixedClock {
    pub fn at(seconds: i64) -> Self {
        Self {
            inner: Arc::new(AtomicI64::new(seconds)),
        }
    }

    pub fn set(&self, seconds: i64) {
        self.inner.store(seconds, Ordering::SeqCst);
    }

    pub fn advance(&self, seconds: i64) {
        self.inner.fetch_add(seconds, Ordering::SeqCst);
    }
}

impl Clock for FixedClock {
    fn unix_seconds(&self) -> i64 {
        self.inner.load(Ordering::SeqCst)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn system_clock_is_positive() {
        assert!(SystemClock.unix_seconds() > 1_700_000_000);
    }

    #[test]
    fn fixed_clock_set_and_advance() {
        let c = FixedClock::at(1_000);
        assert_eq!(c.unix_seconds(), 1_000);
        c.advance(60);
        assert_eq!(c.unix_seconds(), 1_060);
        c.set(0);
        assert_eq!(c.unix_seconds(), 0);
    }
}
