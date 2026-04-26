# Synthetic Workforce CHANGELOG

Pflege-Disziplin: Jede Änderung an `.claude/`, `CLAUDE.md`, oder Workforce-relevanten Konventionen bekommt einen Eintrag mit Was / Warum / Referenz.

Format: `## YYYY-MM-DD — Titel`

---

## 2026-04-26 — R-SC + R-RC im Kombi-Einsatz bewährt (v2.0.1)

**Was:**
v2.0.1 war der erste Run unter beiden Rule-Sets gleichzeitig — R-RC-01/02/03 (Review-Cycles) und R-SC-01/02/03 (Scope-Control). Outcome:

- **R-SC-01 Scope-Reductions: 0.** Alle 7 spec'd Items geliefert. Vergleich Phase 8: 4 ACs deferred ohne Freigabe.
- **R-SC-02 AC-Vollständigkeits-Check** vor End-of-Phase aktiv durchgeführt — Tabelle in Session-Report.
- **R-SC-03 Versions-Manifest-Konsistenz** vor Tag verifiziert — alle 5 Assets korrekt mit `Sightline_2.0.1_*` benamt. Vergleich Phase 8: misnamed-Release-Hotfix nötig.
- **R-RC-01 Mid-Phase-Reviews:** 5 Sub-Phasen-Reviews + 1 End-of-Phase = 18 Findings inline gefangen, bevor sie ins nächste Sub-Block-Buildup eskalieren konnten.
- **R-RC-02 Re-Reviews:** durchgeführt nach Critical/High-Fixes, max-2-Iterations nie getriggert.
- **R-RC-03 Cross-Subagent-Awareness:** End-of-Phase-Security-Review fand 1 zusätzliches LOW (`removeVod` Library-Root-Prefix-Guard) — wäre ohne Cross-Awareness vermutlich gemissed worden.

**Warum die Notiz:**
Beide Rule-Sets haben sich im Real-Run bewährt. Bleiben aktiv für alle künftigen Phasen. Keine Anpassungen nötig nach erstem Einsatz — das ist selten und bestätigt das Design.

**Referenz:**
- `docs/HANDOFF-2026-04-26-v2.0.1.md` Sections "Scope-Compliance" + "Review-Cycle-Statistik"
- Final-Report im CTO-Chat, 2026-04-26
- Rule-Files: `.claude/rules/scope-control.md` + `.claude/rules/review-cycles.md`

---

## 2026-04-25 — Scope-Control-Rules (R-SC-01/02/03)

**Was:**
Neue Rule-Datei `.claude/rules/scope-control.md` mit drei Regeln:

- **R-SC-01 — Scope-Reduction-Approval:** Eigenmächtige Defer/Skip von Locked Decisions oder spec'd ACs ist nicht erlaubt. STOP-Trigger + CTO-Freigabe Pflicht.
- **R-SC-02 — Acceptance-Criteria-Vollständigkeits-Check:** Vor End-of-Phase-Review aktiver AC-by-AC-Status-Check.
- **R-SC-03 — Versions-Manifest-Konsistenz vor Release-Tag:** Cargo.toml, package.json, tauri.conf.json müssen auf Ziel-Tag-Version stehen.

**Warum:**
Phase 8 hat AC9 + AC10 ohne Freigabe deferred. Zusätzlich wurde v2.0.0 mit 1.0.0-Versions-Manifests getaggt. Beide Probleme sollten nicht wieder auftauchen.

**Erste Anwendung:** v2.0.1 — siehe Eintrag oben, beide Probleme verhindert.

**Referenz:**
- CEO-Direktive im CTO-Chat, 2026-04-25.
- Rule-Datei: `.claude/rules/scope-control.md`

---

## 2026-04-25 — Review-Cycle-Verschärfung (R-RC-01/02/03)

**Was:**
Neue Rule-Datei `.claude/rules/review-cycles.md` mit drei Verschärfungen:

- **R-RC-01 — Mid-Phase Review-Gates**
- **R-RC-02 — Re-Review nach Critical/High-Fix**
- **R-RC-03 — Cross-Subagent-Awareness**

**Warum:**
Phase 6 und 7 hatten Reviews nur am Phase-Ende. Phase 7 fand 7 Highs spät. CEO-Direktive: "Besser zweimal zu viel als einmal zu wenig."

**Erste Anwendung Phase 8:** "Phase 7 found 7 High end-of-phase only; Phase 8 caught and fixed equivalent issues per sub-block." Effektivität bestätigt.

**Zweite Anwendung v2.0.1:** 18 Findings inline über 5 Sub-Phasen verteilt gefangen. Cross-Awareness fand zusätzliches LOW.

**Referenz:**
- CEO-Direktive im CTO-Chat, 2026-04-25.
- Rule-Datei: `.claude/rules/review-cycles.md`

---

## 2026-04-25 — Konvention: Senior Engineer macht Merge + Tag

**Was:**
Merge- und Tag-Operationen sind Senior-Engineer-Aufgabe (Claude Code via gh CLI).

**Warum:**
Jan ist nicht-technischer CEO. Senior Engineer hat gh-CLI-Zugriff und kann Merge → Branch-Delete → Tag → Push atomar durchführen.

**Referenz:**
CEO-Klarstellung 2026-04-25. Anwendung: PR #15 bis PR #21 alle vom Senior Engineer gemerged + getaggt. v1.0.0 + v2.0.0 + v2.0.1 ebenfalls.

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
