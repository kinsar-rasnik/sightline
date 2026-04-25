//! Parity test: the Rust domain math agrees with the JS frontend
//! mirror byte-for-byte against a shared fixture.
//!
//! The fixture lives at
//! `src/features/multiview/sync-math.fixture.json` (reachable from
//! `src-tauri/tests/` as `../src/features/multiview/sync-math.fixture.json`).
//! Both this test and `src/features/multiview/sync-math.test.ts`
//! consume it; any divergence between the two implementations fails
//! whichever site is wrong.
//!
//! See ADR-0022 §Wall-clock anchor model.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;

use serde::Deserialize;

use sightline_lib::domain::deep_link::{DeepLinkContext, resolve_deep_link_target};
use sightline_lib::domain::sync::{MemberRange, compute_overlap, is_member_out_of_range};

#[derive(Deserialize)]
struct DeepLinkCase {
    name: String,
    moment_unix_seconds: i64,
    target_stream_started_at: i64,
    target_duration_seconds: i64,
    expected: f64,
}

#[derive(Deserialize)]
struct OverlapCase {
    name: String,
    members: Vec<FixtureMember>,
    expected_start_at: i64,
    expected_end_at: i64,
}

#[derive(Deserialize)]
struct FixtureMember {
    stream_started_at: i64,
    duration_seconds: i64,
}

#[derive(Deserialize)]
struct OutOfRangeCase {
    name: String,
    moment: i64,
    member_start: i64,
    member_duration: i64,
    expected: bool,
}

#[derive(Deserialize)]
struct Fixture {
    deep_link: Vec<DeepLinkCase>,
    overlap: Vec<OverlapCase>,
    out_of_range: Vec<OutOfRangeCase>,
}

fn load_fixture() -> Fixture {
    // The Cargo working directory for `cargo test` is `src-tauri/`, so
    // the fixture resolves under `../src/...`.
    let path = Path::new("../src/features/multiview/sync-math.fixture.json");
    let raw =
        fs::read_to_string(path).unwrap_or_else(|e| panic!("read fixture {}: {e}", path.display()));
    // Strip the JSON's `$schema-note` documentation key — it's not
    // part of the test data and serde would reject it as unknown.
    let cleaned: serde_json::Value = serde_json::from_str(&raw).unwrap();
    let mut obj = cleaned.as_object().unwrap().clone();
    obj.remove("$schema-note");
    let recombined = serde_json::Value::Object(obj);

    serde_json::from_value::<Fixture>(snake_case(recombined))
        .unwrap_or_else(|e| panic!("decode fixture: {e}"))
}

/// Recursively rename camelCase JSON keys to snake_case so the Rust
/// `Deserialize` impls (with serde defaults) can pick them up.
/// Keeps the original numeric / string values intact.
fn snake_case(value: serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(map) => {
            let mut out = serde_json::Map::new();
            for (k, v) in map {
                out.insert(camel_to_snake(&k), snake_case(v));
            }
            serde_json::Value::Object(out)
        }
        serde_json::Value::Array(arr) => {
            serde_json::Value::Array(arr.into_iter().map(snake_case).collect())
        }
        v => v,
    }
}

fn camel_to_snake(name: &str) -> String {
    let mut out = String::with_capacity(name.len() + 4);
    for (i, c) in name.chars().enumerate() {
        if c.is_ascii_uppercase() {
            if i > 0 {
                out.push('_');
            }
            out.push(c.to_ascii_lowercase());
        } else {
            out.push(c);
        }
    }
    out
}

#[test]
fn deep_link_parity() {
    let fx = load_fixture();
    for case in fx.deep_link {
        let got = resolve_deep_link_target(DeepLinkContext {
            moment_unix_seconds: case.moment_unix_seconds,
            target_stream_started_at: case.target_stream_started_at,
            target_duration_seconds: case.target_duration_seconds,
        });
        assert_eq!(
            got, case.expected,
            "deepLink case '{}' mismatch: rust={got} fixture={}",
            case.name, case.expected
        );
    }
}

#[test]
fn overlap_parity() {
    let fx = load_fixture();
    for case in fx.overlap {
        let members: Vec<MemberRange> = case
            .members
            .iter()
            .map(|m| MemberRange {
                stream_started_at: m.stream_started_at,
                duration_seconds: m.duration_seconds,
            })
            .collect();
        let w = compute_overlap(&members);
        assert_eq!(
            w.start_at, case.expected_start_at,
            "overlap case '{}' start_at mismatch: rust={} fixture={}",
            case.name, w.start_at, case.expected_start_at
        );
        assert_eq!(
            w.end_at, case.expected_end_at,
            "overlap case '{}' end_at mismatch: rust={} fixture={}",
            case.name, w.end_at, case.expected_end_at
        );
    }
}

#[test]
fn out_of_range_parity() {
    let fx = load_fixture();
    for case in fx.out_of_range {
        let got = is_member_out_of_range(case.moment, case.member_start, case.member_duration);
        assert_eq!(
            got, case.expected,
            "outOfRange case '{}' mismatch: rust={got} fixture={}",
            case.name, case.expected
        );
    }
}
