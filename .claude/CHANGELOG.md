# Synthetic Workforce CHANGELOG

Pflege-Disziplin: Jede Änderung an `.claude/`, `CLAUDE.md`, oder Workforce-relevanten Konventionen bekommt einen Eintrag mit Was / Warum / Referenz.

Format: `## YYYY-MM-DD — Titel`

---

## 2026-05-12 — ADR-0041 ACCEPTED (Acceptance-Patch nach R-ADR-03 reine Status-Konvertierung)

- **Was:** ADR-0041 Multiview Pane Expansion ACCEPTED 2026-05-12
  als Block; M-Q1..M-Q5 → M-A1..M-A5 mit Engineering-Empfehlungen
  als pinned values. v2.1-ADR-Phase strukturell vollständig.
- **Pattern:** R-ADR-03 reine-Status-Konvertierung (analog ADR-0039
  am selben Tag). Keine Substanz-Patches, keine in-flight-
  Korrekturen, keine neuen Compound-Risks. Erste Forward-as-is-
  Anwendung unter den neuen R-ADR-Rules (verankert nach Welle 3,
  2026-05-12). R-ADR-04 ≥ 5 OQ-Schwelle exact getroffen, Block-
  Acceptance sauber.
- **Warum:** CEO concur mit allen fünf Engineering-Empfehlungen;
  Compound-Risk #4 (M-A3 × M-A4) als Paar adoptiert. Phase-
  Abschluss-Trigger: nach ADR-0041 ACCEPTED ist Wave-4-
  Engineering-Implementation entblockt.
- **Referenz:** `docs/adr/0041-multiview-pane-expansion.md` §Acceptance + Decisions; `docs/decision-log/v2.1-adr-0041-multiview-pane-expansion.md` §7.

---

## 2026-05-12 — ADR-0041 Multiview Pane Expansion entworfen (PROPOSED) — CEO-A1-Folge-Welle aus ADR-0038, parallele v2.1-ADR-Welle

**Was:**
- `docs/adr/0041-multiview-pane-expansion.md` ausgeliefert mit Status PROPOSED. Zwölf M-Decisions:
  - **M1 Pane-Count Layout-Geometry** — drei Layout-Varianten: N=2 horizontal 50/50 (ADR-0021 binding), N=3 Engineering-recommendation (d) 2×2-with-empty (siehe M-Q1), N=4 2×2-Quadranten Equal-size (Anker Q-8 binding). Inter-pane gap `--space-1` (4 px) aus ADR-0040 TV3. N=1 ist nicht Multiview-State per Selection-Cap-2-4 (ADR-0038 D7).
  - **M2 Reshuffle on Pane-Add** — Deterministic slot-index Order (0=top-left, 1=top-right, 2=bottom-left, 3=bottom-right); new pane occupies first empty slot; 200ms opacity crossfade via `--motion-zoom-duration` (ADR-0040 TV6); reduced-motion → 100ms `--motion-fade-instant`.
  - **M3 Reshuffle on Pane-Remove** — Slot-preserving with re-pack at N → N−1 boundary (4→3: slot bleibt leer; 3→2: re-pack zu 50/50; 2→1: session closes). Leader re-election = lowest remaining pane-index. Audio-anchor re-routing = back to leader if anchor removed.
  - **M4 Audio-Anchor Mechanic** — Default Audio = Leader (ADR-0023 binding); Switch on pointer-hover non-Leader-Pane (Anker Q-8 binding). Sub-Decisions M4a/M4b/M4c als Open Questions M-Q3/M-Q4/M-Q5; M4d (viewport-exit) = same logic as M4c; M4e = Lucide Volume2 glyph in pane-chrome (TV11 binding). State-Diagram für Default/Hover-Switch/Return-Behavior.
  - **M5 Leader-Outline Visual** — 2px Outline in `--color-outline-focus` (rgba(255,255,255,0.85)) mit `--focus-outline-offset` 2px inside; Z-index = `--z-card-hover` (20); Crown-glyph aus Lucide TV11 als Leader-Badge. Ersetzt v1 `★ Leader in amber pill` (H-20/H-07-Verletzung).
  - **M6 Pane-Selection-from-Timeline Integration** — Additive widening von `openSyncGroup({ vodIds, layout })`: `vodIds.length` von strict 2 auf `[2, 4]`; `layout` discriminant von `"split-50-50"` auf `"split-50-50" | "quadrant-2x2"`. Selection-Reihenfolge ↔ Pane-Index Ascending; First-selected = Pane 0 = Default-Leader.
  - **M7 Pane-Close Affordance** — Open Question M-Q2; Engineering-Empfehlung (α) Per-pane Close-Button (Lucide X in pane-chrome top-right, hover-revealed). Fallback (β) Group-close only (ADR-0021 status quo).
  - **M8 Transport-Bar Extension** — Strukturell unverändert; nur visuell Wave-4 TV11-Lucide-Re-Skin. Keine globale Audio-Anker-Indicator-Addition (M4e pane-chrome glyph ist single-source).
  - **M9 Mute-Toggle Semantic under Audio-Anchor** — Per-pane mute = local pane property; audio-routing-layer computes `effective_audio = anchor_pane.volume × (anchor_pane.muted ? 0 : 1)`. Preserves existing per-pane mute API + IPC surface unchanged (ADR-0021 binding).
  - **M10 Focus-Mode Toggle** — Deferred zu zukünftiger v2.x-ADR per Anker Q-8 binding ("future opt-in option, not the default"). Spec-binding-Defer, kein eigenmächtiges.
  - **M11 Out-of-Range per Pane at N > 2** — ADR-0022 binding unverändert pro Pane; Pane bleibt im Grid-Slot, Out-of-Range-Overlay angezeigt. Auto-collapse rejected (würde M1 stable-grid-invariant brechen).
  - **M12 Token Consumption + New Multiview Tokens** — Vollständige Konsumptions-Inventur aus ADR-0040 (TV1 outline, TV3 spacing, TV5 outline-width/offset, TV6 motion, TV7 z-index, TV11 icons). Drei neue Multiview-spezifische Tokens: `--multiview-pane-gap` (alias `--space-1`); `--motion-audio-switch-delay` (per M-Q3); `--motion-audio-switch-duration` (per M-Q4).
- **Fünf Open Questions for CEO im PROPOSED-ADR:** M-Q1 (3-Pane Layout (a)/(b)/(c)/(d) — Engineering-Empfehlung (d) 2×2-with-empty), M-Q2 (Per-pane vs Group-only Close — Engineering-Empfehlung (α) Per-pane), M-Q3 (Audio-Switch-Delay 0/200/800ms — Engineering-Empfehlung 200ms), M-Q4 (Hard-switch vs Crossfade — Engineering-Empfehlung Hard-switch), M-Q5 (Spring-back vs Sticky on Hover-Exit — Engineering-Empfehlung Spring-back). Engineering-Defaults dokumentiert mit Alternatives-Tabellen.
- **Escalation-Note für CTO als formaler Forward-Process-Surface (5 OQs at exact R-ADR-04-Schwelle ≥ 5).** Forward-as-is (Option 1) empfohlen über Subdivide (Option 3 — tuning-vs-structural-Klassifikation bei M-Q1/M-Q4 structural, M-Q3 tuning, M-Q5 UX-feel; mapped awkwardly) und Pre-process (Option 2 — würde CEO aus 5 UX-prägenden Decisions herausdrücken). Per-OQ CTO-Empfehlung mit Rationale-Tag.
- **Compound-Risk M-Q3 × M-Q4 explicitly callout in Escalation note.** Audio-Switch-Delay × Audio-Switch-Strategy bilden zusammen das Audio-Switch-Verhalten; verschiedene Wertepaare ergeben non-trivial verschiedene UX-Charaktere (instant/responsive/cinematic). CEO sign-off sollte pair die zwei Decisions. M-Q1 × M-Q2 Interaction sekundär notiert (3-pane-Layout beeinflusst 4→3 Re-Pack-Cost).
- **Decision-Log-Begleit-Eintrag** in `docs/decision-log/v2.1-adr-0041-multiview-pane-expansion.md` mit pro M-Decision: gewählter Wert + Fallback-Wert + Reihenfolge-Begründung; ADR-Reihenfolge-Bestätigung (parallele Welle, nicht Teil der 3-Welle-Sequenz); Domain-Knowledge-Gap-Audit (5 Gaps geprüft als Engineering-Validation-Punkte); AC-Vollständigkeits-Tabelle (12 ACs ✅, 3 ⏳ für CHANGELOG/STATE/CLAUDE-Updates); Scope-Compliance-Notiz; §6 Anti-Smell-Audit mit explizitem Test pro Sub-Decision.
- **R-ADR-01 Engineering-Anti-Smell-Audit präventiv durchgeführt — erste ADR unter den neuen R-ADR-Rules (verankert 2026-05-12 nach Welle 3 retrospect).** Audit-Methode dokumentiert mit 8 Mission-Risk-Kandidaten geprüft (5 als OQs surfacd: M-Q1/M-Q2/M-Q3/M-Q4/M-Q5; 1 dropped als Duplikat M-Q6=M-Q2; 2 als Engineering-Defaults: M-Q7 Mute-Semantik via ADR-0021-binding, M-Q8 Focus-Mode-Defer via Anker-Q-8-binding) plus 13 weitere Sub-Decisions im Schreiben aufgetaucht, alle als Engineering-Defaults mit Spec-Source- oder Engineering-Judgment-Begründung. Counting-Konsistenz-Check (5 OQs konsistent in §Open-Questions / §Escalation / §M-N / §1 Trade-offs / §6.2 Tabelle / §6.3 Audit-Ergebnis); Cross-Reference-Check (alle M-Q1..M-Q5 in Decision-Sections / Tabellen / Consequences konsistent); Compound-Notation (M-Q3 × M-Q4 explizit). **Keine Spec-Source-Diskrepanz gefangen** (Welle-3-Pattern-Lehre H-20 50% → 85% nicht aufgetreten; Mission-Brief reproduzierte Anker §6.3 + Q-8 + ADR-Sources faithfully).
- CLAUDE.md „Active decisions"-Register um ADR-0041 (PROPOSED) ergänzt; STATE.md Änderungshistorie + Ampel grün (PROPOSED ändert nichts an System-Ampel; Real-Use-Phase läuft parallel weiter).
- Keine Produkt-Code-Änderungen. Keine `globals.css`-Edits. Keine `tailwind.config.*`-Erstellung. Keine `lucide-react`-Installation. Keine Multiview-Komponenten-Migration. Keine `src-tauri/`-Touches. Keine Backend-Schema-Änderungen. Keine Anker-Heuristik-Erweiterungen. Keine Re-Litigation von ADR-0021/0022/0023/0038/0040-Decisions. IPC-additive-Widening (`openSyncGroup` vodIds `[2,4]` + `quadrant-2x2` discriminant) und neue Multiview-Tokens (`--multiview-pane-gap`, `--motion-audio-switch-*`) als Wave-4-Engineering-Scope dokumentiert.

