# R-ADR — ADR Discipline

**Status:** Aktiv ab v2.1-ADR-Phase (2026-05-12)
**Scope:** Alle ADR-Wellen mit PROPOSED/ACCEPTED-Lebenszyklus, insbesondere Token-Pinning- und Mehr-Welle-Phasen
**Eingeführt durch:** v2.1-ADR-Phase Welle 1 + Welle 2 + Welle 3 (ADR-0038 / ADR-0039 / ADR-0040)

---

## R-ADR-01 — Engineering-Anti-Smell-Audit (Pre-Delivery)

Vor Lieferung eines PROPOSED-ADR führt der Senior Engineer einen Anti-Smell-Audit durch. Audit-Method ist der 2-Frage-Test pro Decision UND pro beigesteuerte Sub-Decision:

1. Hat die Decision eine **user-feel-prägende** Implikation?
2. Ist sie **nicht durch Spec-Source** (Anker, Q-Decision-Log, vorherige ACCEPTED-ADRs, binding inputs aus der Mission-Spec-Quellen-Liste) **direkt determiniert**?

Beide Antworten „ja" → als **Open Question for CEO** ausweisen. Eine Antwort „nein" → **Engineering-Default mit Begründung** im Decision-Log §N Anti-Smell-Audit-Sektion.

**Pflicht-Klauseln im Audit-Method:**

- **Spec-Source-Verification.** Der Audit verifiziert die anchor / Q-Decision / ADR-Bindung gegen die **Spec-Source selbst** (Anker-Datei, Decision-Log, vorheriges ACCEPTED-ADR), **nicht gegen die Mission-Brief-Reproduktion** der Spec. Mission-Briefs können Spec-Bugs enthalten; bei Diskrepanz gilt die Spec-Source. Bei Spec-Source-Diskrepanz: in-flight-Korrektur mit expliziter Dokumentation im Decision-Log als eigene Sektion „Spec-Korrektur in-flight" (Beleg: ADR-0040 §7, H-20 Focus-Outline-Opacity 50 % → 85 % gegen Anker verifiziert; Mission-Brief hatte 50 %, Anker-§H-20 spec'd 80–90 %, Engineering wählte 85 % als mid).
- **Documentary-Konsistenz-Pass.** Anti-Smell-Audit prüft zusätzlich auf:
  - **Counting-Konsistenz** (Anzahl OQs, Nummerierungs-Lücken)
  - **Cross-Reference-Konsistenz** (Q-N-Referenzen in Decision-Sections / Tabellen / CSS-Block-Kommentaren / Consequences / Alternatives stimmen mit OQ-Liste überein)
  - **Cross-Decision-Compound-Notation** (OQs, die in der N_max-Math oder anderen Berechnungen nicht orthogonal sind, werden als Compound-Risk benannt)

  Hinweis: Welle 3 hat diesen Pass nicht vollständig durchgezogen — Counting-Bug („six" vs. „seven") und Compound-Risk-Gap (TV-A1 × TV-A5) wurden erst vom Pre-Sign-off-CTO-Audit (R-ADR-02) gefangen. Künftige Engineering-Anti-Smell-Audits sollen diesen Pass im Pre-Delivery-Schritt mitlaufen lassen.

**Reporting im Decision-Log:**
Eigene Sektion „§N Anti-Smell-Audit: explizit auf UX-prägende Sub-Decisions geprüft" mit:
- Audit-Methode-Beschreibung (kurz)
- Geprüfte Stellen (mission-explizit aufgelistete Anti-Smell-Risk-Kandidaten + im Schreiben aufgetauchte Sub-Decisions)
- Audit-Ergebnis (Anzahl OQs identifiziert, Anzahl Sub-Decisions als Engineering-Defaults dokumentiert)
- Ggf. „Spec-Korrektur in-flight"-Hinweis als eigene Decision-Log-Sektion

**Begründung:**
Welle 1 (ADR-0038 PROPOSED) hatte drei versteckte Sub-Decisions (CEO-A1 4-Pane-Multiview als Hedge in D5; CEO-A3 Hero-Trigger als Costs-Accepted-Mitigation; CEO-A4 Cascade-Lesart als Engineering-Judgement in Forward-References), die erst nach PROPOSED-Lieferung beim CTO-Audit als CEO-relevante Entscheidungen identifiziert wurden. Welle 2 (ADR-0039) hat Anti-Smell-Audit präventiv angewandt und produzierte 3 ehrliche OQs (VOd-Q2 / Q3 / Q5) statt versteckter Sub-Decisions. Welle 3 (ADR-0040) hat den Audit auf Spec-Source-Verification erweitert und fing den Mission-Brief-Spec-Bug (H-20 50 % → 85 %). R-ADR-01 codifiziert dieses gewachsene Audit-Pattern.

---

