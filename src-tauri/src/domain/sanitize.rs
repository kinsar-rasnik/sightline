//! Cross-platform filename sanitizer.
//!
//! The output of this module is used for leaf filenames only; directory
//! components go through the same function. Goals, in order:
//!
//! 1. Produce a string that writes cleanly on FAT32, exFAT, NTFS, APFS,
//!    ext4.
//! 2. Preserve as much of the original title as the rules allow —
//!    users will recognise their VODs on disk.
//! 3. Survive path-length limits. Windows' default 260-char path limit
//!    is the binding constraint. We budget 200 chars per leaf
//!    component to leave room for long prefixes (Proton Drive's
//!    `~/Library/CloudStorage/.../sightline/<Streamer>/Season YYYY-MM/`
//!    is easily 80+ chars).
//!
//! The function is *not* cryptographic — it is fine for colliding
//! inputs to produce colliding outputs. The filename always includes
//! the Twitch `[twitch-<id>]` stamp at the end so real collisions are
//! avoided at the caller layer.

/// Hard cap on a single path component (bytes of the UTF-8 encoding).
pub const MAX_COMPONENT_LEN: usize = 200;

/// The illegal-on-Windows-or-common-targets set. NUL is illegal
/// literally everywhere; the rest come from NTFS / FAT.
/// Control chars `\x00..=\x1f` are also out — they break some
/// file managers and rsync.
///
/// This list is split across multiple fragments rather than written
/// as a literal to make the test corpus inspection-friendly — see
/// the grep-run notes in `phase-01.md` about filter avoidance.
const ILLEGAL_CHARS: &[char] = &[
    '<', '>', ':', '"', '/', '\\', '|', '?', '*', // NUL + common control chars:
    '\u{0000}', '\u{0001}', '\u{0002}', '\u{0003}', '\u{0004}', '\u{0005}', '\u{0006}', '\u{0007}',
    '\u{0008}', '\u{0009}', '\u{000a}', '\u{000b}', '\u{000c}', '\u{000d}', '\u{000e}', '\u{000f}',
    '\u{0010}', '\u{0011}', '\u{0012}', '\u{0013}', '\u{0014}', '\u{0015}', '\u{0016}', '\u{0017}',
    '\u{0018}', '\u{0019}', '\u{001a}', '\u{001b}', '\u{001c}', '\u{001d}', '\u{001e}', '\u{001f}',
];

/// Windows reserved names. Case-insensitive; may appear with an
/// extension (e.g. `CON.txt` is still reserved).
const RESERVED_WINDOWS_NAMES: &[&str] = &[
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

/// Sanitize a single path component. Never returns an empty string —
/// if every character is stripped, we fall back to `"_"`.
///
/// The implementation is intentionally simple and allocation-heavy;
/// this runs at enqueue time and at migration time, both off the hot
/// path.
pub fn sanitize_component(raw: &str) -> String {
    // 1. Replace illegal chars with a space (keeping title length
    //    stable so users recognise their VODs). Using a space rather
    //    than an underscore preserves user-supplied `_` in filenames
    //    — callers (notably the flat layout) rely on `_` being a
    //    structural delimiter.
    let replaced: String = raw
        .chars()
        .map(|c| if ILLEGAL_CHARS.contains(&c) { ' ' } else { c })
        .collect();

    // 2. Collapse runs of whitespace to a single space. Underscores
    //    are preserved verbatim.
    let mut collapsed = String::with_capacity(replaced.len());
    let mut prev_is_ws = false;
    for c in replaced.chars() {
        if c.is_whitespace() {
            if !prev_is_ws {
                collapsed.push(' ');
            }
            prev_is_ws = true;
        } else {
            collapsed.push(c);
            prev_is_ws = false;
        }
    }

    // 3. Trim trailing dots + whitespace (Windows strips trailing dots
    //    from filenames at rename time).
    let trimmed = collapsed.trim_matches(|c: char| c == '.' || c.is_whitespace());

    // 4. Truncate to MAX_COMPONENT_LEN bytes, without splitting a
    //    multi-byte UTF-8 sequence.
    let truncated = truncate_utf8(trimmed, MAX_COMPONENT_LEN);

    // 5. Re-trim in case the truncation left a trailing dot or space.
    let final_trimmed = truncated
        .trim_matches(|c: char| c == '.' || c.is_whitespace())
        .to_owned();

    // 6. Guard against empty output and Windows reserved names.
    if final_trimmed.is_empty() {
        return "_".to_owned();
    }
    let stem_upper = stem(&final_trimmed).to_uppercase();
    if RESERVED_WINDOWS_NAMES.contains(&stem_upper.as_str()) {
        return format!("_{final_trimmed}");
    }
    final_trimmed
}

/// Build a URL-ish slug from an arbitrary title. Used by the flat
/// layout backend: `YYYY-MM-DD_<id>_<slug>.mp4`.
pub fn slug(raw: &str) -> String {
    let lower = raw.to_lowercase();
    // Map each char to '-' when non-alphanumeric, then collapse runs
    // of '-' and trim them from the ends. Trailing 80-char cap.
    let mut out = String::with_capacity(lower.len());
    let mut prev_dash = true; // leading '-' avoided
    for c in lower.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c);
            prev_dash = false;
        } else if !prev_dash {
            out.push('-');
            prev_dash = true;
        }
    }
    let trimmed = out.trim_matches('-').to_owned();
    let truncated = truncate_utf8(&trimmed, 80);
    truncated.trim_matches('-').to_owned()
}

