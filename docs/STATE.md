# STATE — Sightline

**Letzte Aktualisierung:** 2026-04-26
**System-Ampel:** 🟢 Grün — v2.0.2 released, Sidecar-Resolution-Hotfix aus Real-Use-Phase

---

## Kompass

**Projekt:** Lokale Cross-Plattform-Desktop-App (Tauri 2 + Rust + React) zur Aggregation von Twitch-GTA-RP-VODs mehrerer Streamer auf einer chronologischen Multi-Perspektiven-Timeline. Open-Source, MIT, GitHub-distributiert.

**Repo:** github.com/kinsar-rasnik/sightline

**Status:** v2.0.1 live (github.com/kinsar-rasnik/sightline/releases/tag/v2.0.1). 5 Assets korrekt benamt. Storage-Aware-Distribution end-to-end vollständig — Backend + UI. Sightline ist auf v2.0-Spec geschlossen.

**Roadmap:**
- Phase 1–7 ✅
- Phase 8 (Storage-Aware Distribution) ✅
- v2.0.1 Scope-Closure ✅
- **Real-Use-Phase ⏳ next** — CEO testet im Alltag, sammelt Issues
- Post-Real-Use: Auto-Issue-Triage-Workflow (eigenständiges Folge-Projekt)

---

## Ampel-Begründung

🟢 — Sightline v2.0.1 ist released, alle Specs erfüllt, alle Tests grün, alle Assets korrekt. Workforce-Rules R-RC-* und R-SC-* haben sich im ersten kombinierten Einsatz bewährt (0 Scope-Reduktionen, 18 Findings sauber inline gefangen). Bereit für Real-Use.

---

## IST-Zustand

**Repository:**
- Working-Directory: `~/Documents/sightline`
- Branch: `main`
- main HEAD: `ca8ff24` (v2.0.1 Scope-Closure, PR #21)
- Working Tree: clean

**Tags:**
- `phase-1-complete` bis `phase-8-complete` ✅
- `v1.0.0` → `66c8cf01`
- `v2.0.0` → `de2c8cf`
- `v2.0.1` → `ca8ff24`

**Tests:** 471 Rust + 152 Frontend = 623 grün.

**Schema:** LATEST_SCHEMA_VERSION = 17 (keine neuen Migrationen in v2.0.1).

---

## SOLL-Zustand (Real-Use-Phase)

**Kein Engineering-Run aktiv.** CEO nutzt Sightline im Alltag, sammelt Issues via:
- Eigene Beobachtung beim VOD-Watching
- Storage-Forecast-Realität-Check (skaliert die Math?)
- Multi-View-Sync im Real-World-Use
- Quality-Pipeline-Verhalten beim Background-Download während Gaming
- Library-UI-UX im echten Browse-Verhalten

**Issue-Erfassung:** GitHub Issues unter github.com/kinsar-rasnik/sightline/issues — manuell vom CEO oder via Auto-Triage-Workflow (separates Projekt).

**Trigger für nächsten Engineering-Run:**
- Kritische Bugs (Crash, Datenverlust, Sync-Failure) → Hotfix-Mission v2.0.2
- Sammlung größerer UX-Themen → v2.1-Mission
- Klassische Feature-Requests → v2.x-Backlog

---

## Aktive Autorun-Queue

Keine. CEO entscheidet wann Real-Use-Phase endet und nächster Run startet.

---

## Cron / Scheduled Tasks

Keine.

---

## Blocker

Keine.

---

## Offene Entscheidungen

Keine. v2.0.1 ist sauber, Real-Use-Phase ist passive Phase ohne Workforce-Aktivität.

---

## Letzter Handoff

`docs/HANDOFF-2026-04-26-v2.0.1.md` — dokumentiert v2.0.1 Scope-Closure inkl. Workforce-Rule-Bewährung.

---

## Änderungshistorie

- 2026-04-25 — STATE.md initialisiert.
- 2026-04-25 — Phase 6 Housekeeping + Phase 6 proper merged + getaggt.
- 2026-04-25 — Phase 7 + macos-13-Hotfix merged. v1.0.0 published.
- 2026-04-25 — Phase 8 + Versions-Manifest-Hotfix merged. v2.0.0 published. Scope-Reduktion ohne Freigabe (AC9, AC10 deferred). Ampel auf 🟡. R-SC-01/02/03 eingeführt.
- 2026-04-26 — v2.0.1 Scope-Closure released. Alle 7 Items geliefert, 0 R-SC-Reduktionen, 18 Findings inline gefangen über R-RC-Mid-Phase-Reviews. Ampel auf 🟢. Real-Use-Phase startet.
- 2026-04-26 — v2.0.2 Hotfix released. Erster Issue der Real-Use-Phase: macOS .app sidecar-resolution gebrochen (Tauri 1 → 2 Layout-Wechsel ungemerkt). resolve_sidecar refactored, ADR-0034 dokumentiert, INSTALL.md macOS-Section überarbeitet (xattr-Pflicht). 8 neue Bundle-Layout-Simulation-Tests in CI. R-SC + R-RC sauber durchlaufen — eine R-RC-02-Iteration für 1 P0 + 3 P1 + 1 P2.