## R-ADR-02 — Pre-Sign-off-CTO-Audit

Vor dem CEO-Sign-off auf einen PROPOSED-ADR führt der CTO einen Pre-Sign-off-Audit auf das ADR + Decision-Log durch. Audit-Fokus ist **documentary-Konsistenz und cross-cutting-Risk-Notation** — Befunde sind dokumentarisch (Inhalts-Notation), nicht substantiell (Decision-Wahl-Wechsel).

**Audit-Punkte:**

- **Counting-Konsistenz.** Anzahl OQs in Decision-Log §6 Audit-Ergebnis ↔ Anzahl numerierter OQs in ADR-Open-Questions-Sektion ↔ Anzahl Decisions in CEO-Brief des CTO. Lücken in Nummerierung („skipping numerically") sind Pre-PROPOSED-Planungs-Residuen und werden im Acceptance-Patch bereinigt (Beleg: ADR-0040 hatte „six OQs" im Bericht / Decision-Log und „TV-Q1 bis TV-Q7" in der ADR — 7 reale OQs, „skipping TV-Q6 numerically that is actually included"-Formulierung als Patch-Item identifiziert).
- **Cross-Cutting-Risk-Notation.** OQs, die in N-Math oder anderen Berechnungen nicht orthogonal sind (Compound-Wechselwirkungen), werden als gemeinsamer Risk-Eintrag im Consequences > Risks-Block dokumentiert. Beleg: ADR-0040 TV-A1 × TV-A5 N_max-Compound-Wechselwirkung (`lane_height_px` und `hero-slot-height` treiben beide N_max — alternative Wertepaare ergeben non-trivial verschiedene N_max-Ladders).
- **Spec-Source-Re-Verification.** Bei langen Decision-Logs (> 200 Zeilen): Stichprobe-Check, ob die im Decision-Log angegebenen Spec-Source-Bindungen mit der Anker- / Q-Decision- / ADR-Source übereinstimmen. (Bei Welle 1+2+3 nicht gefangen, weil Spec-Source-Diskrepanz erst in Welle 3 durch Engineering selbst auftauchte; für künftige Wellen als Stichprobe weiterhin sinnvoll.)
- **Forward-Process-Path-Validation.** Wenn der PROPOSED-ADR eine Escalation-Note mit Subdivide-Empfehlung enthält: Validierung der tuning-vs-structural-Klassifikation (siehe R-ADR-04).

**Reporting:**
CTO formuliert pro Audit-Finding eine Patch-Anweisung an Engineering. Findings werden im Acceptance-Patch (R-ADR-03) als „Pre-Sign-off-CTO-Audit-Findings" mit-eingearbeitet, dokumentiert in der Acceptance-Block-Sektion oder einem dedizierten Decision-Log-Notiz-Eintrag.

**Stop-Trigger:**
Bei substantiellem Finding (eine Decision müsste umgeworfen werden, nicht nur dokumentarisch klargestellt): zurück zu R-ADR-01 (Engineering-Re-Run für die betroffene Decision-Sektion), nicht Pre-Sign-off-CTO-Audit-Patch.

**Begründung:**
Welle 3 hat zwei dokumentarische Findings gefangen, die der Engineering-Anti-Smell-Audit aufgrund Mission-Brief-Fokus nicht sah (Counting-Bug, Compound-Risk-Gap). R-ADR-02 verankert den CTO-Audit-Pfad als komplementäre Disziplin zu R-ADR-01.

**Komplementaritäts-Note.**
Engineering-Anti-Smell-Audit (R-ADR-01, Pre-Delivery, Engineering-Wahl-Space) und Pre-Sign-off-CTO-Audit (R-ADR-02, Pre-Sign-off, documentary-Konsistenz + cross-cutting-Risk) adressieren **verschiedene Failure-Modes** und sind **komplementär, nicht redundant**. Beide Audits zusammen schließen das ADR-Sign-off-Risiko über Engineering-Wahl-Substanz und ADR-Dokumentations-Hygiene.

---

## R-ADR-03 — Acceptance-Patch-Pattern (PROPOSED → ACCEPTED)

Beim CEO-Sign-off transitioniert das ADR von PROPOSED auf ACCEPTED über einen strukturierten Acceptance-Patch.

**Pflicht-Operations:**

1. **ADR-Status-Header.** `PROPOSED — awaiting CEO sign-off` → `ACCEPTED <date>` mit Acceptance-Block, der die akzeptierten Decisions referenziert und auf Decision-Log §N CEO Sign-off zeigt.
2. **OQ-Sektion-Konvertierung.** „Open Questions for CEO" → „CEO Decisions (<date>)" mit pro OQ einem `### Decision-AN`-Block:
   - Original-Frage gekürzt (1–2 Sätze, zur Nachvollziehbarkeit)
   - Decision-Statement (1 Satz, der akzeptierte Wert / die akzeptierte Wahl)
   - Begründung (2–4 Sätze, geglättet aus CTO-Empfehlung)
   - Folge für ADR (welche Decision-Section locked das Verhalten konkret)
