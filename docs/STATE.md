# STATE — Sightline

**Letzte Aktualisierung:** 2026-04-26
**System-Ampel:** 🟡 Gelb — v2.0.3 PR open, awaiting merge + Real-User-Validation

---

## Kompass

**Projekt:** Lokale Cross-Plattform-Desktop-App (Tauri 2 + Rust + React) zur Aggregation von Twitch-GTA-RP-VODs mehrerer Streamer auf einer chronologischen Multi-Perspektiven-Timeline. Open-Source, MIT, GitHub-distributiert.

**Repo:** github.com/kinsar-rasnik/sightline

**Status:** v2.0.3 hotfix-branch ready, awaiting merge. Three intertwined download-engine bugs (quality wiring AC1, concurrency cap AC2, frag-rename race AC3) and the Tech-Spec §8 file logger (AC4) addressed. v2.0.2 was effectively unusable in real-world conditions: the CEO's Mac-mini install ran into all three bugs immediately and the disk filled with retry-loop fragments.

**Roadmap:**
- Phase 1–7 ✅
- Phase 8 (Storage-Aware Distribution) ✅
- v2.0.1 Scope-Closure ✅
- v2.0.2 Critical Sidecar Hotfix + UX-Polish ✅
- v2.0.3 Critical Download-Engine Hotfix ⏳ awaiting merge + tag
- **Real-Use-Phase ⏳** — CEO validates v2.0.3 after release
- Bei Issue-Sammlung: v2.1 als Long-Run für UX/Refactor-Themen
- Post-v2.1: Auto-Issue-Triage-Workflow (eigenständiges Folge-Projekt)

---

## Ampel-Begründung

🟡 — v2.0.3 branch ready, all gates green (485 backend + 152 frontend tests, mid-phase + end-of-phase reviews PROCEED), but not yet merged + tagged + released. Switches back to 🟢 once v2.0.3 is published and CEO confirms the three bugs are gone in their actual install.

---

## IST-Zustand

**Repository:**
- Working-Directory: `~/Documents/sightline`
- Branch: `hotfix/v2.0.3-download-engine` (10 commits ahead of main)
- main HEAD: `f776ac6` (PR #23 flaky-test-Fix)
- Working Tree: clean

**Tags:**
- `phase-1-complete` bis `phase-8-complete` ✅
- `v1.0.0` → `66c8cf01`
- `v2.0.0` → `de2c8cf`
- `v2.0.1` → `ca8ff24`
- `v2.0.2` → `adb6588`
- `v2.0.3` → pending (post-merge)

**Schema:** LATEST_SCHEMA_VERSION = 18 (migration `0018_concurrency_default.sql` flattens existing `max_concurrent_downloads` values to 1..=3 range).

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

`docs/HANDOFF-2026-04-26-v2.0.3.md` — dokumentiert v2.0.3 Critical Download-Engine Hotfix (4 Sub-Phasen, AC1–AC4 + 2 Cleanup Tasks, R-RC + R-SC durchgehalten).

---

## v2.1-Backlog (gesammelt aus letzten Runs, wartet auf Long-Run-Mission)

Aus v2.0.3 hinzugekommen:
- Drop legacy `app_settings.quality_preset` + `downloads.quality_preset` columns + matching IPC field + DownloadsPage display fallback + test mocks (refactor exceeds per-task budget for the hotfix).
- SQLite table-rewrite migration to change `app_settings.max_concurrent_downloads` column DEFAULT from 2 to 1 (CEO's "Default = 1 für Neuinstallationen" aspiration is impossible without table-rewrite under SQLite semantics).
- Record actual downloaded tier in `downloads.quality_resolved` by parsing yt-dlp's `format_id`.
- Per-file 20 MB log cap (custom `MakeWriter` wrapper or hourly-rotation switch) — full Tech-Spec §8 compliance.
- Cap `ytdlp.stderr` at `warn` in the file sink so `RUST_LOG=debug` doesn't persist VOD URLs to disk (security-reviewer Medium, low impact since URLs are public).
- `UPDATE ... RETURNING` atomic claim in `pick_next_queued_excluding` — would reduce the in-memory lock to pure belt-and-suspenders.
- `path_is_on_sync_provider` Linux-Proton-Drive Detection (currently macOS-tuned).
- `tauri-plugin-log` adoption (Alternative zu manuellem `tracing-appender` setup).
- `WorkerGuard`-path Integration-Test im Logger.
- "Open log folder" button in Settings (shells out to OS file manager pointed at `default_log_dir()`).
- `DownloadsSummary.in_flight_count` IPC payload addition for the tray tooltip.

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
- 2026-04-26 — v2.0.3 Critical Download-Engine Hotfix branch ready for merge. CEO's v2.0.2 install ran into all three intertwined bugs (quality wiring AC1, concurrency cap AC2, frag-rename race AC3) plus the long-outstanding Tech-Spec §8 file logger (AC4). 4 Sub-Phasen mit R-RC-01/02 + End-of-Phase R-RC-03; ADR-0035/0036/0037; migration 0018 → schema v18. Awaiting merge + tag.
