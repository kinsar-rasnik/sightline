//! Domain primitives for the multi-streamer timeline (Phase 4).
//!
//! Pure, I/O-free. `Interval`s describe a single stream's wall-clock
//! lifetime; helpers compute overlaps between intervals, bucket them by
//! day for day/week/month views, and score co-streams (for the library
//! detail drawer).
//!
//! The services layer (`services::timeline_indexer`) materialises these
//! into the `stream_intervals` table from Phase 4 migration 0006 and the
//! `/timeline` UI route consumes them via the IPC layer.
//!
//! All functions operate on *borrowed* slices so the indexer can reuse
//! the same allocation across queries without cloning. Unit + property
//! tests live at the bottom of this file.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use specta::Type;

/// A single stream's wall-clock lifetime. `start_at` / `end_at` are UTC
/// unix seconds; the invariant `start_at <= end_at` is enforced at the
/// DB layer (CHECK) and by `Interval::new`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Interval {
    pub vod_id: String,
    pub streamer_id: String,
    pub start_at: i64,
    pub end_at: i64,
}

impl Interval {
    /// Construct an interval, coercing `end_at < start_at` into
    /// `end_at = start_at` (a zero-length point interval). We never
    /// return `None` here because the DB migration's CHECK already
    /// guarantees the invariant; this normalisation is defensive for
    /// in-memory fixtures.
    pub fn new(
        vod_id: impl Into<String>,
        streamer_id: impl Into<String>,
        start_at: i64,
        end_at: i64,
    ) -> Self {
        let end_at = end_at.max(start_at);
        Self {
            vod_id: vod_id.into(),
            streamer_id: streamer_id.into(),
            start_at,
            end_at,
        }
    }

    /// Duration in seconds. Always non-negative given `new`'s invariant.
    pub fn duration_seconds(&self) -> i64 {
        self.end_at - self.start_at
    }
}

/// Return the intersection of `a` and `b` if they overlap, `None`
/// otherwise. Two intervals are considered to overlap when they share
/// at least one second in common; touching endpoints (`a.end_at ==
/// b.start_at`) do not count because a zero-length overlap is not
/// interesting to the UI.
///
/// The returned interval copies `a`'s vod_id/streamer_id for debugging
/// purposes — callers should only read `start_at` / `end_at` on the
/// result.
pub fn overlapping(a: &Interval, b: &Interval) -> Option<Interval> {
    let lo = a.start_at.max(b.start_at);
    let hi = a.end_at.min(b.end_at);
    if lo < hi {
        Some(Interval::new(&a.vod_id, &a.streamer_id, lo, hi))
    } else {
        None
    }
}

/// Group intervals by UTC calendar day. Keys are day indices (unix
/// seconds / 86400), which preserves ordering without the serde cost
/// of `chrono::NaiveDate`. Frontend pairs the day index with a `Date`
/// constructed in the user's locale.
///
/// An interval that spans midnight appears under every day it touches,
/// bucketed by the start of the day (not the slice). The slice itself
/// is not clipped — callers render the full bar, which is the correct
/// visual behaviour for a day lane.
pub fn bucket_by_day<'a>(intervals: &'a [Interval]) -> BTreeMap<i64, Vec<&'a Interval>> {
    const DAY: i64 = 86_400;
    let mut out: BTreeMap<i64, Vec<&'a Interval>> = BTreeMap::new();
    for iv in intervals {
        let first_day = iv.start_at.div_euclid(DAY);
        // Empty interval at midnight: place it in its start day only.
        let last_day = if iv.end_at == iv.start_at {
            first_day
        } else {
            // An interval that ends exactly at midnight belongs to the
            // previous day, not the one that starts at that instant.
            (iv.end_at - 1).div_euclid(DAY)
        };
        let mut d = first_day;
        while d <= last_day {
            out.entry(d).or_default().push(iv);
            d += 1;
        }
    }
    // Each day's list is sorted by `start_at` for a stable render order.
    for list in out.values_mut() {
        list.sort_by(|a, b| (a.start_at, &a.vod_id).cmp(&(b.start_at, &b.vod_id)));
    }
    out
}

