# HANDOFF — Phase 7 + v1.0.0 Release

**Datum:** 2026-04-25
**Verantwortlich:** CTO
**Phase / Block:** Phase 7 (Auto-Cleanup, Release Pipeline, v1.0.0) inkl. macos-13 Hotfix

---

## Was wurde geliefert

**Phase 7 (PR #17, Squash `20b3102`):**
- Auto-Cleanup-Service mit Watermarks (90%/75% default), Retention-Policies, Dry-Run-Modus, Cron-Tick im Tray-Daemon
- Cleanup-Log persistiert in `cleanup_log` Table, History-View im Frontend
- `release.yml` Workflow: Tag-Push triggert 3-OS Build-Matrix → Tauri-Bundle → GitHub-Release-Asset-Upload
- Release-Notes-Generator aus Conventional Commits, sectioned Output
- Update-Checker: GitHub Releases API, opt-in via Settings, Skip-Version-Funktion, Notification-Banner
- Asset-Protocol Scope-Narrowing auf library-root (ADR-0027)
- SyncService Drift-Threshold-Caching mit 30s TTL + explizitem `invalidate_drift_cache()`
- cargo audit von informational auf blocking, mit `audit.toml` ignores für nicht-aktive Deps
- README v1.0-Polish, INSTALL.md (Unsigned-Binary-Anleitungen pro OS), Repo-CHANGELOG

**Hotfix (PR #18, Squash `66c8cf0`):**
- Drop x86_64-apple-darwin aus Release-Matrix wegen GitHub macos-13-Runner-Retirement
- Intel-Mac-User auf Build-from-source verwiesen, in INSTALL.md dokumentiert

**v1.0.0 Release:**
- 5 Assets published (macOS aarch64 64MB, Linux AppImage 178MB + Deb 117MB, Windows MSI 96MB + NSIS 68MB)
- Release-Workflow grün durchgelaufen (3 Build-Jobs + 1 Publish)
- Auto-generated Release-Notes aus Conventional Commits

---

## Was wurde NICHT geliefert / verschoben

**Aus Phase 7 verschoben (post-1.0):**
- macOS Developer ID + Windows EV Signing (funding gate, OSS-Distribution braucht es nicht zwingend)
- Intel-Mac Binary-Target re-introduzieren (paid-tier oder self-hosted runner — finanzielle Entscheidung)
- Capability-Allow-List narrowing (replace `core:default` mit explicit IDs)
- `app_settings` UPDATE auf `sqlx::query!` mit offline metadata migrieren
- Self-Update-Install (braucht Signing first)
- Distribution-Channels: Homebrew tap, winget manifest, AUR package
- 1 deferred Medium-Finding: `CheckFailed.reason`-Scrubbing (low-impact)
- 4 deferred Low-Findings (capability allow-list, schedule_due explicit test, sqlx::query! migration, softprops major-version pin)

**Aus Phase 6 verschoben (jetzt nach Phase 8 oder später):**
- Playwright + Tauri-driver E2E für Player + Multi-View
- Per-Pane Volume/Mute-Persistenz für Sync-Sessions
- Multi-View v2 (PiP, >2 Panes, Crossfader, Latency-Aware-Sync, Shareable Sync-URLs)

---

## Neue ADRs

- **ADR-0024** — Auto-Cleanup-Service
- **ADR-0025** — Release-Pipeline (mit follow-up zur Intel-Mac-Re-Introduktion)
- **ADR-0026** — Update-Checker
- **ADR-0027** — Asset-Protocol Scope-Narrowing

---

## Neue Migrationen

- **0012_cleanup_settings.sql** — Schema 11 → 12
- **0013_cleanup_log.sql** — Schema 12 → 13
- **0014_update_settings.sql** — Schema 13 → 14

`LATEST_SCHEMA_VERSION = 14`. In-Memory + On-Disk Migration-Tests grün.

---

## CI / Quality Gates

**PR #17 (Phase 7):**
- CI alle 5 Jobs grün auf allen 3 OS
- Lokale Gates: typecheck, lint, test (frontend + Rust), fmt, clippy, audit alle ✅

**PR #18 (Hotfix):**
- CI alle 5 Jobs grün auf allen 3 OS

**Release-Workflow (v1.0.0):**
- Run 24942841859: 4 Jobs grün (3 Build + 1 Publish)
- Earlier-stuck Run 24941458479 nach macos-13 Diagnose cancelled

---

## Sicherheitsbefunde

**Critical:** 0
**High:** 7 (4 code-review + 3 security) — alle inline gefixt in Commit `3b0b9e5`:
1. `cleanup::delete_candidate` library-root containment guard hinzugefügt
2. `cleanup::execute_plan` UPDATE-then-unlink ordering swap
3. `cleanup::execute_plan` AtomicBool concurrency guard
4. `UpdateBanner` useEffect dependency churn fixed
5. `updater` 64KB cap auf streaming `Response::chunk()` umgestellt
6. Asset-Protocol `$HOME/Sightline/**` static scope entfernt
7. `open_release_url` URL-Parsing tightened (https + github.com host + no creds)

**Medium:** 5 — 4 inline gefixt, 1 deferred (CheckFailed.reason scrubbing, low-impact, post-1.0)

**Low:** 4 deferred (siehe Open Follow-ups)

**cargo audit:** clean mit dokumentierten ignores (RUSTSEC-2023-0071 transitiv, sqlx-mysql nicht aktiv).

---

## Open Questions / Follow-Ups

**Phase 8 (Storage-Aware Distribution) — bereits geplant, separates HANDOFF nach Abschluss:**
1. Quality-Pipeline mit 720p30 H.265 default + Hardware-Encode-Detection + CPU-Throttle
2. Pull-Modell mit Sliding-Window N=2 default
3. Storage-Forecast-UI vor Streamer-Add
4. Library-UI-Re-Konzeption (available + downloaded)

**Post-v2.0 (eigenständige Folge-Projekte):**
1. Auto-Issue-Triage-Workflow via Claude Desktop + GitHub
2. Code-Signing + Notarization (wenn Funding/Beschluss)
3. Self-Update-Install (braucht Signing)
4. Distribution-Channels (Homebrew, winget, AUR)
5. Playwright E2E
6. Multi-View v2

---

## Synthetic Workforce Änderungen

**Neue Rule eingeführt (`.claude/rules/review-cycles.md`):**
- **R-RC-01** Mid-Phase Review-Gates: Code-Reviewer läuft nach jedem größeren Sub-Block, nicht erst am Phase-Ende.
- **R-RC-02** Re-Review nach Critical/High-Fix: Reviewer läuft zweites Mal auf Fix-Diff.
- **R-RC-03** Cross-Subagent-Awareness: Zweiter Reviewer bekommt Findings des ersten im Kontext.

Hintergrund: CEO-Direktive "Besser zweimal zu viel als einmal zu wenig" nach Beobachtung dass Phase 7 7 High-Findings erst am Ende fand. Aktiv ab Phase 8.

CHANGELOG-Eintrag: `.claude/CHANGELOG.md` 2026-04-25 "Review-Cycle-Verschärfung".

---

## Empfehlung für nächsten Step

**Phase 8 — Storage-Aware Distribution (single-run-mission, → v2.0.0).**

Versionssprung v1.0 → v2.0 weil Pull-Modell und Library-UI-Re-Konzeption Breaking-Changes im User-Verhalten darstellen (auch wenn DB-Migration backwards-compatible bleibt).

Branch: `phase-8/storage-aware`

Drei Workstreams:
- **A. Quality-Pipeline:** 720p30 H.265 default, Hardware-Encode (VideoToolbox/NVENC/AMF/QuickSync/VAAPI), CPU-Throttle (nice-Priority + adaptive Backoff bei System-Load), alle Quality-Stufen als User-Override mit Warnungen + Beispielrechnungen, Audio bleibt unverändert.
- **B. Pull-Modell:** Polling markiert VODs als "available" (Metadata-only), User pickt manuell oder via Rule "lade die nächsten N", Auto-Cleanup nach watched-completed, Pre-Fetch der nächsten N beim Watching, Default N=2.
- **C. Storage-Forecast-UI:** Vor Streamer-Add Größen-Schätzung anzeigen, Watermark-Warnungen, Library-UI zeigt available + downloaded.

Phase 8 ist umfangreicher als Phase 7. Geschätzt:
- 5-7 neue ADRs
- 4-6 neue Migrationen
- ~15-20 neue/geänderte Tauri-Commands
- Neue Subagent-Reviews mit R-RC-01/02/03 (Mid-Phase + Re-Review + Cross-Awareness)
- Längere Wallclock-Zeit als Phase 7

Voraussetzungen erfüllt: Repo clean, main aktuell auf `66c8cf0`, alle Tags gesetzt, v1.0.0 live. Phase 8 kann sofort starten.
