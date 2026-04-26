# STATE — Sightline

**Letzte Aktualisierung:** 2026-04-25
**System-Ampel:** 🟡 Gelb — v2.0.0 released, aber 4 ACs offen

---

## Kompass

**Projekt:** Lokale Cross-Plattform-Desktop-App (Tauri 2 + Rust + React) zur Aggregation von Twitch-GTA-RP-VODs mehrerer Streamer auf einer chronologischen Multi-Perspektiven-Timeline. Open-Source, MIT, GitHub-distributiert.

**Repo:** github.com/kinsar-rasnik/sightline

**Status:** v2.0.0 live (github.com/kinsar-rasnik/sightline/releases/tag/v2.0.0). 5 Assets published. Backend-Substanz Storage-Aware vollständig. Aber: 2 spec'd UI-Workstreams + 2 partial ACs deferred ohne Freigabe — wird in v2.0.1 nachgezogen.

**Roadmap:**
- Phase 1–7 ✅
- Phase 8 (Storage-Aware Distribution) → v2.0.0 ✅ released, ⚠️ scope-incomplete
- **v2.0.1 Catch-up-Mission ⏳ next** (Scope-Closure aus Phase 8)
- Post-v2.0.1: Auto-Issue-Triage-Workflow (eigenständiges Folge-Projekt)

---

## Ampel-Begründung

🟡 — v2.0.0 ist released und funktional, aber inkomplett gegenüber Spec. Storage-Forecast-UI (AC9), Library-UI-Re-Konzeption (AC10), Pre-Fetch-Player-Wiring (AC8 partial), Windows-CPU-Suspend (AC5 partial) fehlen. Real-User-Test ohne Forecast-UI bedeutet "blind in Storage-Probleme reinfliegen". v2.0.1 als Sofort-Catch-up notwendig.

---

## IST-Zustand

**Repository:**
- Working-Directory: `~/Documents/sightline`
- Branch: `main`
- main HEAD: `de2c8cf` (Phase 8 Hotfix Versions-Bump, PR #20)
- Working Tree: clean

**Tags:**
- `phase-1-complete` bis `phase-8-complete` ✅
- `v1.0.0` → `66c8cf01`
- `v2.0.0` → `de2c8cf`

**Letzte Merges:** PR #19 (Phase 8) + PR #20 (Versions-Manifest-Hotfix). Release-Workflow grün, 5 Assets published mit korrekten Sightline_2.0.0_*-Namen.

**Schema:** LATEST_SCHEMA_VERSION = 17.

---

## SOLL-Zustand (v2.0.1)

**Catch-up-Mission: Scope-Closure aus Phase 8.**

Vier explizite Items:
1. **AC9 — Storage-Forecast-UI** (komplett deferred): Forecast-Service (Math) ist im Backend gelandet, fehlt: Tauri-Command + Frontend Streamer-Add-Dialog mit Forecast-Box + Settings-Page mit globalem Disk-Forecast + Watermark-Risk-Indicator.
2. **AC10 — Library-UI-Re-Konzeption** (komplett deferred): unified List "available + downloaded", visuelle Differenzierung, Filter-Chips (All/Available/Downloaded/Watched), Per-VOD Quick-Actions.
3. **AC8 — Pre-Fetch-Player-Wiring** (partial): Pre-Fetch-Logic im Backend, fehlt: Player-Hook der bei Watch-Progress den Pre-Fetch triggert + IPC-Anbindung.
4. **AC5 — Windows-CPU-Suspend** (partial): nice-Priority funktioniert auf allen 3 OS, suspend ist Unix-only via SIGSTOP. Fehlt: Windows SuspendThread-Implementierung.

**Zusätzlich (aus Phase 8 als deferred markiert):**
5. Download-Worker beobachtet `vods.status = 'queued'` für State-Convergence
6. WatchEvent::Completed → distribution.on_watched_completed Wiring
7. Stale-PID-Guard für SuspendController (Medium-Finding aus Phase 8)

**Versionssprung:** v2.0.0 → v2.0.1 (Patch-Release, kein Breaking-Change).

**Locked Decisions für v2.0.1:**
- Scope ist genau diese 7 Items, nichts Neues
- R-SC-01/02/03 ab dieser Mission aktiv
- R-RC-01/02/03 weiterhin aktiv
- Keine neuen ADRs nötig (ADR-0028..0033 decken die UI bereits)
- Migrations: vermutlich keine neuen, evtl. eine kleine für Stale-PID-Guard

---

## Aktive Autorun-Queue

**Aktuell laufend:** Keine. Phase 8 abgeschlossen, v2.0.1-Mission wartet auf Start.

**Pipeline:**
1. v2.0.1 Catch-up-Mission (Scope-Closure)
2. Real-Use-Phase + Issue-Sammlung beim CEO
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

`docs/HANDOFF-2026-04-25-phase-7.md` — dokumentiert Phase 7 + v1.0.0.
**Pflicht:** `docs/HANDOFF-2026-04-25-phase-8.md` schreiben (Phase 8 + v2.0.0 + Scope-Issue), bevor v2.0.1-Mission startet.

---

## Änderungshistorie

- 2026-04-25 — STATE.md initialisiert.
- 2026-04-25 — Phase 6 Housekeeping + Phase 6 proper merged + getaggt.
- 2026-04-25 — Phase 7 + macos-13-Hotfix merged. v1.0.0 published.
- 2026-04-25 — Phase 8 + Versions-Manifest-Hotfix merged. v2.0.0 published. Scope-Reduktion ohne Freigabe (AC9, AC10 deferred; AC8, AC5 partial). Ampel auf 🟡. v2.0.1 Catch-up-Mission als next Step. Neue Rules R-SC-01/02/03 eingeführt (Scope-Control).
