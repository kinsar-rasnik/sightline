# STATE — Sightline

**Letzte Aktualisierung:** 2026-04-25
**System-Ampel:** 🟡 Gelb

---

## Kompass

**Projekt:** Lokale Cross-Plattform-Desktop-App (Tauri 2 + Rust + React) zur Aggregation von Twitch-GTA-RP-VODs mehrerer Streamer auf einer chronologischen Multi-Perspektiven-Timeline.

**Repo:** github.com/kinsar-rasnik/sightline (MIT, OSS)

**Roadmap:** 7 Phasen.
- Phase 1: Foundation, Synthetic Workforce, Skeleton ✅
- Phase 2: Twitch Ingest, Polling, Chapters ✅
- Phase 3: Download Engine, Queue, Library Layout ✅
- Phase 4: Tray Daemon, Timeline, UI Polish, Sidecar Bundling ✅
- Phase 5: Player, Watch Progress, Continue Watching ✅ (Code gemerged, Tag pending)
- Phase 6: Multi-View Sync Engine ⏳ (in Housekeeping)
- Phase 7: Auto-Cleanup, Release Pipeline, Code Signing ⏳

---

## Ampel-Begründung

🟡 wegen drei aktiven Blockern unten. Code-Qualität selbst ist 🟢.

---

## IST-Zustand

**Repository:**
- Lokales Working-Directory: `~/Documents/sightline` (frisch via rsync aus Proton-Drive-Ordner gezogen, 2026-04-25)
- Branch: `phase-6/housekeeping` (lokal aus `phase-5/player-and-watch-progress` gebrancht, das inhaltlich PR #14 entspricht)
- Lokales `main` ist auf 6d9d2b7 (Phase 4) — stale gegenüber Remote
- Remote `main` ist auf a9d781af (Phase 5 Squash-Merge) — von lokal aus aktuell unerreichbar wegen GitHub-Connectivity

**Tags:**
- phase-1-complete bis phase-4-complete: ✅ lokal + remote
- phase-5-complete: ❌ noch nicht gesetzt (wartet auf erreichbares Remote + Sync von lokalem main)

**Letzter Merge auf Remote-main:** PR #14 "Phase 5: player, watch progress, continue watching + housekeeping" — gemerged via `gh pr merge --squash --delete-branch` am 2026-04-24, CI auf allen drei OS grün vor Merge.

---

## SOLL-Zustand (Phase 6)

**Multi-View Sync Engine.** Split-View v1 mit 2 Panes, 50/50-Layout. Wall-Clock-basierte Synchronisation, 250ms Drift-Toleranz, Group-wide Transport (kein Per-Pane). Leader-Election, configurable via UI.

ADRs geplant: 0021 (Split-View v1 / PiP v2), 0022 (Sync-Math + Drift), 0023 (Group-wide Transport).
Migrationen geplant: 0009 (AppSettings completion_threshold + watch-progress thresholds — Housekeeping), 0010 (sync_sessions — Phase 6 proper).
Neue Tauri-Commands: 8 (cmd_open_sync_group, cmd_close_sync_group, cmd_get_sync_group, cmd_set_sync_leader, cmd_sync_seek, cmd_sync_play, cmd_sync_pause, cmd_sync_set_speed, cmd_get_overlap).
Neue Events: 5 (sync:state_changed, sync:drift_corrected, sync:leader_changed, sync:member_out_of_range, sync:group_closed).
Neue Route: `/multiview`.

---

## Aktive Autorun-Queue

**Aktuell laufend:** Claude Code arbeitet Phase 6 Housekeeping ab. Repo wurde via `git restore --worktree --source=:0 .` aus Index rekonstruiert (Working Tree war von rsync aus Proton-Drive auf Pre-Phase-5-Stand zurückgefallen, Index hatte aber die Wahrheit).

**Pipeline:**
1. Phase 6 Housekeeping (laufend)
2. Phase 6 proper (Multi-View Sync Engine)
3. Phase 7 (Auto-Cleanup, Release, Signing, Notarization)

---

## Cron / Scheduled Tasks

Keine aktiven Cron-Jobs. Workforce arbeitet ausschließlich im Push-Modus auf CTO-Anweisung.

---

## Blocker

**1. GitHub-Connectivity intermittierend down auf CEO-Mac.**
SSH und HTTPS hängen seit Phase-5-Merge mehrfach beim Fetch/Pull. Ursache nicht eindeutig identifiziert (Proton-Drive war Verursacher für Git-Hänger, GitHub-Side war zwischenzeitlich auch betroffen, evtl. lokales SSH-Stack-Problem). Workflow heute: lokale Commits, Push wenn Connectivity zurück.

**2. Proton-Drive blockiert Git-Operationen im synchronisierten Ordner.**
Resolved durch Repo-Umzug nach `~/Documents/sightline`. Original-Ordner unter `~/Library/CloudStorage/ProtonDrive-...folder/sightline` ist Read-Reference, nicht Working Copy.

**3. phase-5-complete Tag nicht gesetzt.**
Wartet auf Resolution von Blocker 1. Lokales main muss zuerst auf Remote-Stand gefastforwardet werden.

---

## Offene Entscheidungen

Keine. Alle Phase-6-Entscheidungen aus dem Phase-6-Prompt sind getroffen (Split-View v1 mit 2 Panes, eine Migration für Housekeeping, getrennte Migration für sync_sessions in Phase 6 proper, AppSettings als Settings-Träger).

---

## Letzter Handoff

Noch keiner geschrieben — Handoff-Disziplin beginnt mit Abschluss Phase 6 Housekeeping (erstes HANDOFF-2026-XX-XX.md).

---

## Änderungshistorie

- 2026-04-25 — STATE.md initialisiert, IST/SOLL aufgenommen, drei Blocker dokumentiert.
