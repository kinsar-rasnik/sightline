# Synthetic Workforce CHANGELOG

Pflege-Disziplin: Jede Änderung an `.claude/`, `CLAUDE.md`, oder Workforce-relevanten Konventionen bekommt einen Eintrag mit Was / Warum / Referenz.

Format: `## YYYY-MM-DD — Titel`

---

## 2026-04-25 — Konvention: Senior Engineer macht Merge + Tag

**Was:**
- Merge- und Tag-Operationen sind ab sofort Senior-Engineer-Aufgabe (Claude Code via gh CLI), nicht mehr CEO-Aufgabe.
- CEO-Freigabe erfolgt über PR-Review und explizite Anweisung "merge das jetzt" — nicht mehr durch eigenen `gh pr merge`-Aufruf oder Web-UI-Klick.

**Warum:**
- Jan ist nicht-technischer CEO. Terminal-Operationen sind nicht im Rollenbild.
- Web-UI-Merge-Flow ist umständlich für eine atomare Sequenz aus Merge → Branch-Delete → Tag → Push.
- Senior Engineer hat ohnehin gh-CLI-Zugriff und kann den ganzen Block in einem Schritt sauber durchführen, inkl. Verifikation.
- Reduziert CEO-Block-Zeit zwischen "PR ist ready" und "main ist auf neuem Stand".

**Referenz:**
- CEO-Klarstellung im CTO-Chat, 2026-04-25, im Kontext von PR #15 Merge-Vorbereitung.
- Erste Anwendung: PR #15 (Phase 6 Housekeeping) und PR #16 (Phase 6 proper) — beide vom Senior Engineer gemerged + getaggt.
- Project Instructions (CEO macht Merge + Tag) sind damit überschrieben für Sightline-Workforce.

---

## 2026-04-25 — STATE-getriebenes CTO-Onboarding etabliert

**Was:**
- `docs/STATE.md` als Pflicht-Briefingdokument für CTO-Sessions eingeführt
- `docs/HANDOFF-TEMPLATE.md` als Template für Phasen- und Arbeitsblock-Übergaben angelegt
- `.claude/CHANGELOG.md` (dieses File) als zentraler Tracker für Workforce-Evolution etabliert

**Warum:**
Nach mehrtägigem Multi-Phase-Setup mit Proton-Drive-Inkompatibilitäten und GitHub-Connectivity-Issues wurde die Notwendigkeit deutlich, Sessions ohne Memory-Rekonstruktion starten zu können. STATE.md fungiert als Single-Source-of-Truth-Kompass; HANDOFF-Template strukturiert Übergaben; CHANGELOG dokumentiert Workforce-Drift.

**Referenz:**
Project Instructions vom CEO, 2026-04-25. Erste Anwendung: Phase 6 Housekeeping → Phase 6 proper Übergang.
