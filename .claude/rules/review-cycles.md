# R-RC — Review Cycles

**Status:** Aktiv ab Phase 8 (2026-04-25)
**Scope:** Alle Phasen mit Long-Run-Mission und Subagent-Reviews
**Eingeführt durch:** CEO-Direktive 2026-04-25, dokumentiert in `.claude/CHANGELOG.md`

---

## R-RC-01 — Mid-Phase Review-Gates

Nach jedem größeren Sub-Block einer Phase läuft `code-reviewer` auf den Sub-Block-Diff. Findings werden vor dem nächsten Sub-Block gefixt.

**Definition "größerer Sub-Block":**
- Backend-Service-Modul fertig (Service + Commands + Events + IPC-Bindings)
- Frontend-Feature-Modul fertig (Page + Store + Hooks + UI)
- Datenbank-Layer fertig (alle Migrationen einer Phase + zugehörige Domain-Types)
- Workflow-/Pipeline-Definition fertig (z.B. CI-Workflows, Release-Workflows)

**Ablauf:**
1. Sub-Block-Commits sind alle gelandet (Quality Gates pro Commit grün).
2. `code-reviewer` Subagent läuft auf den Sub-Block-Diff (`git diff <last-block-end>..HEAD`).
3. Severity-Mapping:
   - **Critical/High:** SOFORT fixen, dann R-RC-02 (Re-Review).
   - **Medium:** Im selben Sub-Block fixen, danach weiter.
   - **Low:** In Follow-ups dokumentieren, weiter zum nächsten Sub-Block.
4. Erst nach Mid-Phase-Review fortfahren mit dem nächsten Sub-Block.

**Begründung:**
Vermeidet, dass weitere Sub-Blöcke auf Buggy-Foundation aufbauen. End-of-Phase-Review mit 15+ Findings gleichzeitig ist nicht sauber inline fixbar.

**Ausnahme:**
Sub-Block ist trivial klein (≤ 3 Files, ≤ 100 Zeilen Diff) und die Phase hat noch ≥ 2 Sub-Blöcke. Dann darf der Mid-Phase-Review beim nächsten "richtigen" Sub-Block mitlaufen. Im Decision-Log dokumentieren.

---

## R-RC-02 — Re-Review nach Critical/High-Fix

Wenn ein Reviewer (`code-reviewer` oder `security-reviewer`) einen Critical- oder High-Befund meldet und ein Fix-Commit landet, läuft DERSELBE Reviewer ein zweites Mal auf den Fix-Diff.

**Ablauf:**
1. Reviewer findet Critical/High → Finding dokumentiert.
2. Senior Engineer baut Fix → Quality Gates grün → Commit.
3. SAME Reviewer läuft erneut, scope eingegrenzt auf den Fix-Commit.
4. Re-Review-Outcome:
   - **Clean:** Finding als resolved markieren, weiter.
   - **Neue Findings im Fix:** wieder zu Schritt 2 (max. 2 Iterations, dann STOP-Trigger).
   - **Original-Issue weiterhin partiell vorhanden:** STOP-Trigger, Eskalation an CTO.

**Begründung:**
Schließt den Review-Loop. Verhindert dass Fixes neue Issues einführen oder das Original-Issue nur teilweise adressieren. Phase 6 und 7 haben das nicht gemacht — bei 7 High-Findings in Phase 7 wäre Re-Review-Cycle das letzte Sicherheitsnetz gewesen.

**Token-Budget:**
Re-Review kostet ~5-10K Tokens je Run. Bei einer Phase mit 5 High-Findings → ~50K Tokens Zusatz. Akzeptabel im Vergleich zu einem Bug, der erst im Production-Use auftaucht.

---

## R-RC-03 — Cross-Subagent-Awareness

Wenn `code-reviewer` und `security-reviewer` parallel oder sequenziell auf demselben Diff laufen, bekommt der zweite Reviewer die Findings des ersten im Prompt-Kontext mitgeliefert.

**Ablauf:**
1. Erster Reviewer läuft (üblicherweise `code-reviewer`).
2. Findings werden in strukturiertem Format (Severity, File, Line, Issue) gesammelt.
3. Zweiter Reviewer (`security-reviewer`) bekommt im Prompt-Kontext den Block:
   ```
   PEER FINDINGS FROM code-reviewer:
   - [Severity] File:Line — Issue summary
   - ...
   Bitte berücksichtigen: Duplikate als "concur" markieren, nicht doppelt zählen.
   Eigenständig prüfen: Sicherheits-Aspekte, die der peer übersehen haben könnte.
   ```
4. Final-Report konsolidiert: einmal "concurred by both", einmal "code-only", einmal "security-only".

**Begründung:**
Phase 6 hat zufällig beide Reviewer dasselbe `record_drift`-Validation-Issue gefunden — wäre besser, wenn das einmal koordiniert auftaucht statt als doppelter Befund. Spart Doppelarbeit, erhöht Vollständigkeit.

**Ausnahme:**
Wenn `code-reviewer` 0 Findings hat, kann `security-reviewer` ohne Cross-Awareness laufen (nichts zu kommunizieren).

---

## Stop-Trigger spezifisch für Review-Cycles

- Re-Review nach 2 Iterations immer noch nicht clean → STOP, Eskalation.
- Mid-Phase-Review findet Architektur-Issue, das Sub-Block-Rollback erfordert → STOP, Eskalation.
- Reviewer-Output nicht parsbar (Format-Drift) → STOP, manuelle Reviewer-Definition prüfen.

---

## Reporting im Final-Report

Phase-Final-Report bekommt eine neue Section "Review-Cycle-Statistik":
- Anzahl Mid-Phase-Reviews durchgeführt
- Anzahl Re-Reviews durchgeführt (mit Iteration-Count je Re-Review)
- Cross-Subagent-Awareness aktiviert (ja/nein, Begründung wenn nein)
- Findings-Verteilung über die Mid-Phase-Reviews vs. End-of-Phase-Review (zeigt ob die Verschiebung nach vorn funktioniert hat)

---

## Versionshistorie

- 2026-04-25 — Initial draft, eingeführt vor Phase 8 nach CEO-Direktive.