/// Co-stream hit: another interval overlapping a reference, paired
/// with the number of seconds of overlap (> 0).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CoStream {
    pub interval: Interval,
    pub overlap_seconds: i64,
}

/// For a given `around` interval, return every *other* interval that
/// overlaps it, sorted by overlap length descending. The same
/// streamer's own intervals are excluded — co-streams means "different
/// streamer was live at the same time", not "this streamer's other
/// clips from the same session".
pub fn find_co_streams(around: &Interval, all: &[Interval]) -> Vec<CoStream> {
    let mut hits: Vec<CoStream> = all
        .iter()
        .filter(|other| other.vod_id != around.vod_id && other.streamer_id != around.streamer_id)
        .filter_map(|other| {
            overlapping(around, other).map(|intersection| CoStream {
                interval: other.clone(),
                overlap_seconds: intersection.duration_seconds(),
            })
        })
        .collect();
    hits.sort_by_key(|h| std::cmp::Reverse(h.overlap_seconds));
    hits
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    fn iv(vid: &str, sid: &str, start: i64, end: i64) -> Interval {
        Interval::new(vid, sid, start, end)
    }

    #[test]
    fn new_coerces_negative_duration_to_zero() {
        let point = Interval::new("v1", "s1", 100, 50);
        assert_eq!(point.start_at, 100);
        assert_eq!(point.end_at, 100);
        assert_eq!(point.duration_seconds(), 0);
    }

    #[test]
    fn overlapping_disjoint_returns_none() {
        let a = iv("v1", "s1", 0, 100);
        let b = iv("v2", "s2", 200, 300);
        assert_eq!(overlapping(&a, &b), None);
    }

    #[test]
    fn overlapping_touching_endpoints_returns_none() {
        let a = iv("v1", "s1", 0, 100);
        let b = iv("v2", "s2", 100, 200);
        assert_eq!(overlapping(&a, &b), None);
    }

    #[test]
    fn overlapping_partial_returns_intersection() {
        let a = iv("v1", "s1", 0, 100);
        let b = iv("v2", "s2", 50, 150);
        let got = overlapping(&a, &b).expect("overlap");
        assert_eq!(got.start_at, 50);
        assert_eq!(got.end_at, 100);
    }

    #[test]
    fn overlapping_nested_returns_inner() {
        let outer = iv("v1", "s1", 0, 1000);
        let inner = iv("v2", "s2", 200, 300);
        let got = overlapping(&outer, &inner).expect("overlap");
        assert_eq!(got.start_at, 200);
        assert_eq!(got.end_at, 300);
    }

    #[test]
    fn bucket_single_day_groups_under_one_key() {
        let day = 86_400;
        let ivs = vec![
            iv("v1", "s1", day, day + 100),
            iv("v2", "s1", day + 500, day + 700),
        ];
        let buckets = bucket_by_day(&ivs);
        assert_eq!(buckets.len(), 1);
        assert_eq!(buckets[&1].len(), 2);
    }

    #[test]
    fn bucket_span_midnight_appears_in_both_days() {
        let ivs = vec![iv("v1", "s1", 86_000, 87_000)]; // crosses 86_400
        let buckets = bucket_by_day(&ivs);
        assert_eq!(buckets.len(), 2);
        assert_eq!(buckets[&0].len(), 1);
        assert_eq!(buckets[&1].len(), 1);
    }

    #[test]
    fn bucket_endpoint_at_midnight_belongs_to_prior_day() {
        let day2 = 2 * 86_400;
        let ivs = vec![iv("v1", "s1", day2 - 10, day2)];
        let buckets = bucket_by_day(&ivs);
        assert_eq!(buckets.len(), 1);
        assert!(buckets.contains_key(&1));
    }

    #[test]
    fn find_co_streams_excludes_same_vod_and_same_streamer() {
        let around = iv("v1", "s1", 0, 1000);
        let all = vec![
            around.clone(),             // same vod
            iv("v1b", "s1", 100, 200),  // same streamer
            iv("v2", "s2", 100, 500),   // real co-stream, 400 s overlap
            iv("v3", "s3", 600, 2000),  // real co-stream, 400 s overlap
            iv("v4", "s4", 2000, 3000), // disjoint
        ];
        let hits = find_co_streams(&around, &all);
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].overlap_seconds, 400);
        assert_eq!(hits[1].overlap_seconds, 400);
        // Ordering is ties-preserving but by overlap DESC — both OK order.
        assert!(matches!(hits[0].interval.streamer_id.as_str(), "s2" | "s3"));
    }

    #[test]
    fn find_co_streams_sorted_by_overlap_desc() {
        let around = iv("v1", "s1", 0, 1000);
        let all = vec![
            iv("va", "sa", 500, 600), // 100 s overlap
            iv("vb", "sb", 0, 999),   // 999 s overlap
            iv("vc", "sc", 200, 300), // 100 s overlap
        ];
        let hits = find_co_streams(&around, &all);
        assert_eq!(hits[0].interval.streamer_id, "sb");
        assert_eq!(hits[0].overlap_seconds, 999);
    }

    // -------- property tests --------
    // These run under `cargo test --all-features` via the `proptest`
    // dev-dependency; they exist to catch off-by-one issues and the
    // commutativity of `overlapping` over random inputs.
    use proptest::prelude::*;

    fn any_interval(prefix: &'static str) -> impl Strategy<Value = Interval> {
        (0i64..100_000, 0i64..100_000, 0usize..1_000_000).prop_map(move |(a, len, id)| {
            Interval::new(
                format!("{prefix}-{id}"),
                format!("{prefix}-streamer-{}", id % 20),
                a,
                a.saturating_add(len),
            )
        })
    }

    proptest! {
        #[test]
        fn overlapping_is_symmetric(a in any_interval("a"), b in any_interval("b")) {
            let ab = overlapping(&a, &b).map(|iv| (iv.start_at, iv.end_at));
            let ba = overlapping(&b, &a).map(|iv| (iv.start_at, iv.end_at));
            prop_assert_eq!(ab, ba);
        }

        #[test]
        fn overlap_duration_never_exceeds_either_input(a in any_interval("a"), b in any_interval("b")) {
            if let Some(inter) = overlapping(&a, &b) {
                prop_assert!(inter.duration_seconds() <= a.duration_seconds());
                prop_assert!(inter.duration_seconds() <= b.duration_seconds());
            }
        }

        #[test]
        fn co_streams_overlap_is_positive(around in any_interval("around"), others in proptest::collection::vec(any_interval("o"), 0..30)) {
            let hits = find_co_streams(&around, &others);
            for h in &hits {
                prop_assert!(h.overlap_seconds > 0);
                prop_assert_ne!(&h.interval.streamer_id, &around.streamer_id);
                prop_assert_ne!(&h.interval.vod_id, &around.vod_id);
            }
        }

        #[test]
        fn co_streams_sorted_descending(around in any_interval("around"), others in proptest::collection::vec(any_interval("o"), 0..30)) {
            let hits = find_co_streams(&around, &others);
            for pair in hits.windows(2) {
                prop_assert!(pair[0].overlap_seconds >= pair[1].overlap_seconds);
            }
        }

        #[test]
        fn bucket_preserves_every_interval(ivs in proptest::collection::vec(any_interval("b"), 0..50)) {
            let buckets = bucket_by_day(&ivs);
            for iv in &ivs {
                // Find at least one bucket containing a matching vod id.
                let mut found = false;
                for bucket in buckets.values() {
                    if bucket.iter().any(|b| b.vod_id == iv.vod_id) { found = true; break; }
                }
                prop_assert!(found, "interval missing from buckets: {iv:?}");
            }
        }
    }
}
