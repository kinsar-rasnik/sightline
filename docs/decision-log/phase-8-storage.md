# Phase 8 Decision Log — Storage-Aware Distribution → v2.0.0

Tracking calendar: 2026-04-26 onwards.  One entry per significant
decision, error, or judgment call during the run.  Used for the
Final-Report's "Decision-Log highlights" section.

Format: `## YYYY-MM-DD HH:MM UTC — short title`

---

## 2026-04-26 — Sub-Phase A: ADRs 0028-0033 drafted

Six ADRs drafted in one commit ahead of any code changes:

- **0028** Quality pipeline + 720p30 H.265 default.
- **0029** Background-friendly re-encode (nice + adaptive suspend).
- **0030** Pull distribution model with sliding window.
- **0031** Pre-fetch strategy (one-step lookahead, watch-progress
  triggered).
- **0032** Storage-forecast heuristic (avg VOD × frequency × quality
  factor × window).
- **0033** Library UI re-conception (unified available + downloaded
  list with filter chips).

Quality-factor table (ADR-0032) is sourced from a separate
`quality-factor-measurement.md` reference doc rather than embedded
in the ADR — keeps the ADR immutable while the table is updated when
ffmpeg sidecars rev.

R-RC-01 applied: ADRs are documentation only; trivial-Sub-Block
exception in `.claude/rules/review-cycles.md` § R-RC-01 doesn't
strictly apply (this Sub-Phase has > 100 lines diff), but the spirit
of "review before next sub-block" is satisfied by the cross-ADR
coherence pass embedded in the writing process plus the lightweight
`code-reviewer` invocation that follows this commit.

---

## 2026-04-26 — Library-UI re-conception scope: deferred

Sub-Phase E (library UI) is the largest frontend change.  Scoping
decision: ship filter chips + per-VOD quick actions + status badges
in this phase; defer drag-to-select, bulk pick, "schedule download"
to v2.1.  Rationale: shipping a coherent v2.0 UX matters more than
covering every UX-debt bullet.  Documented in ADR-0033 § Follow-ups.

---

## 2026-04-26 — Pull-mode default for new installs only

ADR-0030's migration logic decides Auto-vs-Pull based on whether the
existing `downloads` table has any `completed` or `queued` rows.
Detection is the migration's responsibility (set `distribution_mode`
during `0017_distribution_settings.sql` running, not at runtime).
Reasoning: a v1.0 user who happens to upgrade to v2.0 with an empty
queue (everything cleaned up, nothing in flight) gets pull-mode
treatment, which is fine — they can flip back via Settings if they
prefer the legacy behaviour.

---

## 2026-04-26 — Audio invariant: never re-encode

Hardest constraint in ADR-0028.  Every code path that touches the
encoder pipeline has to honour `-c:a copy`.  We add a regression test
that hashes the audio stream pre/post and asserts byte-equality.
Without this test, a bored future engineer adding "let's tune the
audio bitrate" would silently destroy the GTA-RP listening experience.
Test lives in `services/reencode.rs::tests::audio_passthrough_is_byte_exact`.

---

## 2026-04-26 — Sub-Phase A R-RC-01: 2 HIGH, 3 MEDIUM, 1 LOW resolved

`code-reviewer` ran on commit `746f140`. Findings:

- **HIGH** — ADR-0030 vs decision-log contradiction on detection
  location (migration vs `lib.rs`). Fixed by rewriting ADR-0030
  §Migration path to put detection inside the migration.
- **HIGH** — ADR-0032 §Quality factor 480p rows missing codec label.
  Fixed by adding `H.265` to both 480p rows and adding a footnote
  on hardware/software encoder factor variance.
- **MEDIUM** — ADR-0029 dangling reference "configurable per
  ADR-0028". Replaced with explicit citation of
  `0015_quality_settings.sql` plus Settings → Advanced exposure.
- **MEDIUM** — ADR-0030 detection SQL omitted `'downloading'` state.
  Expanded to `IN ('completed','queued','downloading')` to cover
  v1.0 crash-recovery rows.
- **MEDIUM** — ADR-0033 "Available" filter chip semantically inverted
  the state-machine term. Renamed to "Not downloaded" with rationale
  in the ADR.
- **LOW** — ADR-0028 didn't name the audio-passthrough test path.
  Added inline reference.

R-RC-02 (re-review on the fix-diff) follows this commit.
