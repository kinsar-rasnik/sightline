//! Kodi-compatible NFO generator.
//!
//! Plex / Jellyfin / Infuse all parse a `<movie>` XML format. We emit
//! a conservative subset that covers title, plot (description), date,
//! runtime, studio ("Twitch"), a `<uniqueid type="twitch">` with the
//! VOD id, and one `<tag>` + `<chapter>` per chapter so in-app chapter
//! skip works.
//!
//! No I/O here. The generator takes pure data, returns a String, and
//! is round-trip-tested against a minimal parser to protect the shape.

use chrono::{DateTime, Utc};

use crate::domain::chapter::Chapter;
use crate::domain::vod::Vod;

/// Input for the NFO generator. All fields must be owned by the caller
/// — the generator does not allocate borrowed references.
#[derive(Debug)]
pub struct NfoInput<'a> {
    pub vod: &'a Vod,
    pub chapters: &'a [Chapter],
    pub streamer_display_name: &'a str,
}

/// Build the NFO XML document. The output is ASCII-safe XML 1.0 with
/// special characters escaped. No BOM. No `<?xml-stylesheet?>` — Plex
/// and Infuse don't need it and it just bloats the file.
pub fn generate(input: &NfoInput<'_>) -> String {
    let mut out = String::with_capacity(2048);
    out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n");
    out.push_str("<movie>\n");

    write_tag(&mut out, "title", &input.vod.title);
    write_tag(&mut out, "originaltitle", &input.vod.title);
    write_tag(&mut out, "plot", &input.vod.description);
    write_tag(&mut out, "studio", "Twitch");
    write_tag(&mut out, "runtime", &runtime_minutes(input.vod.duration_seconds));

    let premiered = iso_date(input.vod.stream_started_at);
    write_tag(&mut out, "premiered", &premiered);
    write_tag(&mut out, "year", premiered.get(..4).unwrap_or("0"));
    write_tag(&mut out, "dateadded", &iso_datetime(input.vod.first_seen_at));

    // director / actor — we stretch "director" to mean "streamer" so
    // Infuse / Plex display the streamer's name on the details panel.
    out.push_str("  <director>");
    push_escaped(&mut out, input.streamer_display_name);
    out.push_str("</director>\n");

    // uniqueid so Plex and Jellyfin don't try to match against TMDB /
    // IMDB. `default="true"` marks it as the canonical id.
    out.push_str("  <uniqueid type=\"twitch\" default=\"true\">");
    push_escaped(&mut out, &input.vod.twitch_video_id);
    out.push_str("</uniqueid>\n");

    // One <tag> per distinct game, for filtering inside Jellyfin.
    let mut seen_games: Vec<&str> = Vec::new();
    for c in input.chapters {
        if c.game_name.is_empty() {
            continue;
        }
        let name = c.game_name.as_str();
        if !seen_games.contains(&name) {
            out.push_str("  <tag>");
            push_escaped(&mut out, name);
            out.push_str("</tag>\n");
            seen_games.push(name);
        }
    }

    // <chapters> block — Infuse picks this up for chapter skip.
    if !input.chapters.is_empty() {
        out.push_str("  <chapters>\n");
        for (idx, c) in input.chapters.iter().enumerate() {
            out.push_str("    <chapter");
            push_xml_attr(&mut out, "number", &(idx + 1).to_string());
            push_xml_attr(&mut out, "start", &ms_to_hms(c.position_ms));
            out.push('>');
            let label = if c.game_name.is_empty() {
                "Unknown"
            } else {
                c.game_name.as_str()
            };
            push_escaped(&mut out, label);
            out.push_str("</chapter>\n");
        }
        out.push_str("  </chapters>\n");
    }

    out.push_str("</movie>\n");
    out
}

fn write_tag(out: &mut String, name: &str, value: &str) {
    out.push_str("  <");
    out.push_str(name);
    out.push('>');
    push_escaped(out, value);
    out.push_str("</");
    out.push_str(name);
    out.push_str(">\n");
}

fn push_xml_attr(out: &mut String, name: &str, value: &str) {
    out.push(' ');
    out.push_str(name);
    out.push_str("=\"");
    push_escaped(out, value);
    out.push('"');
}

