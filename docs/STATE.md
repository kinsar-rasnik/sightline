# STATE — Sightline

**Letzte Aktualisierung:** 2026-04-25
**System-Ampel:** 🟢 Grün

---

## Kompass

**Projekt:** Lokale Cross-Plattform-Desktop-App (Tauri 2 + Rust + React) zur Aggregation von Twitch-GTA-RP-VODs mehrerer Streamer auf einer chronologischen Multi-Perspektiven-Timeline. Open-Source, MIT, GitHub-distributiert (unsigned binaries, Build-from-source als first-class Pfad).

**Repo:** github.com/kinsar-rasnik/sightline

**Roadmap:** 7 Phasen.
- Phase 1: Foundation, Synthetic Workforce, Skeleton ✅
- Phase 2: Twitch Ingest, Polling, Chapters ✅
- Phase 3: Download Engine, Queue, Library Layout ✅
- Phase 4: Tray Daemon, Timeline, UI Polish, Sidecar Bundling ✅
- Phase 5: Player, Watch Progress, Continue Watching ✅
- Phase 6: Multi-View Sync Engine (Housekeeping + proper) ✅
- Phase 7: Auto-Cleanup, Release Pipeline, v1.0.0 ⏳ (next, single-run-mission)

---

## Ampel-Begründung

🟢 — Repo ist sauber, alle Phasen-Tags gesetzt, CI grün, keine offenen Blocker. Phase 7 ist startklar.

---

## IST-Zustand

**Repository:**
- Working-Directory: `~/Documents/sightline`
- Branch: `main`
- Lokales und Remote-`main`: `d08b16b` (Phase 6 Squash-Merge, PR #16)
- Working Tree: clean

**Phasen-Tags vollständig:**
- `phase-1-complete` bis `phase-6-complete` ✅ alle lokal + remote
- Phase-6-Tag (`phase-6-complete`) zeigt auf `d08b16b`

**Letzter Merge auf main:** PR #16 "Phase 6: multi-view sync engine (split-view v1)" — squash-merged 2026-04-25. CI auf allen drei OS grün vor Merge. 18 Commits zu einem Squash-Commit zusammengefasst.

---

## SOLL-Zustand (Phase 7)

**Auto-Cleanup, Release Pipeline, v1.0.0.** Letzte Phase, sightline ist danach komplett.

Locked Decisions:
- OSS-Distribution via GitHub Releases, unsigned binaries (kein Code Signing, kein Notarization — bewusste Entscheidung, sightline ist Open Source und Build-from-source ist first-class)
- Tauri-Bundling für alle 3 OS via GitHub Actions Build-Matrix
- Tag-Push triggert Release-Workflow automatisch
- Update-Checker gegen GitHub Releases API (opt-in via Settings)
- Release-Notes aus Conventional Commits generiert
- Senior Engineer (Claude Code) führt finalen Merge + Tag `v1.0.0` selbst durch (full automation, CEO-Freigabe nur über PR-Approval)

Geplant:
- Auto-Cleanup-Service (Disk-Watermarks, Retention-Policies)
- GitHub Release Workflow (.github/workflows/release.yml)
- Update-Checker (Backend-Service + UI-Notification + AppSettings-Toggle)
- Release-Notes-Generator
- Install-Doku inkl. "wie öffne ich unsigned binary auf macOS/Windows"
- README v1.0-Polish
- Pickup ausgewählter Phase-6-Follow-ups (Asset-Protocol-Narrowing per ADR-0019, SyncService-Settings-Caching)
- ADRs, Tests, Subagent-Reviews
- Final-Tag: `v1.0.0` + GitHub-Release auto-published

---

## Aktive Autorun-Queue

**Aktuell laufend:** Keine. Phase 6 abgeschlossen, Phase 7 wartet auf Mission-Start.

**Pipeline:**
1. Phase 7 (Auto-Cleanup, Release Pipeline, v1.0.0) — single-run-mission
2. Post-1.0: Auto-Issue-Triage-Workflow (Claude Desktop + GitHub) — eigenes späteres Projekt, nicht Teil von Phase 7

---

## Cron / Scheduled Tasks

Keine aktiven Cron-Jobs. Workforce arbeitet ausschließlich im Push-Modus auf CTO-Anweisung.

---

## Blocker

Keine offenen Blocker. GitHub-Connectivity stabil. Repo unter `~/Documents/sightline` (nicht im Proton-Drive-Pfad).

---

## Offene Entscheidungen

Keine. Alle Phase-7-Voraussetzungen geklärt:
- Kein Code Signing nötig (OSS-Distribution)
- Senior Engineer mergt + taggt selbst (Konventionsänderung von 2026-04-25)
- v1.0.0 ist der Capstone-Tag

---

## Letzter Handoff

`docs/HANDOFF-2026-04-25.md` — dokumentiert Phase 6 Housekeeping + Phase 6 proper.

---

## Änderungshistorie

- 2026-04-25 — STATE.md initialisiert, IST/SOLL aufgenommen, drei Blocker dokumentiert.
- 2026-04-25 — Phase 6 Housekeeping (PR #15) gemerged, Phase 6 proper (PR #16) gemerged + getaggt. Alle Blocker resolved. Ampel auf 🟢. Phase 7 als nächstes vorbereitet. Konventionsänderung: Senior Engineer macht Merge + Tag (siehe .claude/CHANGELOG.md).
