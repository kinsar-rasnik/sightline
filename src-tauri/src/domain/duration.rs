//! Parser for Twitch's ISO-like duration strings (e.g. `1h23m45s`).
//!
//! The format is mostly well-behaved but has these quirks:
//! * Components are always in descending order (`h`, `m`, `s`).
//! * Any component may be absent (`0h5m0s` and `5m` and `300s` all valid).
//! * `0s` / `0m0s` roundtrips to zero.
//! * Components larger than `h` (i.e. days) are **not** emitted by Helix
//!   — a multi-day stream reports `49h0m0s` rather than `2d1h0m0s`.

use std::str::FromStr;

use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ParseTwitchDurationError {
    #[error("empty duration string")]
    Empty,

    #[error("unexpected character {0:?} in duration")]
    UnexpectedChar(char),

    #[error("duration component overflow")]
    Overflow,

    #[error("out-of-order component {0:?} (expected descending h/m/s)")]
    OutOfOrder(char),

    #[error("duplicate component {0:?}")]
    Duplicate(char),

    #[error("trailing digits with no unit suffix")]
    TrailingDigits,
}

/// Parse a Helix duration string into seconds. Leading/trailing whitespace
/// is stripped; components may appear in any subset of `{h, m, s}` but must
/// remain in descending order; all numeric parts are non-negative integers.
pub fn parse_helix_duration(s: &str) -> Result<i64, ParseTwitchDurationError> {
    let s = s.trim();
    if s.is_empty() {
        return Err(ParseTwitchDurationError::Empty);
    }

    let mut total: i64 = 0;
    let mut current: i64 = 0;
    let mut has_digit = false;
    let mut seen = Components::default();

    for ch in s.chars() {
        match ch {
            '0'..='9' => {
                has_digit = true;
                current = current
                    .checked_mul(10)
                    .and_then(|v| v.checked_add((ch as i64) - ('0' as i64)))
                    .ok_or(ParseTwitchDurationError::Overflow)?;
            }
            'h' | 'H' => {
                if !has_digit {
                    return Err(ParseTwitchDurationError::UnexpectedChar(ch));
                }
                if seen.hours {
                    return Err(ParseTwitchDurationError::Duplicate('h'));
                }
                if seen.minutes || seen.seconds {
                    return Err(ParseTwitchDurationError::OutOfOrder('h'));
                }
                seen.hours = true;
                total = total
                    .checked_add(
                        current
                            .checked_mul(3600)
                            .ok_or(ParseTwitchDurationError::Overflow)?,
                    )
                    .ok_or(ParseTwitchDurationError::Overflow)?;
                current = 0;
                has_digit = false;
            }
            'm' | 'M' => {
                if !has_digit {
                    return Err(ParseTwitchDurationError::UnexpectedChar(ch));
                }
                if seen.minutes {
                    return Err(ParseTwitchDurationError::Duplicate('m'));
                }
                if seen.seconds {
                    return Err(ParseTwitchDurationError::OutOfOrder('m'));
                }
                seen.minutes = true;
                total = total
                    .checked_add(
                        current
                            .checked_mul(60)
                            .ok_or(ParseTwitchDurationError::Overflow)?,
                    )
                    .ok_or(ParseTwitchDurationError::Overflow)?;
                current = 0;
                has_digit = false;
            }
            's' | 'S' => {
                if !has_digit {
                    return Err(ParseTwitchDurationError::UnexpectedChar(ch));
                }
                if seen.seconds {
                    return Err(ParseTwitchDurationError::Duplicate('s'));
                }
                seen.seconds = true;
                total = total
                    .checked_add(current)
                    .ok_or(ParseTwitchDurationError::Overflow)?;
                current = 0;
                has_digit = false;
            }
            other => return Err(ParseTwitchDurationError::UnexpectedChar(other)),
        }
    }

    if has_digit {
        return Err(ParseTwitchDurationError::TrailingDigits);
    }

    Ok(total)
}

/// Convenience wrapper for code paths that already have an owned `String`.
impl FromStr for TwitchDuration {
    type Err = ParseTwitchDurationError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parse_helix_duration(s).map(TwitchDuration)
    }
}

/// Thin wrapper so call sites can say `TwitchDuration::from_str(...)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TwitchDuration(pub i64);

#[derive(Default)]
struct Components {
    hours: bool,
    minutes: bool,
    seconds: bool,
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn parses_h_m_s() {
        assert_eq!(
            parse_helix_duration("1h23m45s").unwrap(),
            3600 + 23 * 60 + 45
        );
    }

    #[test]
    fn parses_subsets() {
        assert_eq!(parse_helix_duration("5m").unwrap(), 300);
        assert_eq!(parse_helix_duration("300s").unwrap(), 300);
        assert_eq!(parse_helix_duration("2h0m0s").unwrap(), 7200);
        assert_eq!(parse_helix_duration("2h30s").unwrap(), 7230);
    }

    #[test]
    fn accepts_case_variants() {
        assert_eq!(parse_helix_duration("1H2M3S").unwrap(), 3723);
    }

    #[test]
    fn trims_whitespace() {
        assert_eq!(parse_helix_duration("  1h0m0s ").unwrap(), 3600);
    }

    #[test]
    fn rejects_empty() {
        assert!(matches!(
            parse_helix_duration(""),
            Err(ParseTwitchDurationError::Empty)
        ));
    }

    #[test]
    fn rejects_trailing_digits() {
        assert!(matches!(
            parse_helix_duration("1h23"),
            Err(ParseTwitchDurationError::TrailingDigits)
        ));
    }

    #[test]
    fn rejects_unit_without_digits() {
        assert!(matches!(
            parse_helix_duration("h1m"),
            Err(ParseTwitchDurationError::UnexpectedChar('h'))
        ));
    }

    #[test]
    fn rejects_out_of_order() {
        assert!(matches!(
            parse_helix_duration("5m1h"),
            Err(ParseTwitchDurationError::OutOfOrder('h'))
        ));
        assert!(matches!(
            parse_helix_duration("30s2m"),
            Err(ParseTwitchDurationError::OutOfOrder('m'))
        ));
    }

    #[test]
    fn rejects_duplicates() {
        assert!(matches!(
            parse_helix_duration("5m5m"),
            Err(ParseTwitchDurationError::Duplicate('m'))
        ));
    }

    #[test]
    fn rejects_garbage() {
        assert!(matches!(
            parse_helix_duration("abc"),
            Err(ParseTwitchDurationError::UnexpectedChar('a'))
        ));
    }

    #[test]
    fn roundtrips_wrapper() {
        let d: TwitchDuration = "1h".parse().unwrap();
        assert_eq!(d.0, 3600);
    }

    #[test]
    fn accepts_multi_day_hours() {
        // Helix emits 49h for a two-day VOD; we accept it.
        assert_eq!(parse_helix_duration("49h0m0s").unwrap(), 49 * 3600);
    }
}
