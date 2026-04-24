//! Interval merger for `watch_progress.total_watch_seconds` (Phase 5).
//!
//! Motivation: `total_watch_seconds` must reflect *unique* playback
//! time, so a user who scrubs back and watches a segment twice
//! doesn't double-count it. We receive a stream of `[start, end)`
//! half-open intervals from the player's debounced `timeupdate` tick
//! and collapse overlapping/adjacent pairs into a single covered
//! range.
//!
//! Every operation is pure. The service layer holds an
//! `IntervalSet` per playback session, calls `observe(..)` on every
//! update, and reads `total_seconds()` before persisting.

use std::cmp::Ordering;

/// Half-open interval `[start, end)` over seconds. `start <= end` is
/// enforced at construction — attempts to build a backwards interval
/// yield a zero-length one.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Interval {
    pub start: f64,
    pub end: f64,
}

impl Interval {
    pub fn new(start: f64, end: f64) -> Self {
        if end < start {
            Self { start, end: start }
        } else {
            Self { start, end }
        }
    }

    pub fn len(self) -> f64 {
        (self.end - self.start).max(0.0)
    }

    pub fn is_empty(self) -> bool {
        self.len() == 0.0
    }

    /// Two intervals overlap if they share any point, counting the
    /// half-open boundary as contiguous.
    pub fn overlaps_or_adjacent(self, other: Self) -> bool {
        self.start <= other.end && other.start <= self.end
    }

    /// Merge two overlapping/adjacent intervals. Undefined behaviour
    /// (callers shouldn't pass disjoint intervals here); we return a
    /// conservative union.
    pub fn merge(self, other: Self) -> Self {
        Self {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }
}

/// Running set of merged intervals. Stored sorted by `start` so
/// `observe` is amortised O(log n) + O(k) where k is the number of
/// newly-merged intervals (which is typically 0 or 1 for real-time
/// playback).
#[derive(Debug, Default, Clone)]
pub struct IntervalSet {
    merged: Vec<Interval>,
}

impl IntervalSet {
    pub fn new() -> Self {
        Self::default()
    }

    /// Total number of seconds covered by the merged set. O(n) —
    /// call lazily (e.g. once per DB write), not on every tick.
    pub fn total_seconds(&self) -> f64 {
        self.merged.iter().map(|i| i.len()).sum()
    }

    pub fn len(&self) -> usize {
        self.merged.len()
    }

    pub fn is_empty(&self) -> bool {
        self.merged.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Interval> {
        self.merged.iter()
    }

    /// Absorb a new observation. Zero-length intervals are a no-op.
    /// Overlapping / adjacent neighbours are coalesced; the result
    /// stays sorted by `start`.
    pub fn observe(&mut self, mut next: Interval) {
        if next.is_empty() {
            return;
        }
        // Binary search for the insertion point by `start`. We then
        // walk forward merging anything that overlaps or touches the
        // incoming interval.
        let idx = self.merged.partition_point(|i| i.start <= next.start);
        // Look behind: if the previous interval overlaps or is
        // adjacent, merge backwards first.
        if idx > 0 {
            let prev = self.merged[idx - 1];
            if prev.overlaps_or_adjacent(next) {
                next = prev.merge(next);
                self.merged.remove(idx - 1);
            }
        }
        // Now walk forward absorbing overlapping neighbours.
        let insert_at = self.merged.partition_point(|i| i.start <= next.start);
        while insert_at < self.merged.len() && self.merged[insert_at].overlaps_or_adjacent(next) {
            next = next.merge(self.merged[insert_at]);
            self.merged.remove(insert_at);
        }
        self.merged.insert(insert_at, next);
    }

    /// Rebuild the set from a vector of raw intervals — useful when
    /// restoring from DB on session resume.
    pub fn from_intervals(mut raw: Vec<Interval>) -> Self {
        raw.retain(|i| !i.is_empty());
        raw.sort_by(|a, b| {
            a.start
                .partial_cmp(&b.start)
                .unwrap_or(Ordering::Equal)
                .then_with(|| a.end.partial_cmp(&b.end).unwrap_or(Ordering::Equal))
        });
        let mut set = Self::new();
        for iv in raw {
            set.observe(iv);
        }
        set
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::float_cmp)]
mod tests {
    use super::*;

