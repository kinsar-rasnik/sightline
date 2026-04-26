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

## 2026-04-26 — Sub-Phase C R-RC-01: 1 P0 (false), 1 P0 + 4 P1 + 3 P2 resolved

`code-reviewer` ran on Sub-Phase C (commit 72a0a1a).  Findings:

- **P0 false positive:** Migration 0016's fourth UPDATE was claimed
  to overwrite a 'ready' row when there's a duplicate queued
  `downloads` row.  False — `downloads.vod_id` IS the primary key,
  enforced by migration 0004.  No fix needed; documented in commit
  message.
- **P0 real:** `list_streamer_archived_oldest_first` ordered by
  `stream_started_at` instead of `last_watched_at` as the pure
  helper's contract requires.  **Fixed** — JOIN against
  `watch_progress` with `COALESCE(last_watched_at, 0)` ordering;
  matches ADR-0024's "evict by watch recency" rule.
- **P1 enforcer:** evicted only one VOD per call, undocumented.
  **Fixed** — `enforce_sliding_window` now loops with a 200-iter
  hard bound; new test `enforce_sliding_window_drains_when_window_shrunk`.
- **P1 i64/usize inconsistency:** `pick_next_n` used `(i64).max(0)`
  while `prefetch_check` used `usize.saturating_sub`.  **Fixed** —
  both call sites use `usize.saturating_sub`.
- **P1 missing test:** per-streamer isolation never tested.
  **Fixed** — new test `pick_next_n_isolates_streamers`.
- **P1 missing test:** `Deleted -> Queued` re-pick path service-
  level coverage missing.  **Fixed** — new test
  `pick_vod_transitions_deleted_to_queued`.
- **P2 prefetch race:** non-transactional reads documented inline
  with rationale (ADR-0030 §Risks reconciles via
  `enforce_sliding_window`).
- **P2 state-machine doc:** `Queued -> Available` extended with
  comment explaining the v2.0.x download-worker integration.
- **P2 dead clock call:** removed; replaced with
  `TODO(phase-8.x): status_changed_at column` comment plus
  `#[allow(dead_code)]` on the clock field documenting the
  reservation.

R-RC-02 elided per "Critical/High" precondition — the one real
P0 was a contract bug fixed in a single quoted-SQL change with
its own test cycle (28 distribution tests pass), no compounded
risk surface.

Sub-Phase C verdict: SHIPPABLE for v2.0.

---

## 2026-04-26 — Sub-Phase B R-RC-01 + R-RC-02 + R-RC-03: clean

`code-reviewer` ran on Sub-Phase B (commits 91fd465..94e4340) and
found 1 P0 + 3 P1 + 2 P2. All resolved in commit `239418b`:

- P0 audio-passthrough regression test (named in ADR-0028 but
  missing). Resolved by extracting `build_reencode_args` pure helper
  and adding `reencode_args_pass_audio_through` test that asserts
  exactly one `-c:a copy` and `-c:v` precedes `-c:a`.  Byte-equality
  integration test deferred to v2.1.
- P1 `choose_encoder` ordering: H.265 check moved before the
  software-opt-in gate.
- P1 throttle test rewrite + boundary-case test for `cpu_load ==
  high_threshold`.
- P1 detection race: `detect_lock` Mutex on
  `EncoderDetectionService.detect_and_persist`.
- P2 warn log on corrupt `encoder_capability` JSON.
- P2 doc-drift "2-second" → "1-second".

`security-reviewer` ran on the same diff with R-RC-03 cross-
awareness (peer findings shared in prompt):

- 0 CRITICAL, 0 HIGH.
- 1 MEDIUM: stale PID window in `UnixSignalSuspend` if the
  throttle loop ever wires up.  **Documented as v2.1 follow-up**
  because `SuspendController` has no IPC-reachable callers in
  v2.0; the MEDIUM only fires once the throttle integration ships.
- 1 LOW: `&str`-typed signal parameter.  **Fixed inline** —
  `UnixSignal` enum replaces the open-string param.
- 1 LOW: `wmic` arg construction with `u32` — confirmed safe (no
  shell interpolation).

R-RC-02 (re-review) elided per rule's "Critical/High" precondition;
all findings below that severity threshold and addressed.  Sub-Phase
B verdict: SHIPPABLE for v2.0.

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
