# R-SC — Scope Control

**Status:** Aktiv ab v2.0.1 (2026-04-25)
**Scope:** Alle Phasen mit Long-Run-Mission und Locked Decisions
**Eingeführt durch:** Phase 8 Scope-Reduktion ohne Freigabe (AC9 + AC10 deferred)

---

## R-SC-01 — Scope-Reduction-Approval

Wenn ein Workstream, eine Sub-Phase oder ein Acceptance-Criterion, das in der Mission als Locked Decision oder als explizit gefordert spezifiziert wurde, während des Runs deferred, gestrichen oder substantiell reduziert werden soll, MUSS ein STOP-Trigger feuern und CTO-Freigabe eingeholt werden.

**Definition "Scope-Reduktion":**
- Workstream-Skip (z.B. Sub-Phase D + E ausgelassen)
- AC-Defer (z.B. AC9 von ✅ auf ❌-deferred)
- Funktionale Kürzung (z.B. "Math im Backend, UI fehlt")
- Locked-Decision-Aufweichung (z.B. "Hardware-Encode für Windows skipped")

**KEINE Scope-Reduktion (R-SC-01 nicht anwendbar):**
- Einzelne Low/Medium-Findings als Follow-up dokumentieren
- Implementation-Detail-Wahl innerhalb spec'd Scopes
- Performance-Optimierung verschoben (solange AC erfüllt)
- Refactoring oder Polish-Items, die nicht spec'd waren

**Ablauf bei drohender Scope-Reduktion:**
1. Senior Engineer erkennt: ein Workstream/AC kann nicht im aktuellen Run geliefert werden.
2. SOFORT STOP — kein eigenmächtiges Defer.
3. Report an CTO mit:
   - Welcher Workstream/AC ist betroffen
   - Warum Lieferung im aktuellen Run nicht möglich (Token-Budget, Komplexität, Blocker)
   - Vorschlag: vollständig deferred / partial mit dokumentierter Lücke / verlängerter Run / etwas anderes deferren
4. CTO entscheidet, gibt explizit frei oder fordert anderen Pfad.
5. Erst nach Freigabe weiter.

**Begründung:**
Phase 8 hat AC9 (Storage-Forecast-UI) und AC10 (Library-UI-Re-Konzeption) komplett deferred plus AC8 + AC5 partial — alle ohne Mid-Run-CTO-Konsultation. Wurde in Open-Follow-ups dokumentiert, aber das ändert nichts daran, dass v2.0.0 released wurde, ohne dass es das spec'd v2.0 war. Locked Decisions sind locked, Spec ist Spec, Defer braucht Freigabe.

**Token-Budget-Erwägung:**
Wenn der Run an Token-/Kontext-Limits stößt und nicht alles geliefert werden kann, ist DAS der STOP-Trigger. Eigenmächtige Priorisierung "Backend vor UI" ist nicht akzeptabel — der CTO entscheidet, ob lieber ein Workstream komplett gut wird oder alle Workstreams partial.

---

## R-SC-02 — Acceptance-Criteria-Vollständigkeits-Check

Vor dem End-of-Phase-Review (also vor dem finalen Subagent-Review-Block) prüft der Senior Engineer aktiv jeden AC einzeln:

**Format pro AC im internen Status-Check:**
- AC-Nummer
- Beschreibung (kurz)
- Implementiert? (ja/nein/partial)
- Tests vorhanden? (ja/nein)
- UI/UX-Surface erreichbar? (falls AC user-facing)
- Migration sauber? (falls schemarelevant)

**Wenn ein AC `nein` oder `partial` ist:**
→ R-SC-01 Scope-Reduction-Approval-Pfad starten, NICHT als "Open Follow-up" stillschweigend dokumentieren.

**Begründung:**
Phase 8 hat AC9 + AC10 ohne aktiven AC-Check verloren — sie tauchten erst im Final-Report als deferred auf. Aktiver Vollständigkeits-Check vor End-of-Phase-Review hätte den Stop-Trigger früher gesetzt.

---

## R-SC-03 — Versions-Manifest-Konsistenz vor Release-Tag

Vor dem Release-Tag-Push verifiziert der Senior Engineer, dass alle Versions-Manifeste auf den Ziel-Tag aktualisiert sind:

- `Cargo.toml` (Workspace + Crates)
- `Cargo.lock`
- `package.json`
- `tauri.conf.json` `version`
- ggf. weitere (`pnpm-lock.yaml` ist auto, prüft `package.json`-Versions-Konsistenz)

**Ablauf:**
1. Phase fertig, vor `git tag vX.Y.Z`
2. Bump-Commit: `chore(release): bump version to X.Y.Z`
3. Verifikation: `grep -r "version" Cargo.toml package.json tauri.conf.json` zeigt überall X.Y.Z
4. Quality Gates re-run nach Bump
5. Tag, Push

**Begründung:**
Phase 8 hat v2.0.0 getaggt, während Manifests auf 1.0.0 standen. Folge: Release-Run produzierte `Sightline_1.0.0_*.dmg` etc., Hotfix-PR + Re-Tag + Re-Run nötig. R-SC-03 verhindert das.

**CI-Gate (zukünftig, optional):**
Release-Workflow kann Vorab-Step "version-consistency-check" haben: liest Tag-Name, vergleicht mit Cargo.toml-Version, fail wenn mismatch. Verhindert auch manuellen Push-Fehler.

---

## Reporting im Final-Report

Phase-Final-Report bekommt eine neue Section "Scope-Compliance":
- Anzahl Scope-Reductions im Run (ideal: 0)
- Bei jeder Scope-Reduction: Datum/Sub-Phase/AC, Freigabe-Quelle (CTO-Chat-Referenz), Pfad (deferred/partial/anderes)
- AC-Vollständigkeits-Check (Tabelle mit allen ACs auf grün vor End-of-Phase-Review)
- Versions-Manifest-Konsistenz vor Release-Tag bestätigt (ja/nein)

---

## Versionshistorie

- 2026-04-25 — Initial draft, eingeführt nach Phase 8 wegen AC9/AC10-Defer ohne Freigabe.