**Pattern:**
- **R-ADR-01 Engineering-Anti-Smell-Audit anwendbar in der ersten ADR nach Verankerung — präventiv statt retrospect.** Die Rules wurden 2026-05-12 nach Welle 1+2+3-Retrospect verankert; diese ADR ist die erste Vorwärtsanwendung. Audit-Workflow hat sich als reproduzierbar erwiesen: Pre-Delivery-2-Frage-Test (user-feel-prägend × nicht-Spec-determined) pro M-Decision und Sub-Decision; Spec-Source-Verification gegen Anker/ADRs (nicht Mission-Brief-Reproduktion); Documentary-Konsistenz-Pass (Counting + Cross-Reference + Compound-Notation). Ergebnis: 5 ehrliche OQs surfacd, 15 Sub-Decisions als Engineering-Defaults mit Begründung; keine versteckten Sub-Decisions wie Welle 1 CEO-A1/A3/A4 (R-ADR-01-Lehre "Hedges-im-ADR sind eskalations-pflichtig" hat hier präventiv getragen).
- **Compound-Risk-Notation generalisiert über N_max-Math hinaus.** Welle 3 fanden TV-A1 × TV-A5 N_max-Compound (Token-Pinning-Welle); hier ist M-Q3 × M-Q4 Audio-Switch-Character-Compound (Pane-Render-Welle). Pattern: jedes Mal wenn zwei OQs **gemeinsam** ein User-Feel produzieren (statt zwei separate Outputs), Compound-Risk explizit als Risks-Eintrag mit konkreten Wertepaar-Beispielen notieren + im Escalation-Note callout. Diese Generalisierung wandert in R-ADR-01-Documentary-Konsistenz-Pass-Klausel als implizite Pflicht.
- **Forward-as-is bleibt Default-Pfad bei klaren Engineering-Empfehlungen — auch bei exact-Schwelle 5 OQs.** R-ADR-04 Welle-3-Retrospect-Lehre: Subdivide-tuning-vs-structural-Klassifikation ist fuzzy; Forward-as-is mit Per-OQ-CTO-Empfehlung skaliert linear (4 Welle 1 → 3 Welle 2 → 7 Welle 3 → 5 jetzt). Diese ADR setzt das Pattern fort. Subdivide-Pfad-Warnung explizit dokumentiert (M-Q1/M-Q4 structural, M-Q3 tuning, M-Q5 UX-feel — mapped awkwardly auf binary tuning-vs-structural-Split); Pre-process-Pfad-Warnung explizit (würde CEO aus 5 UX-prägenden Decisions herausdrücken).
- **Token-Erweiterungen als ADR-Decisions sind R-ADR-01-konform** wenn purpose-distinct dokumentiert. M12 introduces `--multiview-pane-gap` (semantic alias for `--space-1`; nicht Re-Open von ADR-0040 TV3), `--motion-audio-switch-delay` (purpose-distinct von `--motion-hover-delay` Card-Preview-Token; M4-Rationale dokumentiert warum diese ≠ Card-Hover-Delay sein darf), `--motion-audio-switch-duration` (purpose-distinct von Card-Zoom/Crossfade-Tokens). Pattern: neue Tokens in Folge-ADRs sind keine Re-Open der Token-Pinning-Welle, sondern Multiview-spezifische Erweiterungen mit Rationale; ADR-0040-Tokens bleiben binding.

**Warum die Notiz:**
ADR-0041 ist die letzte ADR der v2.1-Phase. Nach Sign-off ist die v2.1-ADR-Phase strukturell vollständig (Wave 1 ADR-0038 + Wave 2 ADR-0039 + Wave 3 ADR-0040 + parallele Welle ADR-0041), und Wave-4-Engineering-Implementation startet entblockt: Token-Migration `globals.css` per ADR-0040, `lucide-react` install per TV11, `TimelinePage.tsx`/`VodCard.tsx`/`TimelineHeroSlot.tsx`-Rewrites per ADR-0038/0039, plus `MultiViewPage.tsx`/`MultiViewPane.tsx`/`sync-store.ts`-Extensions per ADR-0041. R-ADR-Rules präventiv angewandt (erste Vorwärtsanwendung nach Verankerung) — Pattern-Erfassung dokumentiert für künftige Multi-Welle-ADR-Phasen.