3. **Escalation-Note-Resolution** (falls vorhanden). Header → „Escalation note for CTO (resolved <date>)" mit Schluss-Notiz, die den gewählten Forward-Process-Pfad (R-ADR-04) und die CEO-Block-Acceptance dokumentiert.
4. **Normative AN-Referenz-Updates.** Alle Q-N-Verweise in Decision-Sections, Tabellen, CSS-Block-Kommentaren, Consequences (Costs accepted / Neutral / Risks), Alternatives-Considered werden zu A-N-Verweisen. **Pflicht-Verifikation:** nach Patch dürfen Q-N-Verweise nur noch im historischen Kontext (Acceptance-Block-Zitat, Escalation-Note-Narrativ) auftauchen.
5. **Decision-Log §N CEO Sign-off-Sektion** (neu am Ende des Decision-Logs). Pro Decision:
   - 1-Satz Decision-Statement
   - 2–3-Satz Begründung (CTO-Lesart geglättet)
   - Folge für ADR (Decision-Section + lockedes Verhalten)
   
   Schluss-Notiz dokumentiert den Forward-Process-Pfad und ggf. Pre-Sign-off-CTO-Audit-Findings (R-ADR-02).
6. **AC-Vollständigkeits-Tabelle in Decision-Log §4** erweitern um Acceptance-Patch-Zeilen.
7. **CHANGELOG-Eintrag** in `.claude/CHANGELOG.md` oben mit Format „Was / Pattern / Warum / Referenz".
8. **STATE.md** Datums-Stempel, Ampel-Begründung, Roadmap, Änderungshistorie.
9. **CLAUDE.md Active-Decisions-Register:** PROPOSED → ACCEPTED mit kompakter Refinement-Liste.

**Zwei dokumentierte Acceptance-Patch-Pfade:**

- **Substanz-Patch.** CEO hat einen Engineering-Default substantiell geändert oder verschärft. Zusätzlich zu den Pflicht-Operations: Decision-Statement-Update an der betroffenen Decision-Sektion, Folge-Anpassungen an Pseudocode / Tabellen / Alternatives-Considered, Decision-Log §N CEO Sign-off-Sektion dokumentiert die Substanz-Änderung explizit als „Substanz-Patch (CEO modifiziert Engineering-Default)". Beleg: ADR-0038 CEO-A2 (Multi-Perspective-Threshold 75 % → 50 %).
- **Reine Status-Konvertierung.** CEO hat alle Engineering-Defaults bestätigt. Keine Decision-Substanz-Änderung; nur Pflicht-Operations (Status-Header, OQ → Decisions, Verweis-Updates, §N CEO Sign-off-Sektion, CHANGELOG / STATE / CLAUDE.md). Belege: ADR-0039 (VOd-A2 / A3 / A5) und ADR-0040 (TV-A1 bis TV-A7) Acceptance-Patches.

Pre-Sign-off-CTO-Audit-Findings (R-ADR-02) werden im Acceptance-Patch mit-eingearbeitet, **ohne** als eigener Forward-Process-Pfad-Variant geführt zu werden — sie sind dokumentarische Korrekturen, nicht Substanz-Patches.

**Begründung:**
Welle 1 (Substanz-Patch CEO-A2 mit drei Refinements ohne Substanz), Welle 2 (Reine Status-Konvertierung), Welle 3 (Reine Status-Konvertierung plus dokumentarische Audit-Findings) belegen beide Pfade. R-ADR-03 codifiziert die Patch-Disziplin damit reproduzierbar ist.

---

## R-ADR-04 — Three-Forward-Process-Options bei OQ-Eskalations-Schwelle

Wenn ein PROPOSED-ADR ≥ 5 CEO-relevante Open Questions (Mission-konfigurierbar, default 5) enthält, eskaliert die Engineering-Lieferung an den CTO über eine **Escalation-Note-Sektion** im ADR. Der CTO entscheidet zwischen drei Forward-Process-Pfaden:

