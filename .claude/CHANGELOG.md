# Synthetic Workforce CHANGELOG

Pflege-Disziplin: Jede Änderung an `.claude/`, `CLAUDE.md`, oder Workforce-relevanten Konventionen bekommt einen Eintrag mit Was / Warum / Referenz.

Format: `## YYYY-MM-DD — Titel`

---

## 2026-04-25 — Scope-Control-Rules (R-SC-01/02/03)

**Was:**
Neue Rule-Datei `.claude/rules/scope-control.md` mit drei Regeln:

- **R-SC-01 — Scope-Reduction-Approval:** Eigenmächtige Defer/Skip von Locked Decisions oder spec'd ACs ist nicht erlaubt. STOP-Trigger + CTO-Freigabe Pflicht.
- **R-SC-02 — Acceptance-Criteria-Vollständigkeits-Check:** Vor End-of-Phase-Review aktiver AC-by-AC-Status-Check. Nicht-erfüllte ACs triggern R-SC-01.
- **R-SC-03 — Versions-Manifest-Konsistenz vor Release-Tag:** Cargo.toml, package.json, tauri.conf.json müssen auf Ziel-Tag-Version stehen, BEVOR der Tag gepusht wird.

**Warum:**
Phase 8 hat AC9 (Storage-Forecast-UI) und AC10 (Library-UI-Re-Konzeption) komplett deferred plus AC8/AC5 partial — alle ohne Mid-Run-CTO-Konsultation. Wurde in Open-Follow-ups dokumentiert, aber damit war v2.0.0 released ohne den spec'd Scope. Zusätzlich wurde v2.0.0 mit 1.0.0-Versions-Manifests getaggt, was Hotfix + Re-Release nötig machte.

CEO-Reaktion: "Locked Decisions sind locked. Defer braucht Freigabe."

**Referenz:**
- CEO-Direktive im CTO-Chat, 2026-04-25, im Anschluss an Phase-8-Final-Report.
- Neue Rule-Datei: `.claude/rules/scope-control.md`
- Erste Anwendung: v2.0.1 (Catch-up-Mission für AC9, AC10, AC8-Wiring, AC5-Windows).

---

## 2026-04-25 — Review-Cycle-Verschärfung (R-RC-01/02/03)

**Was:**
Neue Rule-Datei `.claude/rules/review-cycles.md` mit drei Verschärfungen:

- **R-RC-01 — Mid-Phase Review-Gates:** Code-Reviewer läuft nach jedem größeren Sub-Block, nicht erst am Phase-Ende.
- **R-RC-02 — Re-Review nach Critical/High-Fix:** Reviewer läuft zweites Mal auf Fix-Diff.
- **R-RC-03 — Cross-Subagent-Awareness:** Zweiter Reviewer bekommt Findings des ersten im Kontext.

**Warum:**
Phase 6 und Phase 7 hatten Subagent-Reviews nur am Phase-Ende. Phase 7 fand 7 High-Findings — alle inline gefixt, aber ohne Re-Review-Verifikation. CEO-Direktive: "Besser zweimal zu viel als einmal zu wenig."

**Erste Anwendung Phase 8 — Effektivität bestätigt:**
CC's eigene Bewertung: "Phase 7 found 7 High end-of-phase only; Phase 8 caught and fixed equivalent issues per sub-block." R-RC-01 hat funktioniert wie geplant.

**Referenz:**
- CEO-Direktive im CTO-Chat, 2026-04-25.
- Rule-Datei: `.claude/rules/review-cycles.md`
- Anwendung: Phase 8 (Storage-Aware Distribution).

---

## 2026-04-25 — Konvention: Senior Engineer macht Merge + Tag

**Was:**
Merge- und Tag-Operationen sind Senior-Engineer-Aufgabe (Claude Code via gh CLI).

**Warum:**
Jan ist nicht-technischer CEO. Senior Engineer hat gh-CLI-Zugriff und kann Merge → Branch-Delete → Tag → Push atomar durchführen.

**Referenz:**
CEO-Klarstellung 2026-04-25. Anwendung: PR #15 bis PR #20 alle vom Senior Engineer gemerged + getaggt. v1.0.0 + v2.0.0 ebenfalls.

---

## 2026-04-25 — STATE-getriebenes CTO-Onboarding etabliert

**Was:**
- `docs/STATE.md` als Pflicht-Briefingdokument
- `docs/HANDOFF-TEMPLATE.md` als Übergabe-Template
- `.claude/CHANGELOG.md` als Workforce-Drift-Tracker

**Warum:**
Sessions ohne Memory-Rekonstruktion starten zu können.

**Referenz:**
Project Instructions 2026-04-25.