**Referenz:**
- ADR: `docs/adr/0041-multiview-pane-expansion.md` (Status PROPOSED, 12 M-Decisions, 5 OQs M-Q1..M-Q5, Escalation note Forward-as-is, Compound-Risk M-Q3 × M-Q4 in Risks #4 + M-Q1 × M-Q2 in Risks #5)
- Decision-Log: `docs/decision-log/v2.1-adr-0041-multiview-pane-expansion.md` (§1 Trade-offs M1–M12 mit Fallback; §2 ADR-Reihenfolge parallele Welle; §3 Domain-Gap-Audit 5 Gaps; §4 AC-Check Tabelle; §5 Scope-Compliance; §6 Anti-Smell-Audit R-ADR-01 explicit mit §6.1 Methode / §6.2 geprüfte Stellen / §6.3 Audit-Ergebnis / §6.4 Spec-Source-Verification / §6.5 Pattern-Lehre)
- Spec-Sources verifiziert gegen: `docs/reference/visual-language-netflix.md` §6.3 + §7 Q-8; `docs/adr/0021-split-view-layout.md` Decision-Sektion; `docs/adr/0022-sync-math-and-drift.md` Decision-Sektion; `docs/adr/0023-group-wide-transport.md` Follow-up #1 + Follow-up #4; `docs/adr/0038-timeline-layout-single-time-axis.md` D7 Selection-Cap-of-4 + CEO-A1; `docs/adr/0040-visual-system-v2.md` TV1/TV3/TV5/TV6/TV7/TV11
- CLAUDE.md Active-Decisions-Register: ADR-0041 als (PROPOSED) eingetragen
- STATE.md Änderungshistorie 2026-05-12 + Ampel-Begründung mit Verweis auf Wave-4-Entblock-Pfad

---

## 2026-05-12 — Workforce-Rules-Erweiterung: `.claude/rules/adr-discipline.md` neu (R-ADR-01..R-ADR-04)

**Was:**
- Neue Rules-Datei `.claude/rules/adr-discipline.md` mit vier Rules, die die v2.1-ADR-Phase-Lehren formal verankern:
  - **R-ADR-01 Engineering-Anti-Smell-Audit (Pre-Delivery).** 2-Frage-Test (user-feel-prägend × nicht-determiniert) pro Decision + Sub-Decision. Pflicht-Klauseln: Spec-Source-Verification (gegen Anker, nicht gegen Mission-Brief-Reproduktion — H-20 Focus-Outline 50 % → 85 %-Catch aus Welle 3 als Beleg); Documentary-Konsistenz-Pass (Counting, Cross-Reference, Cross-Decision-Compound). Reporting: Decision-Log §N Anti-Smell-Audit-Sektion.
  - **R-ADR-02 Pre-Sign-off-CTO-Audit.** CTO-Audit vor CEO-Sign-off; Fokus documentary-Konsistenz und cross-cutting-Risk-Notation, nicht Substanz-Wechsel. Audit-Punkte: Counting-Konsistenz (Bericht ↔ Decision-Log ↔ ADR-OQ-Liste), Cross-Cutting-Risk-Notation (Compound-Wechselwirkungen wie TV-A1 × TV-A5 N_max), Spec-Source-Re-Verification (Stichprobe), Forward-Process-Path-Validation. Stop-Trigger bei substantiellem Finding: zurück zu R-ADR-01.
  - **R-ADR-03 Acceptance-Patch-Pattern.** 9 Pflicht-Operations für PROPOSED → ACCEPTED Transition (Status-Header, OQ-Konvertierung, Escalation-Note-Resolution, AN-Referenz-Updates global, Decision-Log §N CEO Sign-off, AC-Tabelle-Erweiterung, CHANGELOG, STATE, CLAUDE.md). Zwei dokumentierte Pfade: Substanz-Patch (Welle 1 CEO-A2 75 → 50 %) vs. Reine Status-Konvertierung (Welle 2 + 3). Pre-Sign-off-CTO-Audit-Findings werden im Acceptance-Patch mit-eingearbeitet, kein eigener Pfad-Variant.
  - **R-ADR-04 Three-Forward-Process-Options bei ≥ 5 OQ-Schwelle.** Forward-as-is (Default; CTO formuliert pro OQ Empfehlung) / Pre-process (CTO wählt Defaults lokal; führt dritte „CTO-approved"-Kategorie ein) / Subdivide (structural OQs an CEO, tuning OQs als Defaults — braucht R-ADR-02-Validierung der Klassifikation). Empirische Beobachtung: Block-Acceptance skaliert linear (4 → 3 → 7 CEO-Decisions Welle 1 / 2 / 3); Forward-as-is ist Default-Pfad. Subdivide-Pfad-Warnung mit Welle-3-Beleg (TV-Q7 Brand-Touchpoint + TV-Q2 User-Feel-Threshold wären als tuning mis-klassifiziert).
- Stop-Trigger-Sektion für ADR-Discipline (Spec-Source-Diskrepanz / substantielles CTO-Audit-Finding / unklare Forward-Process-Pfad-Wahl).
- Reporting-im-Final-Report-Sektion: ADR-Discipline-Statistik (Anzahl PROPOSED / ACCEPTED, Audit-Statistiken, Patch-Pfade, Block-Acceptance-Skalierungs-Daten).
- Versionshistorie-Eintrag 2026-05-12 mit Verweis auf Welle 1 + 2 + 3 als empirische Pattern-Source.
- **CLAUDE.md unverändert.** CLAUDE.md hat nur einen generischen `.claude/rules/`-Glob-Pointer (Z. 75) ohne per-Rule-Register; neue Datei wird automatisch über den Glob mitgelesen. Mission-AC3 („wenn Rules-Register existiert: erweitern; sonst nichts ändern") greift damit Path-„sonst nichts ändern".
- **STATE.md** Änderungshistorie-Eintrag 2026-05-12 mit Verweis auf neue Rules-Datei und Multiview-Pane-Expansion-ADR als nächste Welle. Kein Ampel-Wechsel; bleibt Grün.

**Pattern:**
- **v2.1-ADR-Phase Welle 1 + 2 + 3 Lehre kondensiert: drei Anwendungs-Iterationen erlauben Rule-Verankerung ohne Über-Spezifikation.** Welle 1 produzierte den Pattern-Bug (3 versteckte Sub-Decisions als CEO-A1/A3/A4 nachgezogen); Welle 2 produzierte das Pattern-Fix (präventiver Anti-Smell-Audit liefert 3 ehrliche OQs); Welle 3 produzierte die Pattern-Erweiterung (Audit fängt zusätzlich Mission-Brief-Spec-Bug; CTO-Audit fängt dokumentarische Findings). Erst nach drei Iterationen sind die Rules verankerbar — frühere Verankerung hätte überanpassende Patterns produziert.
- **Block-Acceptance-Pattern skaliert linear (4 → 3 → 7 CEO-Decisions).** Welle 1 hatte 4 CEO-Decisions (3 versteckte + 1 Substanz), Welle 2 hatte 3 ehrliche OQs, Welle 3 hatte 7 OQs. Alle drei durch saubere Block-Acceptance ohne Re-Litigation. R-ADR-04 codifiziert Forward-as-is als Default-Pfad mit empirischem Backing.
- **Engineering-Anti-Smell-Audit (Pre-Delivery, Wahl-Space) und Pre-Sign-off-CTO-Audit (Pre-Sign-off, documentary + cross-cutting) sind komplementär, nicht redundant.** Verschiedene Failure-Modes: Engineering-Audit prüft Engineering-Wahl-Substanz (was sind die OQs vs. Defaults); CTO-Audit prüft ADR-Dokumentations-Hygiene (Counting, Cross-Reference, Compound-Notation). R-ADR-01 + R-ADR-02 zusammen schließen das ADR-Sign-off-Risiko über beide Achsen.
- **Spec-Source-Verification (gegen Anker, nicht Mission-Brief-Reproduktion) als explizite Audit-Klausel.** Welle 3 fing einen Mission-Brief-Spec-Bug (H-20 50 % → 85 %) weil der Engineering-Anti-Smell-Audit gegen die Anker-Quelle verifizierte statt gegen die Mission-Brief-Reproduktion zu vertrauen. Diese Disziplin wird in R-ADR-01 als Pflicht-Klausel verankert, mit dem Welle-3-§7-Spec-Korrektur-in-flight-Pattern als Beleg.

**Warum:**
Workforce-Lebendigkeit-Pflicht aus System-Prompt: bei neuen Patterns Rules erweitern. v2.1-ADR-Phase hat vier Patterns etabliert (Anti-Smell-Audit, Pre-Sign-off-CTO-Audit, Acceptance-Patch-Pattern, Three-Forward-Process-Options), die in Wave-4-Engineering und künftigen ADR-Wellen reproduzierbar sein müssen. Rule-Verankerung jetzt — nach drei Iterationen — vermeidet Über-Spezifikation und sichert die Patterns für Multiview-Pane-Expansion-ADR (nächste Welle) plus künftige Token-Pinning- oder Mehr-Welle-Phasen.

**Referenz:**
- Neue Datei: `.claude/rules/adr-discipline.md` (R-ADR-01 bis R-ADR-04 + Stop-Trigger + Reporting + Versionshistorie)
- Empirische Pattern-Source:
  - `docs/decision-log/v2.1-adr-0038-timeline-layout.md` §6 (CEO-A1..A4, Substanz-Patch-Beleg für R-ADR-03)
  - `docs/decision-log/v2.1-adr-0039-vodcard.md` §6 (Anti-Smell-Audit präventiv, Beleg für R-ADR-01) + §7 (CEO Sign-off, Reine-Status-Konvertierungs-Beleg für R-ADR-03)
  - `docs/decision-log/v2.1-adr-0040-visual-system-v2.md` §6 (Anti-Smell-Audit mit Spec-Bug-Catch, Beleg für R-ADR-01-Spec-Source-Verification) + §7 (Spec-Korrektur in-flight, Beleg für R-ADR-01-Pflicht-Klausel) + §8 (CEO Sign-off mit Komplementaritäts-Note, Beleg für R-ADR-02 + R-ADR-04)
- STATE-Eintrag: `docs/STATE.md` Änderungshistorie 2026-05-12

---

## 2026-05-12 — ADR-0040 Visual System v2 ACCEPTED — sieben CEO-Refinements als Block (TV-A1 bis TV-A7) + 2 Pre-Sign-off-CTO-Audit-Findings (Counting-Bug, Compound-Risk-Notation)

**Was:**
- ADR-0040 vom CEO am 2026-05-12 mit sieben finalen Decisions als Block abgenommen und damit auf Status **ACCEPTED** umgestellt. Alle sieben sind die im PROPOSED-ADR dokumentierten Engineering-Defaults, also keine TV-Decision-Substanz-Änderung:
  - **TV-A1 — lane_height_px = 72.** GTA-RP-Action ist bei 96 × 54 px Image-Bereich erkennbar (Streamer-Facecam und Overlay-UI lesbar); 60 px wäre 80 × 45 und kippt unter Erkennbarkeitsschwelle für den Sightline-Use-Case. N_max-Ladder 5/3 (no-hero/hero) ist gute Density für typische Multi-Stream-Browse-Szenarien.
  - **TV-A2 — Card-width-thresholds 160 / 96 px.** 96 = Image-Natural-Width und natürlicher Kipp-Punkt zwischen Compact und Sliver. 160 = 1.67× Image-Width, sauber für Two-Line-Strip bei Full-Variant.
  - **TV-A3 — hover-zoom-factor = 1.10.** Geometrisch sicherer Abstand zum Inter-Lane-Overlap-Edge (1.30 würde overlappen). Netflix-typische 1.20-Werte sind für Sightlines dichtere Lane-Struktur am Edge — 1.10 ist die safe-Wahl ohne wahrnehmbaren Verlust an Hover-Affordance.
  - **TV-A4 — sidebar-collapsed-width = 56 px.** 32-px-Avatar ist die kleinste Größe mit erkennbarem Streamer-Gesicht auf typischen Monitoren. 48 px unter Erkennbarkeitsschwelle; 64 px frisst ~5 % Viewport-Breite ohne Mehrwert.
  - **TV-A5 — hero-slot-height = 280 px.** 31 % Viewport — billboard-y ohne dominant. Kombiniert mit TV-A1 (72) ergibt N_max-hero = 3 (akzeptabler Lane-Verlust gegenüber 5). 320 px würde N_max-hero auf 2 drücken und Lane-Browse bei aktivem Hero unverhältnismäßig einschränken.
  - **TV-A6 — Hero-tie-break: most-recently-started.** Nutzt existierendes `streamers.last_live_at` aus DB-Schema, kein neuer IPC-Endpoint. Matches „Happening now"-Mental-Model. Alternative most-watched-by-user braucht IPC-Aggregation — unnötige Wave-4-Scope-Erweiterung für marginalen Semantik-Gewinn.
  - **TV-A7 — Live-state indicator: `#d4a14a` Accent-Dot.** Brand-Touchpoint: Sightline-Honiggold wird in der Sidebar dauerhaft an Live-Avataren sichtbar. H-07-konform als identity-bearing discrete marker. Coverage minimal (64 px² pro Dot × typische 5–10 sichtbare Streamer = vernachlässigbar im 5 %-Viewport-Budget aus TV1).
- **ADR-Patches:** Status-Header auf „Accepted 2026-05-12" + Acceptance-Note mit TV-A1..A7-Mapping; TV1/TV4/TV9/TV10 Decision-Body-Paragraphen und Tabellen-Source-Spalten von TV-Q-Refs auf TV-A-Binding-Refs umgestellt (keine Substanz-Änderung der Werte); CSS-Custom-Properties-Block-Kommentare auf TV-A-Refs aktualisiert; Alternatives-Considered-Sektionen mit aktualisierten TV-A-Refs; Consequences > Costs accepted #1 + #6 und Neutral #4 auf TV-A-Refs umgestellt; „Open Questions for CEO" zu „CEO Decisions (2026-05-12)" umbenannt mit pro Decision `### TV-AN`-Block (Original-Frage / Decision-Statement / Begründung / Folge); „Escalation note for CTO" zu „resolved 2026-05-12" umgestellt mit Resolution-Block (CTO-Forward-as-is-Wahl + Begründung + Pattern-Lehre).
- **Counting-Bug-Fix global.** Mission-Brief-Counting-Bug („six open questions" während tatsächlich sieben TV-Q1..TV-Q7 gelistet waren) gefangen vom Pre-Sign-off-CTO-Audit und global gefixt: ADR Open-Questions-Intro „six" → „seven"; Threshold „(6) is above" → „(7) is above"; CEO-Decisions-Intro durchgehend „seven"; Escalation-Note-Counting-Absatz auf saubere „This ADR surfaced seven Open Questions ... 7 > 5"-Formulierung gekürzt. Decision-Log §1.0-Intro / §6 Audit-Methode-Schluss + Audit-Ergebnis / Pattern-Lehre-Notiz auf „Sieben" geupdated; Versionshistorie-Counting auf 7.
- **TV-A1 × TV-A5 Compound-Risk-Notation.** Neuer Consequences > Risks #6-Eintrag dokumentiert die N_max-Compound-Wechselwirkung der zwei akzeptierten Werte: bei `lane=72` + `hero=280` resolves N_max-Ladder auf 5 / 3; hypothetische `lane=60+hero=240` → 6 / 4; `lane=80+hero=320` → 4 / 2. Mitigation: joint-evaluation-requirement für künftige Re-Tunes; v2.x-ADR-Hinweis für joint-Revisit basierend auf Real-Use-Heatmap.
- **Decision-Log-Nachtrag** in `docs/decision-log/v2.1-adr-0040-visual-system-v2.md` §8 „CEO Sign-off — 2026-05-12 — Final Acceptance Decisions (Block)" mit Intro-Notiz (CTO-Forward-as-is-Wahl, Block-Acceptance erprobtes Pattern aus Welle 1+2, sieben Decisions ohne Substanz-Change), pro TV-A1..A7 Decision-Statement / Begründung (CTO-Lesart) / Folge für ADR, plus Schluss-Notiz zur Komplementarität-Pattern-Lehre der zwei Audit-Disziplinen (Engineering-Anti-Smell-Audit Pre-Delivery / Pre-Sign-off-CTO-Audit) und Verweis auf bevorstehende R-ADR-Rules-Erweiterungs-Welle. §4 AC-Tabelle um 14 ACCEPTED-Patch-Items erweitert; Versionshistorie um Acceptance-Eintrag ergänzt.
- **CLAUDE.md** Active-Decisions-Register: ADR-0040-Eintrag von „PROPOSED — awaiting CEO sign-off on six Open Questions" auf „ACCEPTED 2026-05-12 with seven CEO refinements (TV-A1 lane=72, TV-A2 thresholds 160/96, TV-A3 zoom 1.10, TV-A4 sidebar 56, TV-A5 hero 280, TV-A6 tie-break most-recently-started, TV-A7 live-dot accent)". Plus Hinweis: „v2.1-ADR-Phase strukturell komplett bis auf Multiview-Pane-Expansion-ADR — parallele Welle, eigener Sign-off-Zyklus."
- **STATE.md** System-Ampel-Zeile und Ampel-Begründung referenzieren Welle 3 ACCEPTED + v2.1-ADR-Phase strukturell komplett bis auf Multiview-Pane-Expansion-ADR; Roadmap aktualisiert; Änderungshistorie-Eintrag 2026-05-12 mit Verweis auf decision-log §8.
- **Keine Produkt-Code-Änderungen.** Keine Token-Migration im Code (Wave 4 implementiert). Keine `globals.css`-Edits. Keine `tailwind.config.*`-Erstellung. Keine `lucide-react`-Installation. Keine `src-tauri/`-Touches. Keine TV-Decision-Substanz berührt — reiner Status-Konvertierungs-Patch plus zwei dokumentarische Findings.

**Pattern:**
- **Welle-3-Acceptance demonstriert: Forward-as-is (Option 1) für 7-OQ-Block ist tragfähig wenn jede OQ mit klarer CTO-Empfehlung kommt.** Der CTO wählte Option 1 über Option 3 (Subdivide), weil die tuning-vs-structural-Klassifikation bei Scrutiny unscharf war: TV-Q7 ist Brand-Touchpoint (nicht Tuning), TV-Q2 ist user-feel-prägend durch Compact/Sliver-Threshold-Impact (nicht klar structural). Block-Acceptance-Pattern hat sich in Welle 1 (4 CEO-Decisions), Welle 2 (3 CEO-Decisions), Welle 3 (7 CEO-Decisions) als linear-skalierende Praxis bewährt. Pre-process hätte eine dritte „CTO-approved engineering defaults"-Kategorie außerhalb R-ADR-Rule-Binarität (engineering-default-with-rationale vs. CEO-decision) eingeführt — bewusst vermieden.
- **Pre-Sign-off-CTO-Audit ist komplementär zum Engineering-Anti-Smell-Audit.** Welle 3 hat zum ersten Mal sichtbar gemacht, dass die zwei Audit-Disziplinen unterschiedliche Failure-Modes adressieren. Der Engineering-Anti-Smell-Audit (vor Lieferung, Engineering-Wahl-Space-fokussiert) hat 7 OQs identifiziert + 11 Sub-Decisions als Engineering-Defaults dokumentiert + den Spec-Bug im Mission-Brief gefangen (50% → 85% Focus-Outline gegen Anker statt gegen Brief geprüft). Der Pre-Sign-off-CTO-Audit (vor Sign-off, documentary-Konsistenz- und cross-cutting-Risk-fokussiert) hat zwei Findings gefangen, die der Engineering-Audit aufgrund Mission-Brief-Fokus nicht sah: (1) Counting-Bug „six/sechs" während sieben TV-Q1..TV-Q7 gelistet waren — Konsistenz-Issue, kein Engineering-Wahl-Punkt; (2) fehlende explizite TV-A1×A5 N_max-Compound-Risk-Notation — cross-Decision-Implikation, die jeder einzelne TV-Audit nicht sah. Pattern-Lehre: beide Audit-Typen werden in der R-ADR-Rules-Datei nach diesem Patch als separate Rules verankert.
- **Anti-Smell-Engineering-Audit hat im Token-Pinning-Welle einen Spec-Bug im Mission-Brief gefangen.** Welle 3 demonstrierte zum ersten Mal, dass der Anti-Smell-Audit nicht nur Open-Question-Discovery ist, sondern auch Spec-Source-Verification. Der Mission-Brief hatte „Focus-Outline (white at 50 % per H-20)" geschrieben, was nicht dem actual H-20 entspricht (80–90 % opacity binding). Engineering-Audit verifizierte gegen Anker-Quelle, korrigierte den Wert auf 85 % (mid of 80–90). Pattern-Lehre wandert in R-ADR-01-Rule-Wortlaut: „Anti-Smell-Audit muss gegen Anker-Quelle verifizieren, nicht gegen Mission-Brief-Reproduktion."

**Warum die Notiz:**
v2.1-ADR-Phase strukturell vollständig nach Welle 3; nur Multiview-Pane-Expansion-ADR (CEO-A1-Folge aus ADR-0038, eigene parallele Welle) zwischen ADR-Phase-Abschluss und Wave-4-Engineering. Pattern-Erfassung für Rules-Erweiterungs-Welle nach diesem Patch: Engineering-Anti-Smell-Audit + Pre-Sign-off-CTO-Audit als separate komplementäre Rules; Spec-Source-Verification als Audit-Disziplin-Wortlaut; Block-Acceptance-Pattern-Skalierung dokumentiert (1 → 3 → 4 → 7 CEO-Decisions Welle-übergreifend).

**Referenz:**
- ADR-Patch: `docs/adr/0040-visual-system-v2.md` Status-Header (Acceptance-Block neu); TV1 Live-state-indicator-Paragraph (TV-A7-Binding) + `--color-live`-CSS-Kommentar; TV4 Decision-Statement + Card-Geometry-Tabelle (TV-A1/A2/A3-Bindings); TV9 Decision-Statement + Sidebar-Tokens-Tabelle (TV-A4-Binding + TV-A7-Live-Dot); TV10 Decision-Statement + Hero-Slot-Tokens-Tabelle + Tie-Break-Paragraph (TV-A5/A6-Bindings); CSS-Block-Kommentare (alle Section-Headers aktualisiert); Consequences > Costs accepted #1, #6 + Neutral #4 + neue Risks #6 (TV-A1×A5 Compound); Alternatives-Considered TV4/TV6/TV9/TV10 (TV-Q→TV-A Refs); CEO Decisions (2026-05-12)-Sektion (Open Questions umbenannt); Escalation note for CTO (resolved 2026-05-12)-Sektion.
- Decision-Log-Nachtrag: `docs/decision-log/v2.1-adr-0040-visual-system-v2.md` §8 CEO Sign-off; §4 AC-Tabelle-Erweiterung um 14 ACCEPTED-Patch-Items; §6 + §1.0-Intro + Versionshistorie Counting-Fix.
- §7 Spec-Korrektur in-flight (Focus-Outline 50% → 85%) bleibt als historisches Engineering-Audit-Finding stehen.
- Aktive Entscheidung in CLAUDE.md-Register (Status PROPOSED → ACCEPTED 2026-05-12).
- STATE-Eintrag: `docs/STATE.md` System-Ampel-Zeile, Ampel-Begründung, Roadmap, Änderungshistorie 2026-05-12.
- CEO-Direktive im CTO-Chat, 2026-05-12 (TV-A1 bis TV-A7 als Block).

---

## 2026-05-12 — v2.1-ADR-Phase Welle 3 — ADR-0040 Visual System v2 entworfen

**Was:**
- `docs/adr/0040-visual-system-v2.md` ausgeliefert mit Status PROPOSED. Zwölf Token-Decisions (TV1 Color-System: 4-Tier Surface-Ladder bg/S1/S2/S3 mit L\*-Steps 4/7/10/13 + 2-Tier Text-Ramp ohne tertiary grey + `#d4a14a` Locked-Input Akzent + Coverage-Math-Verifikation 1.72 % viewport-level; TV2 Typography: System-Font-Stack + 4-Weight-Ladder 400/500/600/700 + 7-Size-Ladder 12-32 px; TV3 Spacing: 4-px-Base mit 10-Step-Ladder; TV4 Card-Geometry: lane_height_px=72 + width-thresholds 160/96 px + corner-radius 4 px + hover-zoom 1.10; TV5 Card-Element-Numerics: 16-px Checkmark + 20-px Retention-Badge + 3-px Stripe + 2-px Focus-Outline; TV6 Motion-Constants: 800-ms Hover-Delay + 200-ms Zoom + 250-ms Crossfade + 400-ms Frame-Cadence + `cubic-bezier(0.16, 1, 0.3, 1)` Easing + complete Reduced-Motion-Override-Block; TV7 Elevation: keine Card-Shadows + 2-px-Blur Modal-Shadow + 11-Layer-Z-Index-Stack; TV8 Now-Indicator: 1-px white at 30 % opacity per ADR-0038-D3; TV9 Sidebar: 56-px collapsed / 240-px expanded + Accent-Live-Dot; TV10 Hero-Slot: 280 px Height + total-collapse-on-no-favourited-live + most-recently-started Tie-Break; TV11 Icon-System: Lucide-react install in Wave 4 + 4-Size-Ladder; TV12 Implementation: Tailwind v4 native `@theme {}` Pattern + 16-Item Migration-Liste).
- Decision-Log-Begleit-Eintrag `docs/decision-log/v2.1-adr-0040-visual-system-v2.md` mit pro TV-Decision: gewählter Wert + Fallback-Wert + Reihenfolge-Begründung. AC-Vollständigkeits-Tabelle (21 ACs grün); Scope-Compliance-Notiz; Domain-Knowledge-Gap-Audit (Tailwind-v4-Utility-Generation, APCA-Kontrast-Math, Lucide-Bundle-Size, prefers-color-scheme-Compat, OS-Font-Rendering-Edge-Cases — alle als Engineering-Validation-Punkte oder verified-via-spec); §6 Anti-Smell-Audit mit explizitem Test pro Sub-Decision; §7 Spec-Korrektur in-flight (Focus-Outline-Opacity 50% → 85% per actual H-20 binding 80–90 %).
- **Sechs Open Questions for CEO im PROPOSED-ADR:** TV-Q1 (lane_height_px: 60/72/80/96), TV-Q2 (card-width-thresholds: 160/96 vs. alternatives), TV-Q3 (hover-zoom-factor: 1.08/1.10/1.12/1.20), TV-Q4 (sidebar-collapsed-width: 48/56/64), TV-Q5 (hero-slot-height: 240/280/320), TV-Q6 (hero-tie-break: most-recently-started vs. most-watched vs. user-pinned), TV-Q7 (live-dot-color: accent vs. neutral white). Engineering-Defaults dokumentiert mit Alternatives-Tabellen.
- **Escalation-Note für CTO als formaler Forward-Process-Surface ("6 > 5"-Threshold der Mission-Direktive).** Drei Optionen dokumentiert: (1) Forward-as-is (alle 6 OQs zum CEO), (2) Pre-process (CTO picks defaults als "CTO-approved engineering defaults"), (3) Subdivide (CTO escaliert die 2 structural OQs TV-Q1 + TV-Q5 zum CEO, defaultet die 4 tuning OQs TV-Q3/Q4/Q6/Q7 engineering-side). Choice ist CTO-Privileg per R-SC-01 spirit.
- **Anti-Smell-Audit Spec-Bug entdeckt im Schreiben.** Mission-Brief sagte "Focus-Outline (white at 50 % per H-20)", aber actual H-20 specifiziert "white (or 80–90 % opacity white)". Spec-Korrektur in-flight applied: `--color-outline-focus` und `--color-outline-selection` auf `rgba(255, 255, 255, 0.85)` (mid of 80–90 %), nicht 0.50. Hover-Helper `--color-outline-hover: 0.30` ist Sightline-specific (outside H-20-domain) und bleibt. Anti-Smell-Lehre: Audits müssen gegen Anker-Quelle verifizieren, nicht gegen mission-Brief-Reproduktion.
- **Vollständiger CSS-Custom-Properties-Block** in Implementation-Notes (~150 Zeilen) mit allen 13 Token-Gruppen + `@layer base` mit dark-only + `@media (prefers-reduced-motion: reduce)` Override-Block. Plus Tailwind v4 Binding Notes + 10-Step Migration-Summary-Tabelle + Test-Strategie-Hints (Token-Snapshot / APCA-Contrast / Reduced-Motion-Override / Visual-Regression).
- CLAUDE.md „Active decisions"-Register um ADR-0040 (PROPOSED) ergänzt; STATE.md Änderungshistorie + Ampel grün (Welle 3 PROPOSED ändert nichts an System-Ampel; Real-Use-Phase läuft parallel weiter).
- Keine Produkt-Code-Änderungen. Keine `globals.css`-Edits. Keine `tailwind.config.*`-Erstellung. Keine `lucide-react`-Installation. Keine Komponenten-Migration. Keine `src-tauri/`-Touches. Keine Anker-Heuristik-Erweiterungen. Keine Re-Litigation von ADR-0038/0039-Decisions. Token-Migration und Hard-Coded-Hex-Audit sind Wave-4-Engineering-Scope.

**Pattern:**
- **Anti-Smell-Audit als Wave-3-Pflicht hat einen Spec-Bug entdeckt.** Welle 1 hatte 3 versteckte Sub-Decisions (CEO-A1/A3/A4 als post-sign-off-Promotions), Welle 2 hatte 3 ehrliche OQs nach präventivem Audit, Welle 3 hat 6 OQs **plus** einen entdeckten Spec-Bug (Focus-Outline-Opacity-Mismatch zwischen Mission-Brief und Anker). Pattern-Lehre: der Anti-Smell-Audit ist nicht nur Open-Question-Discovery, er ist auch Spec-Source-Verification. Mission-Briefs können Spec-Fehler enthalten; der Audit muss gegen die Anker-Quelle (`docs/reference/visual-language-netflix.md`) verifizieren, nicht gegen die Brief-Reproduktion. Dieser Pattern-Pin wird in §6 + §7 des decision-logs dokumentiert für künftige Token-pinning-ADRs.
- **6-OQ-Threshold mit Subdivide-Vorschlag ergänzt.** Welle 2 hatte 3 OQs (unter Threshold); Welle 3 hat 6 OQs (über Threshold). Der CTO-Forward-Process-Option-Set wird durch eine konkrete Subdivide-Empfehlung erweitert: structural OQs (TV-Q1 lane-height, TV-Q5 hero-height — beide mit N_max-Math-Impact) zum CEO escalieren, tuning OQs (TV-Q3/Q4/Q6/Q7 — alle single-token-tunable ohne architectural impact) als engineering-default akzeptieren. Pattern: Subdivide ist eine legitime Option wenn OQs in zwei klare Risk-Tiers fallen (structural vs. tuning).
- **Finalisierung aller Forward-References aus Welle 1 + Welle 2.** ADR-0038 D2/D3/D4/D7/D9 Forward-References (lane_height_px, Now-Indicator-Visual, Selection-Checkmark-Size, Hero-Trigger) sind hier konkret gepinnt. ADR-0039 DV1–DV10 Forward-References (card-corner-radius, image-share, hover-zoom, retention-badge-size, avatar-size, stripe-height, focus-outline-width, silhouette-offset, fan-out-gap, etc.) sind hier konkret gepinnt. Nach ACCEPTED ist die v2.1-ADR-Phase strukturell vollständig — nur die parallele Multiview-Pane-Expansion-ADR (CEO-A1-Folge) steht noch zwischen ADR-Phase und Wave-4-Engineering. Pattern: 3-Welle-Sequenz (Pattern → Element → Tokens) hat sich als skalierbare Spec-Discipline bewährt.

**Warum die Notiz:**
Dokumentiert die finale v2.1-ADR-Lieferung. Welle-1+2-Anti-Smell-Audit-Pattern wurde aktiv angewandt und hat zusätzlich einen Mission-Brief-Spec-Bug aufgedeckt — saubere Spec-Korrektur in-flight ohne Engineering-Decision-Erfindung. Sechs OQs an der Threshold-Schwelle als formal-escalation-surface adressiert. Welle 1 (ACCEPTED), Welle 2 (ACCEPTED), Welle 3 (PROPOSED — awaiting sign-off) plus die parallele Multiview-Pane-Expansion-ADR vervollständigen die v2.1-ADR-Phase; nach Sign-off kann Wave-4-Engineering-Implementation starten.

**Referenz:**
- ADR: `docs/adr/0040-visual-system-v2.md`
- Decision-Log: `docs/decision-log/v2.1-adr-0040-visual-system-v2.md`
- Spec-Quellen: `docs/reference/visual-language-netflix.md` H-01..H-25 + §1–§7 + Locked Input (`#d4a14a`)
- Forward-Sources: ADR-0038 D2/D3/D4/D7/D9 (Welle 1, ACCEPTED 2026-05-12); ADR-0039 DV1–DV10 + VOd-A2/A3/A5 (Welle 2, ACCEPTED 2026-05-12)
- STATE-Eintrag: `docs/STATE.md` Änderungshistorie 2026-05-12 (Welle 3 PROPOSED)
- Aktive Entscheidung im CLAUDE.md-Register (ADR-0040 PROPOSED — awaiting CEO sign-off auf TV-Q1..TV-Q7)

---

## 2026-05-12 — ADR-0039 VodCard ACCEPTED — drei CEO-Refinements als Block

**Was:**
- ADR-0039 vom CEO am 2026-05-12 mit drei finalen Decisions abgenommen und damit auf Status **ACCEPTED** umgestellt. Die drei Decisions:
  - **VOd-A2 — Retention-Source → Client-Side-Compute.** Die VodCard berechnet die Retention-Restzeit client-seitig aus `streamer.broadcasterType` + `vod.streamStartedAt`; das Backend-`retention_at`-STORED-Spalten-Add wandert in v2.2-Backlog mit explizitem Trigger Real-Use-Drift-Reports. Begründung: Tier-Promotion-Frequenz pro Streamer ist niedrig; Drift ist konservativ-falsch (Karte zeigt weniger Retention als tatsächlich verfügbar — kein User-Schaden); Backend-Touch in v2.1 würde ADR-0038-D8-No-Backend-Touch-Policy unnötig brechen.
  - **VOd-A3 — Completed-Watch-Indicator → Option (b).** Bei `watchedFraction === 1` rendert die Karte einen kleinen neutralen Checkmark-Glyph in der Metadata-Strip Line 2 Column 2, right-aligned. Begründung: (a) „nichts" verliert Information für die VOD-Aggregator-Kernfunktion (Watched-vs-Unwatched beim Scrollen erkennbar); (c) Accent-Checkmark im Image-Overlay kollidiert räumlich mit Selection-Checkmark-Position und ist H-07-borderline; (d) Image-Scrim ist H-15-borderline und reduziert Thumbnail-Lesbarkeit. (b) ist anchor-compliant, subtil und funktional. Wave-3-Verfeinerungs-Pfad explizit reserviert: ADR-VisualSystem-v2 kann eine optionale Thumbnail-Opacity-Reduzierung auf ~75–80 % als zusätzliches Watched-Signal hinzufügen, falls Real-Use-Feedback einen „every-card-has-checkmark"-Background-Noise-Pattern zeigt — Single-Line-CSS-Change, kein ADR-Re-Open.
  - **VOd-A5 — Stack-Card-Fan-Out-Edge-Trajectory → Auto-Mirror-Left.** Bei Right-Edge-Proximity mirroreriert die Fan-Out-Trajektorie automatisch nach lateral-left. Begründung: Smart-Direction ist Standard-Pattern für Edge-Adaptive-UI-Elemente (Dropdowns, Tooltips); User lernen es in einer Interaktion. „Right cards open right"-Erwartung an Edge-Cards zu brechen ist der akzeptable Preis dafür, dass die Affordance an der Edge überhaupt funktioniert. Alternative Suppress macht Edge-Karten behaviour-divergent; Alternative Vertical ist Layout-Collision mit ADR-0038-D4-Lane-Packing.
- **ADR-Patches:** Status-Header auf „Accepted 2026-05-12" + Acceptance-Note mit VOd-A2/A3/A5-Mapping; DV5/DV6/DV4 Decision-Body-Paragraphen von OQ-Sprache auf Binding-Decision-Sprache umgestellt (keine Substanz-Änderung); DV7-State-Matrix-Row, DV8-Cache-Invalidation-Tabelle und Alternatives-Considered-Sektionen mit aktualisierten Decision-Referenzen; Consequences > Costs-accepted #2/#4, Risks #6 und Neutral #8 auf VOd-A2/A3/A5 umgestellt; Risks #6 enthält jetzt expliziten Wave-3-thumbnail-opacity-reduction-Mitigation-Pfad als ADR-VisualSystem-v2-Single-Line-CSS-Change-Follow-up; „Open Questions for CEO" zu „CEO Decisions (2026-05-12)" umbenannt mit pro Decision `### VOd-AN`-Block (Original-Frage / Decision-Statement / Begründung / Folge); „Escalation note for CTO" zu „Escalation note for CTO (resolved 2026-05-12)" umbenannt mit Resolution-Block (CTO-Forward-as-is / CEO-Block-Acceptance / Anti-Smell-Pattern-Konfirmation).
- **Decision-Log-Nachtrag** in `docs/decision-log/v2.1-adr-0039-vodcard.md` §7 „CEO Sign-off — 2026-05-12 — Final Acceptance Decisions" mit pro-VOd-A-Decision Statement (1 Satz) / Begründung (CTO-Lesart geglättet, 2–3 Sätze) / Folge für ADR-0039 (DV-Nummer und Sektion) plus Schluss-Notiz zur CTO-Forward-as-is-Pfad-Bestätigung und Anti-Smell-Audit-Post-Sign-off-Verifikation. §4 AC-Tabelle um acht ACCEPTED-Patch-Items erweitert; Versionshistorie um Acceptance-Eintrag ergänzt.
- **CLAUDE.md** Active-Decisions-Register: ADR-0039-Eintrag von „PROPOSED — awaiting CEO sign-off" auf „ACCEPTED 2026-05-12 with three CEO refinements (VOd-A2 client-compute, VOd-A3 completed-checkmark, VOd-A5 auto-mirror-left)".
- **STATE.md** System-Ampel-Zeile und Ampel-Begründung referenzieren Welle 2 ACCEPTED + Welle 3 (ADR-VisualSystem-v2) als nächsten Schritt; Roadmap-Eintrag aktualisiert; Änderungshistorie-Eintrag 2026-05-12 mit Verweis auf decision-log §7.
- **Keine Produkt-Code-Änderungen.** Keine Visual-Token. Keine DV-Decision-Substanz berührt. Keine `src/` / `src-tauri/` / `globals.css` / Migrations / IPC-Edits. Keine Re-Litigation außerhalb des Status-Bezug-Updates.

**Pattern:**
- **Anti-Smell-Audit aus Welle 1 hat in Welle 2 präventiv getragen — Acceptance ohne Substanz-Patch.** Welle 1 hatte drei versteckte Sub-Decisions (Multiview-Pane-Count, Hero-Trigger-favourited, Q-6-Cascade), die als CEO-A1/A3/A4 nachgezogen werden mussten — der ADR-Patch war eine echte Hedge-Promotion plus Substanz-Korrektur (CEO-A2 75 → 50 %). Welle 2 hat den Anti-Smell-Audit präventiv im PROPOSED-Run angewandt: drei genuine CEO-relevante OQs identifiziert (VOd-Q2/Q3/Q5), elf Sub-Decisions als Engineering-Defaults mit Begründung dokumentiert, keine versteckten Hedges. Resultat: CEO-Block-Acceptance ohne Substanz-Änderung. Der Acceptance-Patch ist reine Status- und OQ-Surface-Konvertierung — kein DV-Decision-Statement berührt. Das ist die positive Bestätigung des Welle-1-Lehrens.
- **CTO-Forward-as-is-Pfad (Option 1) funktionierte als Sign-off-Pfad.** Die PROPOSED-Lieferung bot dem CTO drei Forward-Process-Options (forward-as-is / pre-process / subdivide). Der CTO hat Option 1 gewählt — drei OQs unverändert an den CEO weitergegeben — und der CEO hat alle drei Engineering-Defaults als Block akzeptiert. Pattern: ein gut auditierter 3-OQ-PROPOSED-ADR braucht kein Pre-Processing; die Forward-Process-Options sind ein Sicherheitsnetz, nicht eine zwingende Vorab-Verarbeitung. Welle 3 (ADR-VisualSystem-v2) folgt demselben Schema.
- **Wave-3-Refinement-Pfad explizit reserviert ohne Re-Litigation.** VOd-A3 dokumentiert eine zukünftige Visual-Token-Verfeinerung (Thumbnail-Opacity-Reduzierung auf ~75–80 %) als Wave-3-Single-Line-CSS-Change-Follow-up. Pattern: ein Acceptance-Patch darf legitime zukünftige Verfeinerungs-Pfade benennen, solange sie keine ADR-Re-Litigation erfordern. Das ist die Inverse zur Welle-1-Lehre „Hedges-im-ADR sind eskalations-pflichtig" — explizite Wave-N+1-Refinement-Options sind okay, weil sie als reservierte Single-Layer-Optionen markiert sind und nicht als versteckte Sub-Decisions getarnt werden.

**Warum die Notiz:**
Dokumentiert die saubere Acceptance-Disziplin der zweiten v2.1-ADR-Welle, bestätigt das Wave-1-Anti-Smell-Audit-Pattern als skalierbare Disziplin (keine versteckten Sub-Decisions post-sign-off), und schließt v2.1-Welle 2 mit ACCEPTED-Stempel ab. Nächster Schritt: Welle 3 (ADR-VisualSystem-v2); parallel hängt die Multiview-Pane-Expansion-ADR (CEO-A1-Folge, eigener Prompt).

**Referenz:**
- ADR-Patch: `docs/adr/0039-vodcard-composition-hover-stack-retention.md` Status-Header (Acceptance-Block neu); DV4 Edge-of-viewport-mirror (VOd-A5-Binding); DV5 Computation source + Backend touch deliberately not in v2.1 (VOd-A2-Binding); DV6 Completed-state marker (VOd-A3-Binding) + Visibility-Tabelle-Row `completed`; DV7 State-Matrix `stack-fanned`-Row (VOd-A5-Ref); DV8 Composite-Hook-Deferral-Paragraph + Cache-Invalidation `watch:completed` + Alternatives-Considered DV4/DV6/DV8 (VOd-A-Refs); Consequences > Costs accepted #2 + #4 + Risks #6 (mit Wave-3-Mitigation-Pfad) + Neutral #8; CEO Decisions (2026-05-12)-Sektion (Open Questions umbenannt); Escalation note for CTO (resolved 2026-05-12)-Sektion.
- Decision-Log-Nachtrag: `docs/decision-log/v2.1-adr-0039-vodcard.md` §7 CEO Sign-off; §4 AC-Tabelle-Erweiterung um acht ACCEPTED-Patch-Items; Versionshistorie-Eintrag.
- Aktive Entscheidung in CLAUDE.md-Register (Status PROPOSED → ACCEPTED 2026-05-12).
- STATE-Eintrag: `docs/STATE.md` System-Ampel-Zeile, Ampel-Begründung, Roadmap, Änderungshistorie 2026-05-12.
- CEO-Direktive im CTO-Chat, 2026-05-12 (VOd-A2 bis VOd-A5).

---

## 2026-05-12 — v2.1-ADR-Phase Welle 2 — ADR-0039 VodCard entworfen

**Was:**
- `docs/adr/0039-vodcard-composition-hover-stack-retention.md` ausgeliefert mit Status PROPOSED. Zehn engineering-binding Decisions (DV1 Card-Geometry-Kontrakt: 16:9 Image ≥ 75 % + Strip ≤ 25 % + drei Width-Varianten Full / Compact / Sliver; DV2 Idle-State-Komposition mit Selection-Top-Right / Retention-Bottom-Right / Stripe-Bottom-Edge Position-Map; DV3 Hover-Preview-Sequenz mit 800 ms Default-Delay + CEO-A4-Cascade-Union-Semantik + Reduced-Motion-Path; DV4 Multi-Perspective-Stack mit 2 Silhouette-Layer + 4 Avatar-Dots-Max + lateral-right-Fan-Out + Auto-Mirror-Left an Right-Edge + ArrowRight-Cascade durch fanned Cards; DV5 Retention-Komposition mit 3-Stage Position-Migration vom Strip auf Image-Overlay ab 48h + Clock-Icon + Client-Side-Compute aus `broadcaster_type`; DV6 3-px Stripe am Image-Area-Bottom mit State-Machine-gated Visibility; DV7 vollständige State-Matrix (13 primary rows × Stack-Variant-Sub-Tabelle); DV8 Composite-Hook `useVodSummary` ohne neuen IPC-Endpoint + Field-Mapping-Tabelle + Cache-Invalidation auf existing events; DV9 Tab-Order lane-by-lane chronological-within + ARIA-Label-Format + Keyboard-Shortcuts-Tabelle; DV10 React.memo + Eager-Mount-CSS-Hidden-Frames + Singleton-Loop-Invariant + 400 ms Frame-Cadence v2.0.x-Precedent).
- Decision-Log-Begleit-Eintrag `docs/decision-log/v2.1-adr-0039-vodcard.md` mit pro DV-Decision: gewählte Option + Fallback-Option + Reihenfolge-Begründung. AC-Vollständigkeits-Tabelle (20 ACs grün); Scope-Compliance-Notiz; Domain-Knowledge-Gap-Audit (Twitch-GQL-Storyboard verified absent, Tauri-WebView-Frame-Cycle als Engineering-Validation, GTA-RP Late-Join + Retention-Tier-Drift als bounded Risks).
- **Anti-Smell-Audit explizit durchgeführt** (decision-log §6) nach Welle-1-Lehre: VOd-Q1 (GIF-Strip-Pre-Ingest) als Engineering Default begründet (existing infrastructure seit Phase 3 deployed); VOd-Q4 (Composite-Batch-Endpoint) als Engineering Default begründet (defer per ADR-0038 D8 precedent, v2.2-Trigger 200 ms-Reiß); zusätzlich geprüft: Avatar-Order, Hover-Delay-Konstante, Tab-Order-Strategy, Composite-Hook-Granularität, Frame-Cycle-Cadence, Click-During-Fanout-Hit-Target, Stack-Silhouette-Layer-Count, Avatar-Dots-Maximum, Retention-Badge-Icon-Choice, Reduced-Motion-Frame-Selection — alle als Engineering-Defaults oder Anchor-konstrainiert mit explicit Begründung dokumentiert.
- **Drei Open Questions for CEO im PROPOSED-ADR:** VOd-Q2 (Retention-Source backend `retention_at` vs. client-Compute aus `broadcaster_type`); VOd-Q3 (Completed-Watch-Indicator bei 100 % — Engineering-Empfehlung (b) neutral-checkmark-glyph im Strip, Alternativen (a) nichts / (c) Accent-Badge im Image-Top-Right / (d) Image-Overlay-Scrim); VOd-Q5 (Stack-Card-Fan-Out-Edge-Trajektorie — Engineering-Default Auto-Mirror-Left, Alternativen vertical-fan-out + suppress rejected).
- **Escalation-Note für CTO** als eigenständige Sektion zwischen Open Questions und References. Macht die "≥ 3 OQs trigger reached"-Schwelle explizit und bietet drei Forward-Process-Options (forward-as-is / pre-process / subdivide). Choice ist CTO-Privileg per R-SC-01 spirit.
- CLAUDE.md "Active decisions"-Register um ADR-0039 (PROPOSED) ergänzt; STATE.md Änderungshistorie + Ampel grün (Welle 2 PROPOSED ändert nichts an System-Ampel; Real-Use-Phase läuft parallel weiter).
- Keine Produkt-Code-Änderungen. Keine Visual-Token. Keine GIF-Pipeline-Implementation. Keine `src/` / `src-tauri/` / `globals.css` / Migrations / IPC-Edits. Keine Re-Litigation von ADR-0038-Decisions (D5/D7/D8/D9/CEO-A4 normativ accepted).

**Pattern:**
- **Anti-Smell-Audit als Wave-2-Pflicht hat getragen.** Welle 1 hatte drei versteckte Sub-Decisions (Multiview-Pane-Count, Hero-Trigger-favourited, Q-6-Cascade), die der CEO als CEO-A1/A3/A4 nachziehen musste. Welle 2 hat den Audit präventiv durchgeführt: jede DV-Decision UND jede beigesteuerte Sub-Decision wurde auf "user-sichtbar UND nicht durch Q-Decision/Heuristik determiniert"-Test geprüft. Ergebnis: drei genuine CEO-relevante Open Questions identifiziert (VOd-Q2/Q3/Q5), elf weitere Sub-Decisions als Engineering-Defaults mit explicit Begründung dokumentiert (Avatar-Order, Hover-Delay-Konstante, Tab-Order-Strategy, Composite-Hook-Granularität, Frame-Cycle-Cadence, Click-During-Fanout-Hit-Target, Stack-Silhouette-Layer-Count, Avatar-Dots-Maximum, Retention-Badge-Icon-Choice, Stack-Hover-Persistence, Reduced-Motion-Frame-Selection). Keine versteckten Sub-Decisions im PROPOSED-ADR mehr.
- **Existing-Infrastructure als Decision-Anker.** Q-3 spezifizierte "animated GIF strip" — das `extract_preview_frames`-Subsystem in `services/downloads.rs:985` ist seit Phase 3 deployed und produziert 6 PNG-Frames pro VOD; die v2.0.x Library-VodCard cycle-t bereits diese Frames bei 400 ms/Frame. ADR-0039 macht das *normativ* (DV8 + DV10): kein neuer IPC-Endpoint, keine neue ffmpeg-Invocation, kein neuer Storage-Bucket — die bytes sind auf disk, die ADR spezifiziert *wie* der Renderer sie sequenziert. Pattern: wenn eine Sub-Decision durch existing-Code de-facto entschieden ist UND der Anker / Q-Decision sie nicht aktiv challengt, normativ accept statt erneut zur Diskussion stellen.
- **Forward-References explizit gebounded an Welle 3.** Jede Stelle, wo ein Token / Pixel / Hex / exakter Wert nötig wäre, ist explizit als "delegated to ADR-VisualSystem-v2 (Wave 3)" markiert. Per-DV mindestens einmal; Consequences > Neutral listet 9 Forward-References. Wave 3 (ADR-VisualSystem-v2) bekommt damit ihre Scope direkt aus zwei ADRs (0038 + 0039) sichtbar, ohne Re-Discovery zu brauchen.
- **Three-OQ-Threshold mit Escalation-Note adressiert.** Mission-Direktive: "wenn ≥ 3 CEO-relevante Open Questions am Ende übrigbleiben, lieber an den CTO eskalieren". ADR enthält explicit Escalation-Note nach Open-Questions-Sektion. Der CTO kann zwischen forward-as-is / pre-process / subdivide wählen (R-SC-01 spirit). Damit ist die Drei-OQ-Lieferung nicht silent-ship sondern formaler escalation-surface.

**Warum die Notiz:**
Dokumentiert die zweite v2.1-ADR-Lieferung. Welle-1-Pattern-Lehre wurde aktiv angewandt: präventiver Anti-Smell-Audit verhindert versteckte Sub-Decisions im PROPOSED. Drei Open Questions sind genuine CEO-Decisions, nicht hide-spots — Escalation-Note macht den CTO-Forward-Process-Choice explizit. Welle 1 (ADR-0038 ACCEPTED) und Welle 2 (ADR-0039 PROPOSED) ziehen die v2.1-ADR-Phase strukturiert durch.

**Referenz:**
- ADR: `docs/adr/0039-vodcard-composition-hover-stack-retention.md`
- Decision-Log: `docs/decision-log/v2.1-adr-0039-vodcard.md`
- Spec-Quellen: `docs/reference/visual-language-netflix.md` §3.2 + §3.3 + §6.2 + §7-Q-3/Q-5/Q-7/Q-10 + Anker-Acceptance §1
- Forward-Source: ADR-0038 D5/D7/D8/D9/CEO-A4 (Welle 1, ACCEPTED 2026-05-12)
- STATE-Eintrag: `docs/STATE.md` Änderungshistorie 2026-05-12 (Welle 2 PROPOSED)
- Aktive Entscheidung im CLAUDE.md-Register (ADR-0039 PROPOSED — awaiting CEO sign-off)

---

## 2026-05-12 — ADR-0038 Timeline-Layout ACCEPTED — vier CEO-Refinements landen

**Was:**
- ADR-0038 vom CEO am 2026-05-12 mit vier finalen Refinements abgenommen und damit auf Status **ACCEPTED** umgestellt. Die vier Decisions:
  - **CEO-A1 — Multiview-Pane-Count → 4 Panes.** v2.1 expandiert Multiview von 2-Pane (ADR-0021 v1) auf 4-Pane; die Pane-Render-Logik wird in einer separaten **Multiview-Pane-Expansion-ADR** (eigene v2.1-Welle, parallel zu VodCard- und VisualSystem-v2-Welle) behandelt; ADR-0038 D5/D7 nehmen die 4-Pane-Capacity als Contract.
  - **CEO-A2 — Multi-Perspective-Threshold → 50 %.** D5-Substanz-Korrektur von 75 % auf 50 % Time-Overlap der kürzeren Session. Begründung: GTA-RP-Welt hat Late-Join als signature pattern (Streamer A 2 h in Szene, B kommt dazu); 75 % verliert die Mehrheit dieser Fälle, 50 % trifft sie. Lower-Bound-Risiko strukturell gedämpft durch Tracked-Set-Constraint und Stack-Silhouette-Card-Render.
  - **CEO-A3 — Hero-Slot Layout-Reservation Trigger → favourited (neue D9).** Hero-Slot ist layout-reserviert genau dann, wenn ≥ 1 favourited Streamer aktuell live ist; nicht-favourited Live-Übergänge nehmen einen einmaligen Layout-Shift in Kauf. Eigene Decision-Sektion D9 zwischen D8 und Consequences; ehemalige Mitigation in „Costs accepted #1" auf reinen Cross-Reference auf D9 reduziert.
  - **CEO-A4 — Q-6 Keyboard-Cascade bestätigt.** GIF-Strip läuft bei Tab-Focus identisch zu Mouse-Hover, mit derselben 0,6–1,0 s Delay (H-18); kein separater Space-Trigger. ADR-VodCard (Welle 2) benötigt keine eigene Q-6-Sektion — der Forward-Reference in ADR-0038 ist die finale Quelle.
- **ADR-Patches:** Status-Header auf „Accepted 2026-05-12" + Acceptance-Note mit CEO-A1..A4-Mapping; D5 Substanz (Decision-Statement, Rationale, Client-Filter) auf 50 %; D5 Multiview-pane-count-Sub-Absatz auf 4-Pane-Capacity ohne Hedge; D7 Decision-Paragraph und Selection-Toolbar-Enable-Rule mit CEO-A1-Referenz; D5 Alternatives-Considered restrukturiert (alter 50%-Reject entfällt, 90% behält Under-Groups + Lower-Bound-Hinweis, neuer 75%-Reject); neue D9-Sektion; Consequences > Costs accepted #1 auf D9-Cross-Reference reduziert; Consequences > Neutral > Q-6-Eintrag auf CEO-A4-Cascade umformuliert; Consequences > Neutral > Multiview-pane-count auf CEO-A1 + separate Multiview-Pane-Expansion-ADR umformuliert; D4 N_max-Tabelle und D8 Komponenten-Hierarchie-Kommentar verweisen auf D9; References > Related ADRs enthält Multiview-Pane-Expansion-ADR-Platzhalter.
- **Decision-Log-Nachtrag** in `docs/decision-log/v2.1-adr-0038-timeline-layout.md` §6 „CEO Sign-off — 2026-05-12 — Final Acceptance Decisions" mit pro-CEO-A-Decision Statement/Begründung/Folge plus Schluss-Notiz zur R-SC-konformen Acceptance-Disziplin. §1 D5 mit Amended-Notiz versehen; §4 AC-Tabelle um die vier CEO-Decisions und alle Konsistenz-Patches erweitert; §5 Audit-Anmerkung für Welle 2 mit CEO-A4-Nachtrag-Notiz versehen; Versionshistorie um Acceptance-Eintrag ergänzt.
- **CLAUDE.md** Active-Decisions-Register: ADR-0038-Eintrag von „PROPOSED — awaiting CEO sign-off" auf „ACCEPTED 2026-05-12".
- **STATE.md** System-Ampel-Zeile und Ampel-Begründung referenzieren Welle 1 ACCEPTED + Welle 2 (ADR-VodCard) als nächsten Schritt; Roadmap-Eintrag „v2.1-ADR-Phase ⏳ aktiv" ergänzt; Änderungshistorie-Eintrag 2026-05-12.
- **Keine Produkt-Code-Änderungen.** Keine Visual-Token. Kein VodCard-Hover-Behavior-Detail. Keine Multiview-Sync-Logik-Änderungen (Render-Mechanik landet in separater Multiview-Pane-Expansion-ADR, eigener Prompt).

**Pattern:**
- **Hedges-im-ADR sind eskalations-pflichtig, nicht engineering-default.** Drei der vier CEO-Refinements (CEO-A1 Multiview-Pane-Count-Hedge in D5; CEO-A3 Hero-Slot-Trigger als Costs-Accepted-Mitigation; CEO-A4 Q-6-Cascade als Engineering-Judgement in Forward-References) waren im PROPOSED-ADR als Engineering-Judgements oder Costs-Accepted-Mitigationen versteckt — in Wahrheit waren sie CEO-Sub-Decisions, die durch CEO-Sign-off bestätigt werden mussten. Der CTO-Pre-Sign-off-Audit hat sie aufgedeckt; die Acceptance-Sequenz hebt sie sauber auf die Decision-Ebene. Pattern für künftige v2.1-ADRs (Welle 2 ADR-VodCard, Welle 3 ADR-VisualSystem-v2, plus Multiview-Pane-Expansion-ADR): vor Sign-off Pre-Audit auf versteckte Sub-Decisions; In-Decision-Hedges („implementation-time either X or Y"), Costs-Accepted-Mitigationen mit substantiver Trigger-Wahl und Engineering-Judgement-Formulierungen an Stellen, wo eigentlich Spec-Constraints binding sind, sind die typischen Hide-Spots.
- **Substanz-Korrektur folgt demselben Acceptance-Pfad wie Hedge-Promotion.** CEO-A2 (Threshold 75 % → 50 %) ist keine versteckte Sub-Decision, sondern eine echte Substanz-Änderung am D5-Decision-Statement, die nur über CEO-Freigabe veränderbar ist. Beide Klassen — Hedge-Promotion und Substanz-Korrektur — gehen prozessual denselben Weg (CEO-Decision dokumentiert, ADR-Folge konsistent durchgezogen, decision-log §6 als Audit-Trail). Der ADR-Footprint unterscheidet sich (Substanz-Korrektur berührt Decision-Statement + Rationale + Alternatives-Considered; Hedge-Promotion berührt typischer ein einziges Sub-Paragraph plus eine eigene neue Decision-Sektion), aber die Acceptance-Disziplin ist dieselbe.
- **CTO-Pre-Sign-off-Audit als R-SC-Erweiterung.** R-SC-01 verlangt CEO-Freigabe für Scope-Reduktionen. Analog dazu sollten in PROPOSED-ADRs versteckte Sub-Decisions vor Sign-off als explizite Decisions identifiziert werden — gleicher Geist, andere Failure-Mode. Dieser Run etabliert das Pattern für die v2.1-ADR-Welle und wird in §6 des decision-logs als Pre-Audit-Pflicht dokumentiert.

**Warum die Notiz:**
Dokumentiert die saubere Acceptance-Disziplin der ersten v2.1-ADR-Welle, fixiert das Hedges-im-ADR-Pattern als künftige Lehre, und schließt v2.1-Welle 1 mit einem ACCEPTED-Stempel ab. Nächster Schritt: Welle 2 (ADR-VodCard); parallel die Multiview-Pane-Expansion-ADR (CEO-A1-Folge, eigener Prompt vom CTO).

**Referenz:**
- ADR-Patch: `docs/adr/0038-timeline-layout-single-time-axis.md` Status-Header, D4, D5, D7, neue D9, D8-Kommentar, Consequences (Costs accepted #1, Neutral #4, Neutral #5), References > Related ADRs.
- Decision-Log-Nachtrag: `docs/decision-log/v2.1-adr-0038-timeline-layout.md` §6 CEO Sign-off; §1 D5 Amended-Notiz; §4 AC-Tabelle-Erweiterung; §5 CEO-A4-Nachtrag; Versionshistorie-Eintrag.
- Aktive Entscheidung in CLAUDE.md-Register (Status PROPOSED → ACCEPTED 2026-05-12).
- STATE-Eintrag: `docs/STATE.md` System-Ampel-Zeile, Ampel-Begründung, Roadmap, Änderungshistorie 2026-05-12.
- CEO-Direktive im CTO-Chat, 2026-05-12 (CEO-A1 bis CEO-A4).

---

## 2026-05-12 — v2.1-ADR-Phase Welle 1 — ADR-0038 Timeline-Layout entworfen

**Was:**
- `docs/adr/0038-timeline-layout-single-time-axis.md` ausgeliefert mit Status PROPOSED. Acht engineering-binding Decisions (D1 Lane-Packing-Algorithmus: Greedy Best-Fit alternierend ober/unter mit Interval-Graph-Coloring-Fallback; D2 sechs diskrete Zoom-Levels 15 min–30 d, default 24 h, hybrid Continuous-During-Gesture; D3 Now-Indicator-Tick 10 s / 30 s je Zoom-Level, Auto-Scroll opt-in; D4 Hybrid Lane-Capacity mit Stack-Marker für Overflow; D5 Multi-Perspective-Grouping bei ≥ 75 % Time-Overlap; D6 DOM-Render mit Time-Window-Render-Buffer plus SVG-Escape-Hatch; D7 Multi-Selection via Shift/Cmd-Click mit Selection-Cap 4 + Accent-Checkmark-Badge; D8 zwei neue Zustand-Stores `timeline-view-store` + `timeline-multiselect-store`, Composite-Hook `useVodSummary`, kein Backend-Touch).
- Decision-Log-Begleit-Eintrag `docs/decision-log/v2.1-adr-0038-timeline-layout.md` mit pro D-Decision: gewählte Option + Fallback-Option + Reihenfolge-Begründung. AC-Vollständigkeits-Tabelle, Scope-Compliance-Notiz, Domain-Knowledge-Gap-Audit (Stream-Intervals-Last-Profile, Tauri-WebView-Performance, Multi-Perspective-Detection — alle als Engineering-Validation-Punkte mit Fallback-Pfaden, keine CEO-relevanten Open-Questions).
- CLAUDE.md "Active decisions"-Register um ADR-0038 ergänzt; STATE.md Änderungshistorie + Ampel grün; v2.0.3 Real-Use-Phase läuft parallel weiter — diese ADR-Arbeit blockiert nichts.
- Keine Produkt-Code-Änderungen. Keine Visual-Token. Kein VodCard-Hover-Behavior-Detail (delegiert an Welle 2). Keine Multiview-Sync-Logik-Änderungen (ADR-0023 normativ referenziert).

**Pattern:**
- **§3.6 als verbindliche Spec-Quelle trägt die ADR-Arbeit.** Bestätigt das Anker-First-Pattern aus dem 2026-05-12-Acceptance-Run: Single-Time-Axis-Pattern war fest gepinnt, die Q-1/Q-2/Q-4/Q-5/Q-10-Decisions waren binding, und die ADR konnte die strukturellen, algorithmischen, daten-layer-Decisions abschließen, **ohne** dass irgendwo eine Anker-Decision re-erfunden werden musste. Engineering-Judgements (D1 Best-Fit, D2 Zoom-Levels, D3 Tick-Cadence, D6 DOM-vs-SVG) sind Trade-offs innerhalb der Anker-Constraints, keine Frage an den CEO. Open-Questions-Sektion = "None. ADR ready for ACCEPTED."
- **Fallback-Option pro D-Decision dokumentiert.** R-SC-konformes Pattern: wenn Engineering bei Implementierung feststellt, dass Erstwahl nicht trägt, liefert der decision-log den nächsten Schritt ohne erneute Trade-off-Diskussion. Beispiele: D1 Greedy-Best-Fit-Fallback ist Interval-Graph-Coloring; D6 DOM-Fallback ist SVG; D8 Composite-Hook-Fallback ist Backend-Batch-Endpoint. Spart in der Implementation-Phase Token + Diskussion.
- **Forward-References explizit gebounded.** Wo D-Decisions Wave-2 (ADR-VodCard) oder Wave-3 (ADR-VisualSystem-v2) brushen, ist die ADR-0038-Decision normativ ("hover behaviour delegated; matches H-17/H-18/H-22"), nicht re-litigiert. Consequences-Sektion „Neutral" listet 5 Forward-References explizit, sodass die Folge-ADRs ihre Scope direkt sehen.

**Warum die Notiz:**
Dokumentiert die erste tatsächliche v2.1-ADR-Lieferung. Anker-First-Disziplin hat im Lauf bestätigt, dass die ADR-Welle Engineering-fertig ist ohne neue CEO-Decisions zu brauchen — exakt das Ziel, das Workforce-Erweiterung 2026-05-11 + Anker-Acceptance 2026-05-12 gesetzt hatten. Pattern bestätigt; künftige Welle-2/Welle-3-Runs folgen demselben Pfad.

**Referenz:**
- ADR: `docs/adr/0038-timeline-layout-single-time-axis.md`
- Decision-Log: `docs/decision-log/v2.1-adr-0038-timeline-layout.md`
- Spec-Quelle: `docs/reference/visual-language-netflix.md` §3.6 + §6.1 + §7
- Anker-Acceptance-Audit: `docs/decision-log/v2.1-anchor-acceptance.md`
- STATE-Eintrag: `docs/STATE.md` Änderungshistorie 2026-05-12 (Welle 1)
- Aktive Entscheidung im CLAUDE.md-Register

---

## 2026-05-12 — Anker-First-Disziplin bestätigt: CEO-Decisions Q-1 bis Q-10 angenommen

**Was:**
- CEO hat das Netflix-Anker-Dokument durchgearbeitet und alle zehn Open Questions Q-1 bis Q-10 als Block angenommen, plus eine strukturelle Korrektur an Q-4: Timeline ist nicht Modifier auf H-25 Horizontal Rail, sondern eigenständiges Pattern. Folge: neues Component-Pattern §3.6 "Single-Time-Axis (Sightline-specific)" im Anker zwischen §3.5 Horizontal Rail und §4 Motion System. H-25 wird in §6.1 TimelinePage gedropt mit Verweis auf §3.6.
- Anker-Dokument `docs/reference/visual-language-netflix.md` umgebaut: Sektion 7 von "Open Questions for the CEO" → "CEO Decisions (2026-05-12)" mit `### Q-N`-Block pro Decision (alte Frage / Decision / Folge für Anker). Mapping-Sektionen §6.1 TimelinePage / §6.2 VodCard / §6.3 MultiviewPage / §6.4 LibraryPage entsprechend resolved (Q-1 Avatar-Sidebar, Q-2 Conditional-Hero-bei-Live, Q-3 GIF-Strip statt Video → H-19 N/A, Q-5 Stack-Silhouette+Avatar-Dots+Fanout, Q-7 3-Stufen-Retention-Escalation, Q-8 Equal-Quadranten+Audio-Anker, Q-9 Compact-Storage-Strip, Q-10 Card-Progress-Stripe auf Timeline / Continue-Watching-Rail auf Library). Q-6 kollabiert sauber durch Q-3-Cascade (kein Video → keine Tab-vs-Hover-Trigger-Frage), dokumentiert ohne erfundene Decision.
- Audit-Trail in `docs/decision-log/v2.1-anchor-acceptance.md` mit AC-Vollständigkeits-Tabelle, expliziter Begründung jeder Decision wie vom CTO im Chat geliefert, der strukturellen Korrektur Q-4, und der ADR-Reihenfolge-Begründung.
- ADR-Reihenfolge bestätigt: ADR-Timeline-Layout zuerst (Pattern-Geometrie), dann ADR-VodCard (Element-Geometrie), dann ADR-VisualSystem-v2 (Token-Layer). Tokens folgen Layouts folgen Pattern.

**Pattern:**
- **Anker-First-Disziplin hat funktioniert.** Anker wurde am 2026-05-11 als prüfbare Spec geschrieben (25 H-NN-Heuristiken + 10 nummerierte Open-Questions). CEO arbeitete den Anker durch und lieferte Decisions als Block — nicht als Drip-by-Drip-Klarstellungen während Implementation. Decisions wurden am Anker dokumentiert, **bevor** ADRs starten. Das ist der Anti-Pattern-Bruch zu Phase 6/7/8: dort wurden Specs parallel zur Implementierung "gefunden", was Re-Reviews mit weiten Findings provozierte. Heute: Spec ist fest, ADR-Phase kann jetzt sauber starten.
- **Strukturelle Korrektur als legitime CTO-Operation.** Q-4 wurde nicht "umbeantwortet" sondern als falsch gestellt erkannt und durch ein neues Pattern (§3.6) ersetzt. Der Decision-Log dokumentiert diesen Move explizit als eigenständige Operation neben den Q-Decisions. Das ist wichtig für künftige Audits: nicht jede Anker-Frage hat eine "Option (a)/(b)/(c)" als Decision — manchmal ist die richtige Antwort "Frage war falsch gestellt, hier ist die korrigierte Struktur."
- **Cascade-Resolutions sauber dokumentieren statt Decisions erfinden.** Q-6 wurde durch Q-3-Cascade trivial (kein Video → keine Trigger-Differenzierung). Der Brief war explizit: keine Q-Decisions umdeuten oder verfeinern. Lösung: Q-6 als "resolved-by-cascade" markiert mit ausdrücklichem Hinweis im decision-log, dass ein Follow-up möglich ist, wenn der CEO ein explizites Keyboard-Statement nachreichen will.

**Warum:**
Dokumentiert, dass die am 2026-05-11 eingeführte Anker-First-Disziplin im ersten Lauf trägt. v2.1 startet aus einer Spec-First-Position, nicht aus einer Spec-During-Implementation-Position. Damit ist auch klar, dass die `@vision-compliance-reviewer`-Rolle (noch nicht angelegt) gegen einen vollständig-verbindlichen Anker grading-fähig sein wird, ohne dass die Reviewer-Definition ihrerseits "die Decisions noch festzurren" muss.

**Referenz:**
- Anker-Dokument: `docs/reference/visual-language-netflix.md` (Sektion 7 CEO Decisions + §3.6 Single-Time-Axis)
- Decision-Log: `docs/decision-log/v2.1-anchor-acceptance.md` (Sektionen 1–5 inkl. AC-Vollständigkeits-Tabelle und Scope-Compliance)
- STATE-Eintrag: `docs/STATE.md` Änderungshistorie 2026-05-12
- CEO-Direktive im CTO-Chat, 2026-05-12

---

## 2026-05-11 — Workforce-Erweiterung: `visual-research`-Subagent + Netflix-Anker

**Was:**
- Neuer Subagent `visual-research` unter `.claude/agents/visual-research.md`. Rolle: ein fuzzy Visual-Design-Anker ("wie Netflix", "wie Apple Music") in ein anker-prüfbares Referenzdokument unter `docs/reference/` übersetzen. Tools: nur Read/Write/Grep/Glob — kein Bash, kein Edit am Produkt-Code. Output ist ausschließlich ein Spec-Dokument mit nummerierten Heuristiken (Pass/Fail-Check pro Regel) plus Mapping auf die betroffenen Repo-Views.
- Erstanwendung: `docs/reference/visual-language-netflix.md`. 25 Heuristiken (H-01 bis H-25), Visual System (Color/Typo/Spacing/Elevation), 5 Component-Patterns (Hero, Thumbnail-Card, Hover-Preview, Detail-Overlay, Horizontal-Rail), Motion-System inkl. `prefers-reduced-motion`-Fallback, 12 Anti-Patterns. Sightline-Mapping mit Adopt/Modify/Drop pro Heuristik für `TimelinePage`, `VodCard`, `MultiviewPage`, `LibraryPage`, `SettingsPage`. 10 offene Q-N-Fragen für den CEO an den Konfliktstellen (Sidebar-Posture, Hover-Preview-Source ohne Trailer-Material, Multi-Perspektive-Indikator, Retention-Countdown, Multiview-Leader-Prominence, Storage-Outlook-Posture, Continue-Watching-Rail).
- Brand-Decision des CEO als Locked Input dokumentiert: Sightline-Akzent `#d4a14a` (warmes Amber/Honiggold) ersetzt das v2.0-`#7c3aed`. Akzent-Restraint-Heuristiken (H-06/H-07/H-08) gelten gegen die neue Farbe identisch.

**Pattern:**
- **"Anker first, Token später."** Heuristik-Spec ist die Quelle; ADRs (`Timeline-Layout`, `VOD-Card`, `Visual-System v2`) folgen erst nach CEO-Sign-off auf das Anker-Dokument. Token-Hex-Werte landen downstream in `globals.css`, gegen die Heuristiken gewählt — nicht im Anker.
- **Reviewer-Anker statt Reviewer-Geschmack.** Künftiger `@vision-compliance-reviewer` (bisher hypothetisch, K-Reviewer-Kette wird gegen dieses Dokument grading-fähig) prüft Pass/Fail pro `H-NN`. Visual-Drift wird damit ein dokumentierbares Finding statt eine subjektive Anmerkung.
- **Domain-Konflikt → Open-Question, nicht Annahme.** Wo Netflix' Sprache (kurze Filme, einzelne Perspektive, Kategorie-Achse) auf Sightlines Domain (8-Stunden-Streams, Multi-Perspektive, chronologische Achse, Retention-Countdowns) trifft und keine offensichtliche Übertragung existiert, wandert das in die nummerierte Q-Liste an den CEO — nicht in eine improvisierte Spec.

**Warum:**
Direkte Antwort auf den im STATE-Kompass impliziten Visual-Drift: das v2.0-Visual-System (`#7c3aed`-Akzent, generische Dark-SaaS-Karten, sichtbare Material-Shadows, Card-Border-Highlighting) entsprach nicht der CEO-Originalvorgabe "wie Netflix". Die K-Reviewer-Kette hat den Drift nicht gefangen, weil der Anker nirgends als prüfbare Referenz im Repo lag — beide Reviewer (`code-reviewer`, `security-reviewer`) prüfen gegen Repo-Konventionen und Sicherheits-Kategorien, keiner gegen einen Visual-Anker. Diese Lücke wird hier strukturell geschlossen: ein dediziertes Referenzdokument plus ein Subagent, der solche Anker künftig erzeugt, wenn der CEO einen Visual-Anker nennt, bevor die Implementierung beginnt.

**Referenz:**
- Agent-Definition: `.claude/agents/visual-research.md`
- Anker-Dokument: `docs/reference/visual-language-netflix.md`
- Decision-Log: `docs/decision-log/v2.1-visual-research.md`
- STATE-Eintrag: `docs/STATE.md` Änderungshistorie 2026-05-11
- CEO-Direktive im CTO-Chat, 2026-05-11

---

## 2026-04-26 — Drei Real-User-Bug-Discovery-Driven Hotfixes in Folge (v2.0.2 + v2.0.3)

**Was:**
v2.0.2 (Sidecar-Resolution) und v2.0.3 (Download-Engine-Wiring) waren beide getriggert durch Real-User-Bug-Reports vom CEO nach Install. Beide Bug-Klassen wären durch End-to-End-Tests vor Release fangbar gewesen — Sidecar-Path-Bug durch Bundle-Integration-Test, Settings-Wiring-Gap durch UI-zu-Backend-Spawn-E2E-Test.

**Pattern-Bestätigung — keine neuen Rules nötig:**
- STATE-getriebenes CTO-Onboarding stabil über 3 Hotfix-Sessions
- R-RC-01 Mid-Phase-Reviews fingen 5 P1-Findings in v2.0.3 ab (gleiche Größenordnung wie v2.0.1 mit 18 Findings über mehr ACs)
- R-RC-03 Cross-Subagent-Awareness fand jeweils 1 zusätzlichen Medium am End-of-Phase
- R-SC-01 Scope-Reduction-Approval: 0 unabgesprochene Defers in v2.0.2 + v2.0.3
- R-SC-03 Versions-Manifest-Konsistenz: alle Assets korrekt benamt — kein Versions-Manifest-Hotfix-PR mehr seit Phase 8
- Senior-Engineer-Merge-Konvention: Senior macht Merge + Tag, CEO gibt expliziten Go. Stabil seit v1.0.

**Was als E2E-Test-Lücke ins v2.1-Backlog wandert:**
- Playwright + Tauri-driver E2E (war schon seit Phase 8 im Backlog, jetzt empirisch belegt)
- Bundle-Integration-Tests pro OS sind seit v2.0.2 in CI — verhindert Sidecar-Klasse
- UI-zu-Backend-Wiring-Tests sind die nächste Lücke (verhindert Settings-Wiring-Klasse)

**Warum die Notiz:**
Dokumentiert das Pattern für künftige Workforce-Selbstreflexion: Wenn nächste Real-User-Discovery wieder eine Wiring-Klasse aufdeckt, ist das ein starkes Signal für eine v2.1-E2E-Test-Mission. Solange einzelne Bugs auftauchen, reicht der Hotfix-Loop. Drei in Folge wäre Drift.

**Referenz:**
- `docs/HANDOFF-2026-04-26-v2.0.2.md` und `docs/HANDOFF-2026-04-26-v2.0.3.md`
- ADR-0035 dokumentiert die Settings-Wiring-Lehre explizit
- v2.1-Backlog in `docs/STATE.md`

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
