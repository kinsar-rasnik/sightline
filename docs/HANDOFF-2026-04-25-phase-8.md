# HANDOFF — Phase 8 + v2.0.0 Release

**Datum:** 2026-04-25
**Verantwortlich:** CTO
**Phase / Block:** Phase 8 (Storage-Aware Distribution) inkl. Versions-Manifest-Hotfix

---

## Was wurde geliefert

**Phase 8 (PR #19, Squash `3caab1c`) + Hotfix (PR #20, Squash `de2c8cf`):**

**Backend (komplett):**
- Quality-Pipeline: 720p30 H.265 default, Hardware-Encode-Detection (VideoToolbox/NVENC/AMF/QuickSync/VAAPI), Software-Encode als Opt-in mit Warnung
- Background-Friendly Re-Encode: nice-Priority auf allen 3 OS, adaptive CPU-Throttling mit Suspend/Resume (Unix-only via SIGSTOP, Windows pending)
- Pull-Modell mit State-Machine: available → queued → downloading → ready → archived → deleted
- Sliding-Window-Enforce mit watch-recency-basierter Eviction
- Pre-Fetch-Logic mit one-step lookahead
- Storage-Forecast-Math im Domain-Layer (avg-VOD × frequency × quality-factor × window-size)
- Audio-Passthrough strukturell garantiert (`-c:a copy`, hash-equal-Test)

**Datenbank:**
- Migration 0015: quality_settings (6 Spalten in app_settings)
- Migration 0016: vod_status_machine (Backfill aus existierenden downloads + watch_progress)
- Migration 0017: distribution_settings (mode, sliding_window_size, prefetch_enabled)
- Backwards-Compat: v1.0-Bestand → mode='auto', neue Installs → mode='pull'

**ADRs:**
- ADR-0028 Quality Pipeline & Default Quality
- ADR-0029 Background-Friendly Re-Encode
- ADR-0030 Pull-on-Demand Distribution + Sliding Window
- ADR-0031 Pre-Fetch-Strategie (one-step lookahead)
- ADR-0032 Storage-Forecast-Heuristik
- ADR-0033 Library-UI-Re-Konzeption

**Tauri-Commands neu (8):** getEncoderCapability, redetectEncoders, setVideoQualityProfile, pickVod, pickNextN, unpickVod, setDistributionMode, setSlidingWindowSize.

**Events neu (4):** distribution:vod_picked, distribution:vod_archived, distribution:prefetch_triggered, distribution:window_enforced.

**v2.0.0 Release:**
- 5 Assets published (macOS aarch64 / Linux AppImage + Deb / Windows NSIS + MSI)
- Auto-generated Release-Notes mit BREAKING-Note prominent
- Release-Workflow-Run 24945162664 grün

---

## Was wurde NICHT geliefert / Scope-Issue

**Wichtig:** Phase 8 hat Scope-Reduktion ohne CTO-Freigabe vorgenommen. Das hat zur Einführung von R-SC-01/02/03 geführt (siehe `.claude/rules/scope-control.md`).

**Komplett deferred (wird in v2.0.1 nachgezogen):**

- **AC9 — Storage-Forecast-UI:** Math im Backend, aber kein Tauri-Command, kein Streamer-Add-Dialog mit Forecast-Box, keine Settings-Page mit globalem Disk-Forecast, kein Watermark-Risk-Indicator. Direkter Effekt: User addet Streamer "blind", sieht Storage-Risiko erst wenn Disk voll.
- **AC10 — Library-UI-Re-Konzeption:** Aktuelle Library zeigt weiterhin nur downloaded VODs. Available-VODs sind via Pull-Modell-Backend pickbar, aber UI-Surface fehlt. Pull-Modell ist UX-mäßig unvollständig.

**Partial:**

- **AC8 — Pre-Fetch:** Logic implementiert + getestet, aber kein Player-Hook der bei Watch-Progress den Pre-Fetch triggert. Pre-Fetch passiert nicht automatisch.
- **AC5 — CPU-Throttle:** Priority funktioniert auf allen 3 OS, Suspend nur Unix. Windows ffmpeg läuft mit niedriger Priority, wird aber bei System-Load nicht pausiert.

**Zusätzlich aus Phase-8-Final-Report deferred:**

- Download-Worker beobachtet `vods.status = 'queued'` (State-Convergence-Loose-End)
- WatchEvent::Completed → distribution.on_watched_completed-Wiring
- Stale-PID-Guard für SuspendController (Medium-Finding)

---

## Versions-Manifest-Hotfix

Phase-8-PR #19 wurde gemerged, während Cargo.toml / package.json / tauri.conf.json noch auf 1.0.0 standen. v2.0.0-Tag-Push triggerte Release-Workflow, Assets wurden als `Sightline_1.0.0_*` published. Hotfix:

1. Misnamed Release + Tag gelöscht
2. PR #20 mit Versions-Bump auf 2.0.0 (Cargo.toml, Cargo.lock, package.json, tauri.conf.json)
3. Re-tagged + Re-Release-Workflow
4. Final Assets korrekt `Sightline_2.0.0_*`

R-SC-03 (Versions-Manifest-Konsistenz vor Release-Tag) wurde als Folge eingeführt, um das künftig zu verhindern.

---

## CI / Quality Gates

**PR #19 (Phase 8) + PR #20 (Hotfix):**
- CI alle 5 Jobs grün auf allen 3 OS
- Lokale Gates: typecheck, lint, test, fmt, clippy, audit alle ✅

**Release-Workflow:**
- Run 24944729873: success, aber Assets misnamed (1.0.0)
- Run 24945162664: success, Assets korrekt (2.0.0) — DAS ist der live Release

---

## Sicherheitsbefunde

**Critical:** 0
**High (resolved before merge):** 4
- 2 aus Sub-Phase A (ADR-stale-references)
- 1 P0 aus Sub-Phase B (Audio-Test)
- 1 P0 aus Sub-Phase C (Sort-Key-Bug — gefunden durch R-RC-01 Mid-Phase-Review!)

**Medium deferred zu v2.1:**
- Stale-PID-Guard für SuspendController (low-impact, IPC-reachable callers existieren in v2.0 nicht)

**Low (4):** in Open-Follow-ups dokumentiert.

---

## Review-Cycle-Statistik (R-RC erstmalig angewendet)

| Sub-Phase | R-RC-01 reviews | R-RC-02 re-reviews | R-RC-03 cross-aware |
|---|---|---|---|
| A — ADRs | 1 | 1 iteration | n/a |
| B — Quality pipeline | 1 | elided (low) | yes (security with peer findings) |
| C — Pull model | 1 | elided (single P0 fix) | n/a |
| End-of-phase | 1 | n/a | covered by B |

**Effektivität:** "Phase 7 found 7 High end-of-phase only; Phase 8 caught and fixed equivalent issues per sub-block." R-RC-01 hat sich bewährt. Bleibt als Standard für künftige Phasen.

---

## Open Questions / Follow-Ups

**v2.0.1 Catch-up-Mission (sofort, single-run):**
1. AC9 — Storage-Forecast-UI komplett
2. AC10 — Library-UI-Re-Konzeption komplett
3. AC8 — Pre-Fetch-Player-Wiring + IPC
4. AC5 — Windows-CPU-Suspend (SuspendThread)
5. Download-Worker State-Convergence
6. WatchEvent::Completed Wiring
7. Stale-PID-Guard

**v2.1 (post-real-use):**
- AV1-Hardware-Encoder
- Per-Streamer Quality-Overrides
- Mobile-Network-Detection für Pause-bei-Hotspot
- Per-Pane Volume/Mute-Persistenz für Sync-Sessions
- Playwright + Tauri-driver E2E

**Post-v2.0 (bereits aus Phase 7 inherited):**
- Code-Signing + Notarization (funding gate)
- Self-Update-Install
- Distribution-Channels (Homebrew/winget/AUR)
- Intel-Mac-Binary-Target Re-Introduktion (paid runner)

**Eigenständiges Folge-Projekt:**
- Auto-Issue-Triage-Workflow via Claude Desktop + GitHub

---

## Synthetic Workforce Änderungen

**Neue Rule eingeführt (`.claude/rules/scope-control.md`):**
- **R-SC-01** Scope-Reduction-Approval: Defer/Skip von Locked Decisions oder spec'd ACs braucht CTO-Freigabe.
- **R-SC-02** Acceptance-Criteria-Vollständigkeits-Check vor End-of-Phase-Review.
- **R-SC-03** Versions-Manifest-Konsistenz vor Release-Tag.

Hintergrund: Phase 8 hat AC9 + AC10 ohne Freigabe deferred; v2.0.0 wurde mit 1.0.0-Manifests getaggt. Beide Probleme adressiert.

CHANGELOG-Eintrag: `.claude/CHANGELOG.md` 2026-04-25 "Scope-Control-Rules".

---

## Empfehlung für nächsten Step

**v2.0.1 Catch-up-Mission als single-run-mission, deutlich kleiner als Phase 8.**

Branch: `v2.0.1/scope-closure`

Scope ist genau die 7 Items aus "Open Follow-ups → v2.0.1 Catch-up-Mission" — nichts Neues. Kein neuer ADR-Bedarf (ADR-0028..0033 decken die UI bereits). Migrationen vermutlich keine, evtl. eine für Stale-PID-Guard.

R-SC-01/02/03 erstmals aktiv. R-RC-01/02/03 weiterhin aktiv. Erwartete Wallclock-Zeit: deutlich kürzer als Phase 8, weil Backend-Substanz schon da ist.

Voraussetzungen erfüllt: Repo clean, main aktuell auf `de2c8cf`, alle Tags + v2.0.0 live.
