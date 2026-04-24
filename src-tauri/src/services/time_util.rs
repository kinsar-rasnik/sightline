//! Small time helpers shared across services.

use chrono::DateTime;

use crate::error::AppError;

/// Parse an RFC 3339 / ISO 8601 timestamp (as emitted by Helix) into
/// unix seconds UTC. Twitch uses `Z` suffix uniformly, but chrono
/// accepts both `+00:00` and `Z`.
pub fn parse_iso_to_unix(s: &str) -> Result<i64, AppError> {
    DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.timestamp())
        .map_err(|e| AppError::Parse {
            detail: format!("iso timestamp {s:?}: {e}"),
        })
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn parses_twitch_z_timestamp() {
        let secs = parse_iso_to_unix("2021-01-01T00:00:00Z").unwrap();
        assert_eq!(secs, 1_609_459_200);
    }

    #[test]
    fn parses_timestamp_with_offset() {
        // Same moment — 2021-01-01 00:00 UTC expressed as +02:00.
        let secs = parse_iso_to_unix("2021-01-01T02:00:00+02:00").unwrap();
        assert_eq!(secs, 1_609_459_200);
    }

    #[test]
    fn rejects_garbage() {
        assert!(matches!(
            parse_iso_to_unix("not-a-date"),
            Err(AppError::Parse { .. })
        ));
    }
}
