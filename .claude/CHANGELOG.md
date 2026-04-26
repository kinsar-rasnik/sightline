# Synthetic Workforce CHANGELOG

Pflege-Disziplin: Jede Änderung an `.claude/`, `CLAUDE.md`, oder Workforce-relevanten Konventionen bekommt einen Eintrag mit Was / Warum / Referenz.

Format: `## YYYY-MM-DD — Titel`

---

## 2026-04-25 — Review-Cycle-Verschärfung (Mid-Phase + Re-Review)

**Was:**
Neue Rule-Datei `.claude/rules/review-cycles.md` mit drei Verschärfungen für künftige Phasen:

- **R-RC-01 — Mid-Phase Review-Gates:** Nach jedem größeren Sub-Block (Backend-Service-fertig, Frontend-Feature-fertig, Migrationen-fertig) läuft `code-reviewer` auf den Sub-Block-Diff. Findings werden vor dem nächsten Sub-Block gefixt, nicht erst am Ende. Ziel: vermeiden, dass weitere Sub-Blöcke auf Buggy-Foundation aufbauen.
- **R-RC-02 — Re-Review nach Critical/High-Fix:** Wenn `code-reviewer` oder `security-reviewer` einen Critical- oder High-Befund melden und ein Fix-Commit landet, läuft der entsprechende Reviewer NOCHMAL auf den Fix. Ziel: Schließen des Review-Loops, Verifikation dass der Fix tatsächlich die Root Cause adressiert und kein Folge-Issue erzeugt.
- **R-RC-03 — Cross-Subagent-Awareness:** Wenn beide Reviewer (code + security) parallel laufen, bekommen sie jeweils im Prompt-Kontext die Findings des anderen mitgeliefert. Ziel: Koordinierte Bewertung, weniger Duplikation, mehr Vollständigkeit.

**Warum:**
Phase 6 und Phase 7 haben Subagent-Reviews nur am Phase-Ende ausgeführt. Phase 6 fand 2 P0-Findings spät, Phase 7 fand 7 High-Findings — alle inline gefixt, aber ohne Re-Review-Verifikation. Das hat funktioniert, aber das Risiko skaliert: bei größeren Phasen (Phase 8 wird umfangreicher als Phase 7) wäre ein End-of-Phase-Review-Stack mit 15+ Findings auf einmal schwer sauber zu fixen. CEO-Direktive 2026-04-25: "Besser zweimal zu viel als einmal zu wenig."

**Referenz:**
- CEO-Direktive im CTO-Chat, 2026-04-25, im Anschluss an Phase-7-Final-Report.
- Neue Rule-Datei: `.claude/rules/review-cycles.md`
- Erste Anwendung: Phase 8 (Storage-Aware Distribution).

---

## 2026-04-25 — Konvention: Senior Engineer macht Merge + Tag

**Was:**
- Merge- und Tag-Operationen sind Senior-Engineer-Aufgabe (Claude Code via gh CLI), nicht CEO-Aufgabe.
- CEO-Freigabe erfolgt über PR-Review und explizite Anweisung "merge das jetzt".

**Warum:**
- Jan ist nicht-technischer CEO. Terminal-Operationen sind nicht im Rollenbild.
- Web-UI-Merge-Flow ist umständlich für eine atomare Sequenz aus Merge → Branch-Delete → Tag → Push.
- Senior Engineer hat ohnehin gh-CLI-Zugriff und kann den ganzen Block atomar durchführen.

**Referenz:**
- CEO-Klarstellung im CTO-Chat, 2026-04-25, im Kontext von PR #15 Merge-Vorbereitung.
- Anwendung: PR #15, #16, #17, #18 alle vom Senior Engineer gemerged + getaggt. v1.0.0-Release-Tag ebenfalls vom Senior Engineer gesetzt.

---

## 2026-04-25 — STATE-getriebenes CTO-Onboarding etabliert

**Was:**
- `docs/STATE.md` als Pflicht-Briefingdokument für CTO-Sessions
- `docs/HANDOFF-TEMPLATE.md` als Template für Phasen-Übergaben
- `.claude/CHANGELOG.md` als zentraler Tracker für Workforce-Evolution

**Warum:**
Sessions ohne Memory-Rekonstruktion starten zu können. STATE.md fungiert als Single-Source-of-Truth-Kompass; HANDOFF-Template strukturiert Übergaben; CHANGELOG dokumentiert Workforce-Drift.

**Referenz:**
Project Instructions vom CEO, 2026-04-25. Erste Anwendung: Phase 6 Housekeeping → Phase 6 proper Übergang.