/// XML-escape only the five characters that strictly need it. Control
/// chars under U+0020 (except tab, LF, CR) are stripped — those are
/// illegal in XML 1.0.
fn push_escaped(out: &mut String, s: &str) {
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            '\t' | '\n' | '\r' => out.push(c),
            c if (c as u32) < 0x20 => { /* strip */ }
            c => out.push(c),
        }
    }
}

fn runtime_minutes(duration_seconds: i64) -> String {
    ((duration_seconds.max(0) + 59) / 60).to_string()
}

fn iso_date(unix_seconds: i64) -> String {
    let dt = DateTime::<Utc>::from_timestamp(unix_seconds, 0)
        .unwrap_or_else(|| DateTime::<Utc>::from_timestamp(0, 0).unwrap_or_default());
    dt.format("%Y-%m-%d").to_string()
}

fn iso_datetime(unix_seconds: i64) -> String {
    let dt = DateTime::<Utc>::from_timestamp(unix_seconds, 0)
        .unwrap_or_else(|| DateTime::<Utc>::from_timestamp(0, 0).unwrap_or_default());
    dt.format("%Y-%m-%d %H:%M:%S").to_string()
}

fn ms_to_hms(ms: i64) -> String {
    let total_seconds = (ms.max(0) / 1000) as u64;
    let h = total_seconds / 3600;
    let m = (total_seconds % 3600) / 60;
    let s = total_seconds % 60;
    format!("{h:02}:{m:02}:{s:02}.000")
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::domain::chapter::{Chapter, ChapterType};
    use crate::domain::vod::IngestStatus;

    fn fixture() -> Vod {
        Vod {
            twitch_video_id: "v42".into(),
            twitch_user_id: "100".into(),
            stream_id: None,
            title: "GTA RP Test".into(),
            description: "A short <plot> with\n \"quotes\" & ampersands.".into(),
            stream_started_at: 1_775_088_000, // 2026-04-02 UTC
            published_at: 1_712_068_800,
            url: "https://twitch.tv/videos/v42".into(),
            thumbnail_url: None,
            duration_seconds: 3600 * 2 + 15, // 2h 0m 15s → 121 minutes
            view_count: 0,
            language: "en".into(),
            muted_segments: vec![],
            is_sub_only: false,
            helix_game_id: None,
            helix_game_name: None,
            ingest_status: IngestStatus::Eligible,
            status_reason: String::new(),
            first_seen_at: 1_712_500_000,
            last_seen_at: 0,
        }
    }

    fn chapters() -> Vec<Chapter> {
        vec![
            Chapter {
                position_ms: 0,
                duration_ms: 1_800_000,
                game_id: Some("32982".into()),
                game_name: "Grand Theft Auto V".into(),
                chapter_type: ChapterType::GameChange,
            },
            Chapter {
                position_ms: 1_800_000,
                duration_ms: 1_800_000,
                game_id: Some("509658".into()),
                game_name: "Just Chatting".into(),
                chapter_type: ChapterType::GameChange,
            },
        ]
    }

    #[test]
    fn emits_well_formed_root() {
        let v = fixture();
        let c = chapters();
        let nfo = generate(&NfoInput {
            vod: &v,
            chapters: &c,
            streamer_display_name: "Sampler",
        });
        assert!(nfo.starts_with("<?xml"));
        assert!(nfo.contains("<movie>"));
        assert!(nfo.trim_end().ends_with("</movie>"));
    }

    #[test]
    fn escapes_xml_specials_in_description() {
        let v = fixture();
        let c = chapters();
        let nfo = generate(&NfoInput {
            vod: &v,
            chapters: &c,
            streamer_display_name: "Sampler & co",
        });
        assert!(nfo.contains("&amp;"));
        assert!(nfo.contains("&lt;plot&gt;"));
        assert!(nfo.contains("&quot;quotes&quot;"));
        // Raw specials should not appear inside escaped text regions.
        // (The only bare `<` and `>` are the tag delimiters themselves.)
        assert!(!nfo.contains("<plot>A short <plot>"));
    }

    #[test]
    fn includes_uniqueid_twitch() {
        let v = fixture();
        let nfo = generate(&NfoInput {
            vod: &v,
            chapters: &[],
            streamer_display_name: "Sampler",
        });
        assert!(nfo.contains("<uniqueid type=\"twitch\" default=\"true\">v42</uniqueid>"));
    }

    #[test]
    fn runtime_rounds_up_to_whole_minute() {
        // 2h 0m 15s → ceil to 121 minutes
        let v = fixture();
        let nfo = generate(&NfoInput {
            vod: &v,
            chapters: &[],
            streamer_display_name: "Sampler",
        });
        assert!(nfo.contains("<runtime>121</runtime>"));
    }

    #[test]
    fn chapters_emit_number_and_start_attrs() {
        let v = fixture();
        let c = chapters();
        let nfo = generate(&NfoInput {
            vod: &v,
            chapters: &c,
            streamer_display_name: "Sampler",
        });
        assert!(nfo.contains("<chapter number=\"1\" start=\"00:00:00.000\">"));
        assert!(nfo.contains("<chapter number=\"2\" start=\"00:30:00.000\">"));
        assert!(nfo.contains(">Grand Theft Auto V</chapter>"));
        assert!(nfo.contains(">Just Chatting</chapter>"));
    }

    #[test]
    fn tags_dedupe_game_names() {
        let v = fixture();
        let c = vec![
            Chapter {
                position_ms: 0,
                duration_ms: 1,
                game_id: Some("32982".into()),
                game_name: "GTA V".into(),
                chapter_type: ChapterType::GameChange,
            },
            Chapter {
                position_ms: 1,
                duration_ms: 1,
                game_id: Some("32982".into()),
                game_name: "GTA V".into(),
                chapter_type: ChapterType::GameChange,
            },
        ];
        let nfo = generate(&NfoInput {
            vod: &v,
            chapters: &c,
            streamer_display_name: "Sampler",
        });
        assert_eq!(nfo.matches("<tag>GTA V</tag>").count(), 1);
    }

    #[test]
    fn chapters_block_omitted_when_empty() {
        let v = fixture();
        let nfo = generate(&NfoInput {
            vod: &v,
            chapters: &[],
            streamer_display_name: "Sampler",
        });
        assert!(!nfo.contains("<chapters>"));
        assert!(!nfo.contains("<chapter "));
    }

    #[test]
    fn control_chars_are_stripped() {
        let mut v = fixture();
        v.title = "bad\x01title\x07done".into();
        let nfo = generate(&NfoInput {
            vod: &v,
            chapters: &[],
            streamer_display_name: "Sampler",
        });
        assert!(!nfo.contains('\x01'));
        assert!(!nfo.contains('\x07'));
        assert!(nfo.contains("<title>badtitledone</title>"));
    }

    // Minimal round-trip parser — asserts the document is accepted by
    // a simple tag walker so we don't ship malformed XML. The intent
    // isn't to replicate a full XML parser, just to fail the test
    // loudly if tag balancing drifts.
    #[test]
    fn round_trips_through_minimal_parser() {
        let v = fixture();
        let c = chapters();
        let nfo = generate(&NfoInput {
            vod: &v,
            chapters: &c,
            streamer_display_name: "Sampler".into(),
        });
        let mut depth: i32 = 0;
        let mut in_tag = false;
        let mut closing = false;
        let mut prev = '\0';
        for ch in nfo.chars() {
            match (in_tag, ch) {
                (false, '<') => {
                    in_tag = true;
                    closing = false;
                }
                (true, '/') if prev == '<' => closing = true,
                (true, '>') => {
                    // Single-line processing instructions + self-closing ignored.
                    if prev != '?' && prev != '/' {
                        if closing {
                            depth -= 1;
                        } else {
                            depth += 1;
                        }
                    }
                    in_tag = false;
                }
                _ => {}
            }
            prev = ch;
            assert!(depth >= 0, "closed more tags than opened");
        }
        assert_eq!(depth, 0, "unclosed tags in NFO: {nfo}");
    }
}