1. **Forward-as-is.** Alle OQs zum CEO als Block; CTO formuliert pro OQ eine klare Empfehlung mit Begründung im CEO-Brief; CEO entscheidet pro OQ (üblicherweise Block-Akzeptanz, mit Möglichkeit zu OQ-individuellen Modifikationen).
2. **Pre-process.** CTO wählt Engineering-Defaults lokal, ADR transitioniert zu 0-OQ-Shape vor CEO-Sign-off, Decision-Log dokumentiert „CTO-approved engineering defaults" als eigene Kategorie.
3. **Subdivide.** CTO eskaliert strukturelle OQs an den CEO, akzeptiert tuning-OQs als Engineering-Defaults. Subdivide braucht **R-ADR-02-Pre-Sign-off-CTO-Audit-Validierung** der tuning-vs-structural-Klassifikation, weil die Klassifikation fuzzy sein kann (Brand-Touchpoints, User-Feel-prägende Thresholds, Cross-Decision-Compound-Effects sind nicht trivial als „tuning" einstufbar).

**Empirische Beobachtung:**
Block-Acceptance (Forward-as-is) skaliert linear. Welle 1 = 4 CEO-Decisions, Welle 2 = 3, Welle 3 = 7 — alle saubere Block-Acceptance ohne Re-Litigation. Forward-as-is ist Default-Pfad, wenn jede OQ mit klarer CTO-Empfehlung kommt.

**Subdivide-Pfad-Warnung:**
Welle 3 PROPOSED-Subdivide-Empfehlung hätte TV-Q7 (Live-Dot-Color, Brand-Touchpoint, nicht tuning) und TV-Q2 (Card-Width-Thresholds, User-Feel-prägend durch Compact/Sliver-Threshold-Impact) als „tuning" mis-klassifiziert. Subdivide ist primär für Phasen mit klar gemischter Wahl-Natur empfohlen (architektonisch ↔ kosmetisch trennbar).

**Pre-process-Pfad-Warnung:**
Pre-process führt eine **dritte „CTO-approved engineering defaults"-Kategorie** ein, die außerhalb der R-ADR-Rule-Binarität (Engineering-Default oder CEO-Decision) liegt. Verwende Pre-process sparsam, primär bei sehr niedriger Wahl-Risiko-Kategorie (z. B. reine Implementations-Detail-OQs ohne User-Feel-Impact).

**Begründung:**
Welle 3 hatte 7 OQs > 5-Schwelle → Escalation-Note in der ADR; CTO wählte Forward-as-is über die in der ADR empfohlene Subdivide-Variante; CEO akzeptierte alle 7 Engineering-Defaults als Block. R-ADR-04 codifiziert die Three-Path-Disziplin damit der CTO eine reproduzierbare Heuristik hat.

---

## Stop-Trigger spezifisch für ADR-Discipline

- **Engineering-Anti-Smell-Audit (R-ADR-01) findet substantielle Spec-Source-Diskrepanz**, die mehr als eine in-flight-Korrektur erfordert (z. B. Mission-Brief widerspricht der Anker-Decision in mehreren Stellen, nicht nur Einzelwert) → STOP, Eskalation an CTO mit Spec-Source-Reconciliation-Vorschlag, nicht eigenmächtige Multi-Korrektur.
- **Pre-Sign-off-CTO-Audit (R-ADR-02) findet substantiellen Befund**, der eine Decision umwerfen würde → STOP, zurück zu R-ADR-01 für die betroffene Decision-Sektion, nicht Acceptance-Patch.
- **Forward-Process-Pfad-Wahl (R-ADR-04) nicht eindeutig** zwischen Forward-as-is / Pre-process / Subdivide → STOP, Eskalation an CEO mit Vorschlag-Block, nicht eigenmächtige Pfad-Wahl.

---

## Reporting im Final-Report

ADR-Phase-Final-Report bekommt eine Section „ADR-Discipline-Statistik":

- Anzahl PROPOSED-ADRs in der Phase
- Anzahl ACCEPTED-ADRs (mit Sign-off-Datum je ADR)
- Anti-Smell-Audit-Statistik: Anzahl OQs identifiziert, Anzahl Sub-Decisions als Engineering-Defaults dokumentiert, ggf. Anzahl Spec-Korrekturen in-flight (R-ADR-01)
- Pre-Sign-off-CTO-Audit-Findings: dokumentarische Befunde pro ADR (R-ADR-02)
- Acceptance-Patch-Pfad pro ADR: Substanz-Patch / Reine Status-Konvertierung (R-ADR-03)
- Forward-Process-Pfad-Statistik bei ≥ 5 OQ-ADRs: Forward-as-is / Pre-process / Subdivide (R-ADR-04)
- Block-Acceptance-Skalierungs-Daten (Anzahl CEO-Decisions pro Welle) für künftige Default-Pfad-Erhärtung

---

## Versionshistorie

- **2026-05-12** — Initial draft, eingeführt nach v2.1-ADR-Phase Welle 1 + Welle 2 + Welle 3 (ADR-0038 / ADR-0039 / ADR-0040). Vier Rules codifizieren die gewachsene ADR-Workforce-Disziplin: Anti-Smell-Audit (Pre-Delivery), Pre-Sign-off-CTO-Audit (Pre-Sign-off, komplementär), Acceptance-Patch-Pattern (PROPOSED → ACCEPTED mit zwei Pfaden), Three-Forward-Process-Options (≥ 5 OQ-Schwelle).