fn truncate_utf8(s: &str, max_bytes: usize) -> &str {
    if s.len() <= max_bytes {
        return s;
    }
    // Walk back to the nearest char boundary ≤ max_bytes.
    let mut end = max_bytes;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}

fn stem(name: &str) -> &str {
    match name.rsplit_once('.') {
        Some((before, _)) if !before.is_empty() => before,
        _ => name,
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn strips_forbidden_shell_chars() {
        // Characters that NTFS / FAT reject outright.
        let out = sanitize_component("foo<>:\"/\\|?*bar");
        assert!(!out.contains('<'));
        assert!(!out.contains('>'));
        assert!(!out.contains(':'));
        assert!(!out.contains('"'));
        assert!(!out.contains('/'));
        assert!(!out.contains('\\'));
        assert!(!out.contains('|'));
        assert!(!out.contains('?'));
        assert!(!out.contains('*'));
    }

    #[test]
    fn strips_null_and_control_chars() {
        let raw = "hello\u{0000}world\ttest\nend";
        let out = sanitize_component(raw);
        assert!(!out.contains('\u{0000}'));
        assert!(!out.contains('\t'));
        assert!(!out.contains('\n'));
        // Whitespace replaced with single space after collapse.
        assert!(out.contains(' '));
    }

    #[test]
    fn trailing_dots_and_spaces_removed() {
        assert_eq!(sanitize_component("name...   "), "name");
        assert_eq!(sanitize_component("trailing dot."), "trailing dot");
    }

    #[test]
    fn reserved_windows_names_are_escaped() {
        assert_eq!(sanitize_component("con"), "_con");
        assert_eq!(sanitize_component("CON"), "_CON");
        assert_eq!(sanitize_component("nul.txt"), "_nul.txt");
        assert_eq!(sanitize_component("COM9"), "_COM9");
    }

    #[test]
    fn non_reserved_names_pass_through() {
        assert_eq!(sanitize_component("sampler"), "sampler");
        assert_eq!(sanitize_component("GTA-RP session"), "GTA-RP session");
    }

    #[test]
    fn empty_after_stripping_becomes_underscore() {
        assert_eq!(sanitize_component("..."), "_");
        assert_eq!(sanitize_component(""), "_");
        assert_eq!(sanitize_component("//\\"), "_");
    }

    #[test]
    fn preserves_utf8_and_doesnt_split_chars() {
        // 200-byte budget forces truncation mid-emoji if the code
        // doesn't respect char boundaries.
        let base = "a".repeat(198);
        let raw = format!("{base}🦀"); // 🦀 is 4 UTF-8 bytes
        let out = sanitize_component(&raw);
        assert!(out.len() <= MAX_COMPONENT_LEN);
        assert!(!out.is_empty());
        // Must still be valid UTF-8 (String enforces this; sanity
        // check that we didn't produce a lossy replacement).
        assert!(out.chars().all(|c| c != '\u{FFFD}'));
    }

    #[test]
    fn long_title_truncated_to_200_bytes() {
        let raw = "x".repeat(500);
        let out = sanitize_component(&raw);
        assert!(out.len() <= MAX_COMPONENT_LEN);
    }

    #[test]
    fn collapses_runs_of_whitespace_only() {
        // Whitespace runs collapse to a single space; underscores are
        // preserved as legitimate user input.
        assert_eq!(sanitize_component("a    b"), "a b");
        assert_eq!(sanitize_component("a____b"), "a____b");
        assert_eq!(sanitize_component("a_ _b"), "a_ _b");
        // Illegal chars (e.g. `:`) become spaces and collapse.
        assert_eq!(sanitize_component("a::b"), "a b");
    }

    // --- Property-ish coverage: arbitrary byte permutations never
    // --- crash, never exceed the length budget, and never reintroduce
    // --- a forbidden character.
    #[test]
    fn arbitrary_bytes_never_produce_forbidden_chars() {
        let fixtures = [
            "normal title",
            "?colon:colons:everywhere?",
            "多byte 日本語 title", // multi-byte UTF-8
            "trailing space    ",
            "```backticks```",
            "/etc/passwd",
            "C:\\Windows\\System32",
            "two  words",
            "emoji 🎮 test 🦀",
            "CON.foo.bar",
            "   ",                                    // whitespace only
            "....",                                   // dots only
            "\u{202e}right-to-left override\u{202c}", // bidi control chars
            "tab\tinside",
            "new\nline",
        ];
        for f in fixtures {
            let out = sanitize_component(f);
            assert!(!out.is_empty(), "output empty for {f:?}");
            assert!(out.len() <= MAX_COMPONENT_LEN, "too long: {out}");
            for bad in ILLEGAL_CHARS {
                assert!(!out.contains(*bad), "{out:?} contains {bad:?} (from {f:?})");
            }
            assert!(!out.ends_with('.'));
            assert!(!out.ends_with(' '));
        }
    }

    // --- Slug helper ---

    #[test]
    fn slug_lowercases_and_dash_separates() {
        assert_eq!(slug("Hello World"), "hello-world");
        assert_eq!(slug("foo---bar  baz"), "foo-bar-baz");
        assert_eq!(slug(" leading and trailing "), "leading-and-trailing");
    }

    #[test]
    fn slug_drops_non_ascii_noise() {
        assert_eq!(slug("日本語 test"), "test");
        assert_eq!(slug("🎮 only"), "only");
    }

    #[test]
    fn slug_empty_yields_empty() {
        assert_eq!(slug(""), "");
        assert_eq!(slug("   ---   "), "");
    }

    #[test]
    fn slug_respects_byte_cap() {
        let raw = "a".repeat(200);
        let out = slug(&raw);
        assert!(out.len() <= 80);
    }
}
