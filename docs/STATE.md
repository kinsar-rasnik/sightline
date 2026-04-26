# STATE — Sightline

**Letzte Aktualisierung:** 2026-04-26
**System-Ampel:** 🟢 Grün — v2.0.2 released, kritische Bugs behoben, Real-Use offen

---

## Kompass

**Projekt:** Lokale Cross-Plattform-Desktop-App (Tauri 2 + Rust + React) zur Aggregation von Twitch-GTA-RP-VODs mehrerer Streamer auf einer chronologischen Multi-Perspektiven-Timeline. Open-Source, MIT, GitHub-distributiert.

**Repo:** github.com/kinsar-rasnik/sightline

**Status:** v2.0.2 live (github.com/kinsar-rasnik/sightline/releases/tag/v2.0.2). Sidecar-Resolution-Bug aus v2.0.1 behoben — Sightline funktioniert jetzt out-of-the-box auf macOS-Bundles. Co-Streams "Download only"-Action verfügbar. INSTALL.md xattr-Doku klar.

**Roadmap:**
- Phase 1–7 ✅
- Phase 8 (Storage-Aware Distribution) ✅
- v2.0.1 Scope-Closure ✅
- v2.0.2 Critical Sidecar Hotfix + UX-Polish ✅
- **Real-Use-Phase ⏳ aktiv** — CEO testet im Alltag
- Bei Issue-Sammlung: v2.1 als Long-Run für UX/Refactor-Themen
- Post-v2.1: Auto-Issue-Triage-Workflow (eigenständiges Folge-Projekt)

---

## Ampel-Begründung

🟢 — v2.0.2 live mit Real-User-Bestätigung. Post-Merge-CI-Incident auf main wurde sofort selbstständig gefixt (PR #23, flaky PID-Test entfernt). Main CI grün. Repo clean.

---

## IST-Zustand

**Repository:**
- Working-Directory: `~/Documents/sightline`
- Branch: `main`
- main HEAD: `f776ac6` (PR #23 flaky-test-Fix)
- Working Tree: clean

**Tags:**
- `phase-1-complete` bis `phase-8-complete` ✅
- `v1.0.0` → `66c8cf01`
- `v2.0.0` → `de2c8cf`
- `v2.0.1` → `ca8ff24`
- `v2.0.2` → `adb6588`

**Schema:** LATEST_SCHEMA_VERSION = 17 (unverändert seit Phase 8).

---

## SOLL-Zustand (Real-Use-Phase)

CEO nutzt Sightline im Alltag, sammelt Issues via GitHub Issues. Trigger für nächsten Engineering-Run:

- **Kritische Bugs** (Crash, Datenverlust, Sync-Failure) → Sofort-Hotfix v2.0.x
- **Größere UX-Themen** gesammelt → v2.1-Mission als Long-Run
- **Auto-Issue-Triage-Setup** als eigenständiges Folge-Projekt (Claude Desktop + GitHub)

---

## Aktive Autorun-Queue

Keine.

---

## Cron / Scheduled Tasks

Keine.

---

## Blocker

Keine.

---

## Offene Entscheidungen

Keine. Real-Use-Phase ist passive Phase ohne Workforce-Aktivität.

---

## Letzter Handoff

`docs/HANDOFF-2026-04-26-v2.0.2.md` — dokumentiert v2.0.2 Hotfix + Post-Merge-Incident.

---

## v2.1-Backlog (gesammelt aus letzten Runs, wartet auf Long-Run-Mission)

Aus v2.0.2 hinzugekommen:
- `tauri-plugin-shell::Command::new_sidecar` Adoption (sauberer als manueller Resolver)
- Release-Workflow Post-Bundle-Inspection-Step (hätte Sidecar-Bug vor Release gefangen)
- `SHA256SUMS` Asset auf Releases publishen

Aus v2.0.1 inherited:
- SuspendController in `infra::process::suspend` refactoren
- `VodDeleted` Event-Variant (statt VodArchived-Reuse)
- Windows CI Runner für SuspendThread-Coverage
- StorageOutlook Component-Level Vitest Coverage
- Live-Update Forecast on Settings-Change

Aus Phase 8 inherited:
- AV1-Hardware-Encoder
- Per-Streamer Quality-Overrides
- Mobile-Network-Detection
- Per-Pane Volume/Mute-Persistenz für Sync-Sessions
- Playwright + Tauri-driver E2E

Post-v2.0 (große Themen):
- Code-Signing + Notarization (CEO-Entscheidung: nein, OSS bleibt unsigned)
- Self-Update-Install (nicht ohne Signing)
- Distribution-Channels (Homebrew/winget/AUR)

---

## Änderungshistorie

- 2026-04-25 — STATE.md initialisiert.
- 2026-04-25 — Phase 6 Housekeeping + Phase 6 proper merged + getaggt.
- 2026-04-25 — Phase 7 + macos-13-Hotfix merged. v1.0.0 published.
- 2026-04-25 — Phase 8 + Versions-Manifest-Hotfix merged. v2.0.0 published. R-SC-01/02/03 eingeführt.
- 2026-04-26 — v2.0.1 Scope-Closure released. R-RC + R-SC im Kombi-Einsatz bewährt.
- 2026-04-26 — v2.0.2 Critical Sidecar-Resolution-Hotfix released. Real-User-Bug-Report von CEO triggerte Mission. PR #23 Post-Merge-Hotfix für flaky PID-Test. Real-Use-Phase aktiviert.