    fn iv(a: f64, b: f64) -> Interval {
        Interval::new(a, b)
    }

    #[test]
    fn new_interval_clamps_backwards_input_to_zero_length() {
        let v = iv(10.0, 5.0);
        assert!(v.is_empty());
    }

    #[test]
    fn empty_set_has_zero_total() {
        assert_eq!(IntervalSet::new().total_seconds(), 0.0);
    }

    #[test]
    fn single_observation_preserves_length() {
        let mut s = IntervalSet::new();
        s.observe(iv(10.0, 15.0));
        assert_eq!(s.total_seconds(), 5.0);
    }

    #[test]
    fn disjoint_observations_stack() {
        let mut s = IntervalSet::new();
        s.observe(iv(0.0, 10.0));
        s.observe(iv(20.0, 25.0));
        assert_eq!(s.total_seconds(), 15.0);
        assert_eq!(s.len(), 2);
    }

    #[test]
    fn overlapping_observations_merge() {
        let mut s = IntervalSet::new();
        s.observe(iv(0.0, 10.0));
        s.observe(iv(5.0, 15.0));
        assert_eq!(s.total_seconds(), 15.0);
        assert_eq!(s.len(), 1);
    }

    #[test]
    fn adjacent_observations_merge() {
        let mut s = IntervalSet::new();
        s.observe(iv(0.0, 10.0));
        s.observe(iv(10.0, 20.0));
        assert_eq!(s.total_seconds(), 20.0);
        assert_eq!(s.len(), 1);
    }

    #[test]
    fn nested_observations_absorb_into_outer() {
        let mut s = IntervalSet::new();
        s.observe(iv(0.0, 100.0));
        s.observe(iv(20.0, 30.0));
        s.observe(iv(50.0, 55.0));
        assert_eq!(s.total_seconds(), 100.0);
        assert_eq!(s.len(), 1);
    }

    #[test]
    fn scrubbing_back_does_not_double_count() {
        // Realistic scrub-back scenario: user watches 0–60, scrubs
        // back to 40, plays to 80. Total covered = 80 s (not 100).
        let mut s = IntervalSet::new();
        s.observe(iv(0.0, 60.0));
        s.observe(iv(40.0, 80.0));
        assert_eq!(s.total_seconds(), 80.0);
        assert_eq!(s.len(), 1);
    }

    #[test]
    fn many_tiny_intervals_coalesce_into_one() {
        // The player's debounced writer sends a 4 Hz stream — after
        // a minute of playback we should still have one merged range.
        let mut s = IntervalSet::new();
        let mut t = 0.0;
        while t < 60.0 {
            s.observe(iv(t, t + 0.25));
            t += 0.25;
        }
        assert_eq!(s.len(), 1);
        assert_eq!(s.total_seconds(), 60.0);
    }

    #[test]
    fn from_intervals_sorts_and_merges() {
        // Feed pre-sorted-wrong data and confirm the rebuild path
        // normalises it. This is what we'd do when restoring a
        // session from the DB — we don't trust the saved order.
        let set = IntervalSet::from_intervals(vec![
            iv(50.0, 60.0),
            iv(0.0, 10.0),
            iv(5.0, 20.0),
            iv(55.0, 70.0),
        ]);
        assert_eq!(set.total_seconds(), 40.0);
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn empty_observation_is_noop() {
        let mut s = IntervalSet::new();
        s.observe(iv(0.0, 10.0));
        s.observe(iv(5.0, 5.0));
        assert_eq!(s.total_seconds(), 10.0);
        assert_eq!(s.len(), 1);
    }
}
