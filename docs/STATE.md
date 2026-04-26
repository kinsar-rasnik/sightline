# STATE — Sightline

**Letzte Aktualisierung:** 2026-04-26
**System-Ampel:** 🟢 Grün — v2.0.1 released, alle Phase-8-ACs geschlossen

---

## Kompass

**Projekt:** Lokale Cross-Plattform-Desktop-App (Tauri 2 + Rust + React) zur Aggregation von Twitch-GTA-RP-VODs mehrerer Streamer auf einer chronologischen Multi-Perspektiven-Timeline. Open-Source, MIT, GitHub-distributiert.

**Repo:** github.com/kinsar-rasnik/sightline

**Status:** v2.0.1 live (github.com/kinsar-rasnik/sightline/releases/tag/v2.0.1). 5 Assets published. Phase-8-Scope vollständig: Backend-Substanz + UX-Surface komplett.

**Roadmap:**
- Phase 1–7 ✅
- Phase 8 (Storage-Aware Distribution) → v2.0.0 ✅ released, ⚠️ scope-incomplete
- **v2.0.1 Catch-up-Mission ✅ released** (Scope-Closure aus Phase 8 — alle 7 Items geliefert)
- Real-Use-Phase + Issue-Sammlung beim CEO
- Post-v2.0.1: Auto-Issue-Triage-Workflow (eigenständiges Folge-Projekt)

---

## Ampel-Begründung

🟢 — v2.0.1 schließt den Phase-8-Scope. Alle 7 Items (S1..S7) inkl. UX-Surfaces geliefert: Storage-Forecast-UI (AC9), Library-UI-Re-Konzeption (AC10), Pre-Fetch-Player-Wiring (AC8), Windows-CPU-Suspend (AC5), Download-Worker-Convergence (S5), WatchEvent::Completed-Wiring (S6), Stale-PID-Guard (S7). 0 P0/P1-Findings am Ende; alle R-RC-02-Re-Reviews CLEAN. Real-User-Test jetzt mit voller Storage-Sichtbarkeit möglich.

---

## IST-Zustand

**Repository:**
- Working-Directory: `~/Documents/sightline`
- Branch: `main`
- main HEAD: v2.0.1 squash-merge (siehe v2.0.1 Final-Report)
- Working Tree: clean

**Tags:**
- `phase-1-complete` bis `phase-8-complete` ✅
- `v1.0.0` → `66c8cf01`
- `v2.0.0` → `de2c8cf`
- `v2.0.1` → siehe Final-Report

**Letzte Merges:** PR v2.0.1/scope-closure (squash). Release-Workflow grün, 5 Assets published mit korrekten Sightline_2.0.1_*-Namen.

**Schema:** LATEST_SCHEMA_VERSION = 17 (unverändert von v2.0).

---

## SOLL-Zustand

**Aktuell:** Keine offene Mission. Real-Use-Phase + Issue-Sammlung beim CEO.

**Nächste Pipeline-Items:**
- v2.1: Per-Streamer Quality-Overrides, AV1-Hardware-Encoder, Mobile-Network-Detection, Per-Pane Volume-Persistenz, Library-UI-Refactor (`SuspendController` → `infra::process::suspend`), `VodArchived`-Event-Splitting für `removeVod`.
- Eigenständiges Folge-Projekt: Auto-Issue-Triage-Workflow.

---

## Aktive Autorun-Queue

**Aktuell laufend:** Keine. v2.0.1 abgeschlossen.

**Pipeline:**
1. Real-Use-Phase + Issue-Sammlung beim CEO
2. v2.1 (Datum offen)
3. Auto-Issue-Triage-Workflow (eigenes Folge-Projekt)

---

## Cron / Scheduled Tasks

Keine. Workforce arbeitet im Push-Modus.

---

## Blocker

Keine technischen Blocker. Repo clean. v2.0.1-Mission startklar.

---

## Offene Entscheidungen

Keine — alle v2.0.1-Constraints sind durch Phase-8-Final-Report und CEO-Direktive klar.

---

## Letzter Handoff

`docs/HANDOFF-2026-04-25-phase-8.md` — dokumentiert Phase 8 + v2.0.0 + Scope-Issue.
`docs/session-reports/v2.0.1-scope-closure.md` — v2.0.1-Mission Final-Report.

---

## Änderungshistorie

- 2026-04-25 — STATE.md initialisiert.
- 2026-04-25 — Phase 6 Housekeeping + Phase 6 proper merged + getaggt.
- 2026-04-25 — Phase 7 + macos-13-Hotfix merged. v1.0.0 published.
- 2026-04-25 — Phase 8 + Versions-Manifest-Hotfix merged. v2.0.0 published. Scope-Reduktion ohne Freigabe (AC9, AC10 deferred; AC8, AC5 partial). Ampel auf 🟡. v2.0.1 Catch-up-Mission als next Step. Neue Rules R-SC-01/02/03 eingeführt (Scope-Control).
- 2026-04-26 — v2.0.1 Scope-Closure released. Alle 7 Items (S1..S7) geliefert. R-SC-01 0 Reductions, R-SC-02 alle ACs ✅, R-SC-03 Manifeste-konsistent vor Tag. R-RC-01/02 5× CLEAN-Iterations. Ampel auf 🟢. Pipeline-Slot für Real-Use-Phase + v2.1 frei.
