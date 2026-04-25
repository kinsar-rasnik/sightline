# Phase 6 — Multi-View Sync Engine: Decision Log

Working log of in-flight decisions during Phase 6 proper. Captured
per the operating model in
[`docs/reference/synthetic-workforce-blueprint.md`](../reference/synthetic-workforce-blueprint.md).

ADR-worthy decisions land in `docs/adr/NNNN-*.md` (immutable once
merged); this log captures the smaller judgment calls and the
context behind them.

---

## 2026-04-25 18:35 — Sync-loop cadence: setInterval @ 250 ms, not rAF

**Kontext:** The Phase-6 prompt's draft pseudocode used
`requestAnimationFrame` for the multi-view sync loop. The CTO's
go-ahead message corrected that to a `setInterval`/`setTimeout`-based
250 ms tick before any code landed.

**Optionen:**
- A. rAF (~60 fps). High resolution, high CPU/battery cost, unnecessary
  for a 250 ms drift threshold.
- B. setInterval @ 250 ms. Matches threshold cadence exactly.
- C. setInterval @ 125 ms. Half the threshold; would catch brief drifts
  faster but doubles the CPU.

**Gewählt:** B. setInterval/setTimeout @ 250 ms.

**Begründung:**
1. Drift threshold is 250 ms — measuring more often than threshold can't
   improve correction quality; it just burns cycles.
2. rAF is correct for layout-correlated rendering, not for low-rate
   measurement loops.
3. Per ADR-0022, we keep 125 ms in reserve as a fallback if field
   testing shows visible UI stutter, before considering rAF (which
   would be a separate decision because of its CPU implications).
4. Pause-when-hidden via `document.visibilityState` covers the common
   "user tabbed away" case, where the browser already throttles
   `<video>` decoders.

**Reversibilität:** high. The cadence is a single constant in
`multiview/sync-loop.ts`. Switching to rAF or 125 ms is one config flip
+ test update; no schema or IPC contract changes.

---

## 2026-04-25 18:50 — Three ADRs, one PR

**Kontext:** ADR-0021 (layout), ADR-0022 (sync-math), ADR-0023
(transport) interlock — the layout decisions cite the math, the math
references the transport semantics, the transport relies on the
math's wall-clock anchor model. The Phase-6 prompt asks for them as
three separate ADRs.

**Optionen:**
- A. Three separate ADRs as specified.
- B. Bundle into one ADR-0021 "Multi-view v1".
- C. ADR-0021 layout, ADR-0022 sync engine (combined math + transport).

**Gewählt:** A. Three separate ADRs as specified.

**Begründung:**
1. The mission explicitly enumerates three ADRs.
2. Each one is independently revisitable: a v2 PiP layout would
   supersede ADR-0021 without touching the math; a different drift
   strategy in v3 would supersede ADR-0022 without touching layout
   or transport.
3. Cross-references inside each ADR keep the scope clear without
   forcing readers to navigate one mega-document.

**Reversibilität:** medium. Once merged, ADRs are immutable; a
revision would require new ADRs that supersede the originals.
Combining mid-stream wouldn't be a hard reversal but would mean
re-doing the cross-references.

---

## 2026-04-25 18:58 — Persist sync_sessions despite no v1 resume UI

**Kontext:** Migration 0010 will create `sync_sessions` and
`sync_session_panes` tables. v1 doesn't yet expose a "resume previous
multi-view session" affordance, so the rows are write-only from a
user-visible angle. Question: do we still create the tables, or wait
until v2 needs them?

**Optionen:**
- A. Create the tables now. v1 writes; v2 reads.
- B. Defer the migration. v1 keeps state in-memory only; v2 ships the
   migration when it ships the resume UI.

**Gewählt:** A. Create now.

**Begründung:**
1. The Phase-6 prompt is explicit on this point (D6).
2. Once installed, an upgrade path that adds resume UI in v2 doesn't
   require a forensic-style backfill — the audit data is already there
   for any installation that's run v1.
3. The migration overhead is small (two tables, no triggers).
4. The session row gives us a stable identity for IPC events
   (`sync:state_changed`, `sync:leader_changed`, etc.) without having
   to thread a UUID through the in-memory store as the only source.

**Reversibilität:** low. Once a column or table is shipped via a
migration, removing it is a forward-only operation that requires a
follow-up migration to drop. Not destructive — just irreversible
within the natural model.

---

<!-- New entries get appended below. Keep entries terse: title +
context + chosen option + reasoning + reversibility. -->
