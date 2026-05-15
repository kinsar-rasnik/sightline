# STATE — Sightline

**Letzte Aktualisierung:** 2026-05-12
**System-Ampel:** 🟢 Grün — v2.0.3 in Real-Use; **v2.1-ADR-Phase strukturell vollständig 2026-05-12** — ADR-0038/0039/0040/0041 alle ACCEPTED. Wave-4-Engineering-Implementation entblockt.

---

## Kompass

**Projekt:** Lokale Cross-Plattform-Desktop-App (Tauri 2 + Rust + React) zur Aggregation von Twitch-GTA-RP-VODs mehrerer Streamer auf einer chronologischen Multi-Perspektiven-Timeline. Open-Source, MIT, GitHub-distributiert.

**Repo:** github.com/kinsar-rasnik/sightline

**Status:** v2.0.3 live (github.com/kinsar-rasnik/sightline/releases/tag/v2.0.3). Drei kritische Download-Engine-Bugs aus Real-Use-Discovery behoben: Quality-Profile wird jetzt in yt-dlp-Format-Spec übersetzt (AC1), Concurrency-Cap wird respektiert (AC2), Per-VOD-Process-Lock verhindert Frag-Race-Conditions (AC3). Plus Tech-Spec-§8-Compliance: File-Logger mit Daily-Rotation aktiviert (AC4). Real-User-Validation läuft parallel.

**v2.1-ADR-Phase abgeschlossen 2026-05-12.** Vier ACCEPTED ADRs (0038 Timeline-Layout, 0039 VodCard, 0040 Visual System v2, 0041 Multiview-Pane-Expansion); R-ADR-Rules verankert (R-ADR-01..R-ADR-04 in `.claude/rules/adr-discipline.md`). Bereit für Wave-4-Engineering-Implementation.

**Roadmap:**
- Phase 1–7 ✅
- Phase 8 (Storage-Aware Distribution) ✅
- v2.0.1 Scope-Closure ✅
- v2.0.2 Critical Sidecar Hotfix + UX-Polish ✅
- v2.0.3 Critical Download-Engine Hotfix ✅
- **Real-Use-Phase ⏳ aktiv** — CEO validiert v2.0.3 im Alltag
- **v2.1-ADR-Phase ✅ abgeschlossen 2026-05-12** — vier ACCEPTED ADRs als Block-Acceptance-Sequenz (Welle 1 ADR-0038 Timeline-Layout / Welle 2 ADR-0039 VodCard / Welle 3 ADR-0040 Visual System v2 / parallele Welle ADR-0041 Multiview-Pane-Expansion), insgesamt 19 CEO-Decisions (4 + 3 + 7 + 5) sauber durchakzeptiert, alle Forward-as-is-Pfad. Plus R-ADR-Rules verankert in `.claude/rules/adr-discipline.md` (R-ADR-01 Engineering-Anti-Smell-Audit, R-ADR-02 Pre-Sign-off-CTO-Audit, R-ADR-03 Acceptance-Patch-Pattern mit Substanz- und reine-Status-Konvertierungs-Pfaden, R-ADR-04 Three-Forward-Process-Options bei ≥ 5 OQs). **Wave-4-Engineering-Implementation entblockt** — fünf Engineering-Missions: Visual-System-Migration (Tokens in `globals.css` + `lucide-react`-Install) / VodCard-Rewrite (`VodCard.tsx` gegen ADR-0039) / TimelinePage-Rewrite (`TimelinePage.tsx` + `TimelineHeroSlot.tsx` gegen ADR-0038) / Multiview-Pane-Expansion-Implementation (`MultiViewPage.tsx`/`MultiViewPane.tsx`/`sync-store.ts` gegen ADR-0041) / Integration + E2E + Polish. Geschätzt 10–20 Prompts insgesamt für Wave 4 + 2–4 Prompts für v2.1-Release danach.
- Bei Issue-Sammlung: v2.1 als Long-Run für UX/Refactor-Themen (Backlog gewachsen aus v2.0.3) — startet nach Abschluss aller v2.1-ADRs
- Post-v2.1: Auto-Issue-Triage-Workflow (eigenständiges Folge-Projekt)

---

## Ampel-Begründung

🟢 — v2.0.3 released, alle 5 CI-Jobs grün auf PR #25, Release-Workflow ~24min, 5 OS-Assets korrekt benamt (R-SC-03 ✅). Repo clean. **v2.1-ADR-Phase strukturell vollständig 2026-05-12** — ADR-0038/0039/0040/0041 alle ACCEPTED. Wave-4-Engineering-Implementation entblockt.

Pattern-Lehren über die vier Wellen kondensiert in R-ADR-01..R-ADR-04: Block-Acceptance-Pattern skaliert linear (4 → 3 → 7 → 5 CEO-Decisions); Engineering-Anti-Smell-Audit (Pre-Delivery) und Pre-Sign-off-CTO-Audit (Pre-Sign-off) komplementär; Spec-Source-Verification Pflicht-Klausel; Compound-Risk-Notation bei non-orthogonalen OQs; Forward-as-is bleibt Default-Pfad bei klaren Engineering-Empfehlungen. ADR-0041 als erste Vorwärts-Anwendung der R-ADR-Rules bestätigt das Pattern — keine versteckten Sub-Decisions post-Sign-off, sauberer Acceptance-Patch als reine Status-Konvertierung.

Ampel bleibt grün bis Wave-4-Start (Real-User-Validation läuft parallel). Wenn nach v2.0.3-Install eine der vier Validation-Punkte fehlschlägt → 🟡 + v2.0.4-Trigger. Wave-4-Start ist eigene Mission im neuen Chat-Kontext.

---

## IST-Zustand

**Repository:**
- Working-Directory: `~/Documents/sightline`
- Branch: `main`
- main HEAD: `18e307a` (PR #25 squash)
- Working Tree: doc-only-Änderungen für v2.1-ADR-Phase + R-ADR-Rules + ADR-0041-Acceptance (kein Code-Touch); commitbar in v2.1-ADR-Phasen-Abschluss-Commit.

**Tags:**
- `phase-1-complete` bis `phase-8-complete` ✅
- `v1.0.0` → `66c8cf01`
- `v2.0.0` → `de2c8cf`
- `v2.0.1` → `ca8ff24`
- `v2.0.2` → `adb6588`
- `v2.0.3` → `18e307a`

**Schema:** LATEST_SCHEMA_VERSION = 18 (Migration `0018_concurrency_default.sql` flattens unsafe `max_concurrent_downloads`-Werte auf 1, preserved 1/2/3).

**v2.1-ADR-Phase abgeschlossen 2026-05-12.** Bereit für Wave-4-Engineering-Implementation.

---

## SOLL-Zustand (Wave-4-Engineering-Implementation)

Wave-4 implementiert die in der v2.1-ADR-Phase fixierten Specs gegen `src/` / `src-tauri/`. Fünf Engineering-Missions, jeweils 2–4 Prompts, geschätzt 10–20 Prompts insgesamt; danach 2–4 Prompts für v2.1-Release.

1. **Visual-System-Migration** (gegen ADR-0040): Token-Migration in `src/styles/globals.css` (`@theme {}`-Block-Rewrite); `lucide-react`-Install + Icon-Imports; Hard-Coded-Hex-Audit; Token-Snapshot- und APCA-Kontrast-Tests; Reduced-Motion-Override-Block.
2. **VodCard-Rewrite** (gegen ADR-0039): `src/features/vods/VodCard.tsx` Rewrite mit drei Width-Varianten (Full/Compact/Sliver) + Hero-Variante; Stack-Silhouette + Avatar-Dots + Fan-Out + Retention-Stages + 3-px Progress-Stripe; State-Matrix-Coverage.
3. **TimelinePage-Rewrite** (gegen ADR-0038): `src/features/timeline/TimelinePage.tsx` Rewrite mit Single-Time-Axis-Layout + Lane-Packing + Zoom-Chrome + Now-Indicator + TimelineLanes + TimelineCard + TimelineStackMarker + TimelineSelectionBar + neuer `TimelineHeroSlot.tsx` (favourited-AND-live-trigger per D9 + TV-A5 280px Height).
4. **Multiview-Pane-Expansion-Implementation** (gegen ADR-0041): `MultiViewPage.tsx` Layout-variable für N=2/3/4 + Audio-Anker-Mechanik + Leader-Outline (Lucide Crown) + Per-Pane-Close-Button (Lucide X) + IPC additive Widening (`openSyncGroup` + neuer `cmd_close_pane`); `sync-store.ts` Erweiterung um `audioAnchorPaneIndex`.
5. **Integration + E2E + Polish:** Cross-Feature-Tests (Timeline-Selection → Multiview-Open-with-N-Panes), Playwright-E2E (gated auf E2E-Mission-Land), Performance-Validation (Per-Tick-CPU bei N=4 per ADR-0041 Risk #1), Visual-Regression-Baselines, v2.1-Release-Prep.

**Trigger für Wave-4-Start:** CTO bereitet Übergabe-Brief vor (siehe HANDOFF-2026-05-12), neuer Chat-Kontext für Wave-4-Mission.

---

## Aktive Autorun-Queue

Wave-4-Engineering-Missions (Start in neuem Chat-Kontext nach CTO-Übergabe):
1. Visual-System-Migration (Tokens + Lucide-Install)
2. VodCard-Rewrite
3. TimelinePage-Rewrite (inkl. TimelineHeroSlot)
4. Multiview-Pane-Expansion-Implementation
5. Integration + E2E + Polish

Geschätzt 10–20 Prompts insgesamt; danach 2–4 Prompts für v2.1-Release. Sequenz: 1 zuerst (alle anderen Missions konsumieren Tokens und Icons), dann 2/3/4 parallel-fähig, dann 5.

---

## Cron / Scheduled Tasks

Keine.

---

## Blocker

Keine. v2.1-ADR-Phase entblockt Wave-4-Engineering-Implementation.

---

## Offene Entscheidungen

Keine. Alle v2.1-ADR-OQs RESOLVED 2026-05-12 (4 + 3 + 7 + 5 = 19 CEO-Decisions über vier Wellen). Real-User-Validation läuft parallel.

---

## Letzter Handoff

`docs/HANDOFF-2026-05-12.md` — dokumentiert v2.1-ADR-Phasen-Abschluss (vier ACCEPTED ADRs 0038/0039/0040/0041 + R-ADR-Rules-Verankerung + Wave-4-Entblock-Pfad).

`docs/HANDOFF-2026-04-26-v2.0.3.md` — dokumentiert v2.0.3 Download-Engine-Hotfix (AC1-AC4 + Cleanup A/B). Real-Use-Phase läuft parallel; nicht stale.

---

## v2.1-Backlog

**Aus v2.0.3 hinzugekommen** (alle in ADR-0035/0036/0037 + STATE referenziert):
- Drop legacy `quality_preset` Spalten (Migration `0019_drop_legacy_quality_preset.sql` deferred — Code-Referenzen-Audit nicht in v2.0.3-Scope geleistet)
- Column DEFAULT-Change für `max_concurrent_downloads` (Migration setzt nur unsafe Werte, der DEFAULT-Constraint bleibt für sauberen v2.1-Cut)
- Record actual downloaded tier (für UI-Anzeige "geladen mit 720p30" pro VOD)
- Per-File 20-MB-Log-Cap (tracing-appender hat keine Size-Cap; daily rotation + keep-7 reichen für v2.0.3, sauberer Fix für v2.1)
- Cap `ytdlp.stderr` in File-Sink (verhindert Log-Spam bei Error-Storm)
- Atomic-Claim `UPDATE … RETURNING` für Download-Pick (statt SELECT-then-UPDATE)
- Linux Proton-Drive Detection
- `tauri-plugin-log` Adoption als saubere Tauri-2-API-Nutzung

**Aus v2.0.2 inherited:**
- `tauri-plugin-shell::Command::new_sidecar` Adoption
- Release-Workflow Post-Bundle-Inspection-Step
- `SHA256SUMS` Asset auf Releases publishen

**Aus v2.0.1 inherited:**
- SuspendController in `infra::process::suspend` refactoren
- `VodDeleted` Event-Variant (statt VodArchived-Reuse)
- Windows CI Runner für SuspendThread-Coverage
- StorageOutlook Component-Level Vitest Coverage
- Live-Update Forecast on Settings-Change

**Aus Phase 8 inherited:**
- AV1-Hardware-Encoder
- Per-Streamer Quality-Overrides
- Mobile-Network-Detection
- Per-Pane Volume/Mute-Persistenz für Sync-Sessions
- Playwright + Tauri-driver E2E

**Post-v2.0 (große Themen):**
- Code-Signing + Notarization (CEO-Entscheidung: nein, OSS bleibt unsigned)
- Self-Update-Install (nicht ohne Signing)
- Distribution-Channels (Homebrew/winget/AUR)

---

## Änderungshistorie

- 2026-04-25 — STATE.md initialisiert.
- 2026-04-25 — Phase 6 Housekeeping + Phase 6 proper merged + getaggt.
- 2026-04-25 — Phase 7 + macos-13-Hotfix merged. v1.0.0 published.
- 2026-04-25 — Phase 8 + Versions-Manifest-Hotfix merged. v2.0.0 published. R-SC-01/02/03 eingeführt.
- 2026-04-26 — v2.0.1 Scope-Closure released. R-RC + R-SC im Kombi-Einsatz bewährt.
- 2026-04-26 — v2.0.2 Critical Sidecar-Resolution-Hotfix released. Real-User-Bug-Report von CEO triggerte Mission. PR #23 Post-Merge-Hotfix für flaky PID-Test. Real-Use-Phase aktiviert.
- 2026-04-26 — v2.0.3 Critical Download-Engine-Hotfix released. PR #25, Tag `18e307a`. Drei Real-Use-Bugs gefixt (Quality-Profile-Wiring, Concurrency-Cap, Per-VOD-Process-Lock) plus Tech-Spec-§8-Compliance (File-Logger). ADRs 0035/0036/0037, Migration 0018, Schema v18. R-RC + R-SC dritter Hotfix-Run sauber durch.
- 2026-05-11 — Workforce-Erweiterung `visual-research`-Subagent angelegt, Netflix-Anker-Dokument erstellt als v2.1-Spec-Quelle. Keine Code-Änderungen am Produkt. Real-Use-Phase parallel weiter, Ampel bleibt grün. Anker-Dokument enthält 10 nummerierte Open-Questions für den CEO; ohne Sign-off keine v2.1-ADRs.
- 2026-05-12 — CEO hat Anker-Dokument-Decisions Q-1 bis Q-10 angenommen, strukturelle Korrektur an Q-4 freigegeben (Timeline ist sui generis, neues §3.6 Single-Time-Axis im Anker). Anker damit verbindliche Spec; v2.1-ADR-Phase entblockt. Nächster Schritt: ADR-Timeline-Layout (vor ADR-VodCard und ADR-VisualSystem-v2). Audit-Trail in `docs/decision-log/v2.1-anchor-acceptance.md`. Keine Code-Änderungen am Produkt; Real-Use-Phase läuft parallel weiter; Ampel grün.
- 2026-05-12 — ADR-0038 Timeline-Layout proposed, awaiting CEO sign-off. Acht engineering-binding Decisions (Lane-Packing-Algorithmus, Zoom-Skala, Now-Indicator, Lane-Capacity, Multi-Perspective-Grouping, Viewport-Virtualisierung, Multiview-Selection-Integration, Data-Layer + State-Ownership) ausgeliefert, alle mit Fallback-Option im decision-log. Keine CEO-relevanten Open Questions. Spec-Quelle: Anker §3.6/§6.1/§7. Welle 1 von 3 (Welle 2 ADR-VodCard, Welle 3 ADR-VisualSystem-v2). Keine Produkt-Code-Änderungen; Real-Use-Phase parallel weiter; Ampel grün.
- 2026-05-12 — ADR-0038 Timeline-Layout vom CEO mit vier Refinements ACCEPTED: CEO-A1 (Multiview-Pane-Count → 4, separate Multiview-Pane-Expansion-ADR als Render-Contract), CEO-A2 (Multi-Perspective-Threshold 75 % → 50 %, D5-Substanz-Korrektur, Late-Join-GTA-RP als tragendes Argument), CEO-A3 (Hero-Slot Layout-Reservation Trigger → favourited, eigene neue D9), CEO-A4 (Q-6 Keyboard-Cascade bestätigt, Tab-Focus = Mouse-Hover-Sequence mit 0,6–1,0 s Delay). Decision-Log §6 dokumentiert pro-CEO-A-Decision Statement/Begründung/Folge plus Schluss-Notiz zur R-SC-konformen Acceptance-Disziplin („Hedges-im-ADR sind eskalations-pflichtig, nicht engineering-default"). ADR-Status-Header auf „Accepted 2026-05-12"; CLAUDE.md Active-Decisions-Register synchron. Keine Produkt-Code-Änderungen. Welle 1 ACCEPTED, Welle 2 (ADR-VodCard) als nächster Schritt; parallel hängt Multiview-Pane-Expansion-ADR (CEO-A1-Folge, eigener Prompt). Ampel grün.
- 2026-05-12 — ADR-0039 VodCard proposed, awaiting CEO sign-off. Zehn DV-Decisions ausgeliefert: DV1 Card-Geometry-Kontrakt (16:9 Image ≥ 75 % + Strip ≤ 25 % + drei Width-Varianten Full/Compact/Sliver); DV2 Idle-State-Komposition mit kollisionsfreier Position-Map (Selection-Top-Right, Retention-Bottom-Right, Stripe-Bottom-Edge); DV3 Hover-Preview-Sequenz mit 800 ms Default-Delay als H-18-Midpoint und CEO-A4-Cascade-Union-Semantik; DV4 Multi-Perspective-Stack mit 2 Silhouette-Layer + 4 Avatar-Dots-Max + lateral-right-Fan-Out + Auto-Mirror-Left an Right-Edge; DV5 Retention-Komposition mit 3-Stage Position-Migration vom Strip auf Image-Overlay ab 48h und Client-Side-Compute aus `broadcaster_type`; DV6 3-px Stripe am Image-Area-Bottom mit State-Machine-gated Visibility; DV7 vollständige State-Matrix (13 primary rows × Stack-Variant); DV8 Composite-Hook `useVodSummary` ohne neuen IPC-Endpoint; DV9 Tab-Order lane-by-lane chronological-within plus ARIA-Label-Format; DV10 React.memo + Eager-Mount-CSS-Hidden + Singleton-Loop-Invariant + 400 ms Frame-Cadence v2.0.x-Precedent. Drei genuine Open Questions for CEO: VOd-Q2 (Retention-Source backend vs. client-Compute), VOd-Q3 (Completed-Watch-Indicator (a)/(b)/(c)/(d) — Engineering-Empfehlung (b)), VOd-Q5 (Stack-Fan-Out-Edge-Trajektorie — Engineering-Default Auto-Mirror-Left). Anti-Smell-Audit explizit durchgeführt (Welle-1-Lehre angewandt); VOd-Q1 (GIF-Strip-Pre-Ingest) und VOd-Q4 (Composite-Batch-Endpoint) als Engineering Defaults mit begründung dokumentiert; elf weitere Sub-Decisions geprüft und als Engineering-Defaults oder Anchor-konstrainiert markiert. Escalation-Note für CTO als formaler Forward-Process-Surface („≥ 3 OQs" Threshold; CTO kann forward-as-is / pre-process / subdivide). Keine Produkt-Code-Änderungen; keine Token-Werte; keine GIF-Pipeline-Implementation. Welle 3 (ADR-VisualSystem-v2) als nächster Schritt nach VodCard-Sign-off; parallel hängt Multiview-Pane-Expansion-ADR (CEO-A1-Folge). Ampel grün.
- 2026-05-12 — ADR-0039 VodCard vom CEO mit drei Decisions ACCEPTED — alle drei Engineering-Defaults aus dem PROPOSED-ADR ohne Substanz-Änderung übernommen: VOd-A2 (Retention-Source = Client-Side-Compute aus `streamer.broadcasterType` + `vod.streamStartedAt`; Backend-`retention_at`-STORED-Spalte deferred auf v2.2 mit explizitem Trigger Real-Use-Drift-Reports; konservativ-falsch-Drift-Argument trägt), VOd-A3 (Completed-Watch-Indicator = Option (b) neutraler Checkmark-Glyph in Metadata-Strip Line 2 Column 2 right-aligned; Wave-3-Thumbnail-Opacity-Reduction auf ~75–80 % als Single-Line-CSS-Change-Follow-up reserviert falls Real-Use-Feedback Background-Noise zeigt — kein ADR-Re-Open), VOd-A5 (Stack-Card-Fan-Out-Edge-Trajektorie = Auto-Mirror-Left; Smart-Direction-Pattern wie Dropdowns/Tooltips; Vertical und Suppress rejected). Decision-Log §7 dokumentiert pro-VOd-A-Decision Statement/Begründung/Folge plus Schluss-Notiz zur CTO-Forward-as-is-Pfad-Bestätigung und Anti-Smell-Audit-Post-Sign-off-Verifikation (keine versteckten Sub-Decisions entdeckt — präventiver Audit aus Welle-1-Lehre hat getragen). ADR-Status-Header auf „Accepted 2026-05-12"; Open Questions-Sektion zu CEO Decisions (2026-05-12) umbenannt; Escalation-Note for CTO zu „resolved 2026-05-12" umgestellt; Consequences > Risks #6 mit explizitem Wave-3-Mitigation-Pfad ergänzt; CLAUDE.md Active-Decisions-Register synchron. Keine Produkt-Code-Änderungen; keine Token-Werte; keine DV-Substanz berührt — reiner Status- und OQ-Surface-Konvertierungs-Patch (im Unterschied zur Welle-1-Acceptance, die mit CEO-A2 75 → 50 % eine echte Substanz-Korrektur hatte). Welle 2 ACCEPTED, Welle 3 (ADR-VisualSystem-v2) als nächster Schritt; parallel hängt Multiview-Pane-Expansion-ADR (CEO-A1-Folge, eigener Prompt). Ampel grün.
- 2026-05-12 — ADR-0040 Visual System v2 proposed, awaiting CEO sign-off. Zwölf TV-Token-Decisions ausgeliefert: TV1 Color-System (4-Tier Surface-Ladder L\*=4/7/10/13 + 2-Tier Text-Ramp ohne tertiary grey + `#d4a14a` Locked Input + neutral H-20-Outlines + Coverage-Math-Verifikation 1.72 % viewport-level); TV2 Typography (System-Font-Stack + 4-Weight-Ladder + 7-Size-Ladder 12–32 px); TV3 Spacing (4-px-Base mit 10-Step-Ladder); TV4 Card-Geometry (lane_height_px=72 + width-thresholds 160/96 + corner-radius 4 + hover-zoom 1.10); TV5 Card-Element-Numerics (16-px Checkmark + 20-px Retention-Badge + 3-px Stripe + 2-px Focus-Outline); TV6 Motion-Constants (800-ms Hover-Delay + 200-ms Zoom + 250-ms Crossfade + 400-ms Frame-Cadence + `cubic-bezier(0.16, 1, 0.3, 1)` Easing + Reduced-Motion-Override-Block); TV7 Elevation (kein Card-Shadow + 2-px Modal-Shadow + 11-Layer Z-Index-Stack); TV8 Now-Indicator (1-px white at 30 % opacity); TV9 Sidebar (56-px collapsed / 240-px expanded + Accent-Live-Dot); TV10 Hero-Slot (280 px Height + total-collapse-on-no-favourited-live + most-recently-started Tie-Break); TV11 Icon-System (Lucide-react install in Wave 4); TV12 Implementation (Tailwind v4 native `@theme {}` + 16-Item Migration-Liste + Test-Strategie). Sieben genuine Open Questions for CEO: TV-Q1 (lane_height_px), TV-Q2 (card-width-thresholds), TV-Q3 (hover-zoom-factor), TV-Q4 (sidebar-collapsed-width), TV-Q5 (hero-slot-height), TV-Q6 (hero-tie-break), TV-Q7 (live-dot-color). Anti-Smell-Audit explizit durchgeführt (Welle-1+2-Lehre angewandt) — 7 OQs identified + 11 weitere Sub-Decisions als Engineering-Defaults mit Begründung dokumentiert; Spec-Korrektur in-flight entdeckt (Focus-Outline-Opacity 50 % → 85 % per actual H-20 binding 80–90 %) und applied. Escalation-Note für CTO als formaler Forward-Process-Surface (7 > 5 Threshold). Vollständiger CSS-Custom-Properties-Block (~150 Zeilen) als verbindliche Wave-4-Migration-Spec. Ampel grün.
- 2026-05-12 — Workforce-Rules erweitert: `.claude/rules/adr-discipline.md` mit R-ADR-01 (Engineering-Anti-Smell-Audit, Pre-Delivery, mit Spec-Source-Verification + Documentary-Konsistenz-Pass), R-ADR-02 (Pre-Sign-off-CTO-Audit, komplementär), R-ADR-03 (Acceptance-Patch-Pattern PROPOSED → ACCEPTED mit Substanz-Patch / Reine-Status-Konvertierung als zwei Pfade), R-ADR-04 (Three-Forward-Process-Options bei ≥ 5 OQ-Schwelle: Forward-as-is / Pre-process / Subdivide) nach v2.1-ADR-Phase Welle 1 + 2 + 3 Lehre verankert. Empirische Pattern-Source: Decision-Logs ADR-0038 §6 + ADR-0039 §6/§7 + ADR-0040 §6/§7/§8. CLAUDE.md unverändert (kein per-Rule-Register; generischer `.claude/rules/`-Glob-Pointer deckt die neue Datei automatisch ab). **Multiview-Pane-Expansion-ADR (CEO-A1-Folge aus ADR-0038, parallele Welle) ist nächste ADR-Welle; danach Wave-4-Engineering-Implementation startbereit.** Kein Ampel-Wechsel; bleibt Grün.
- 2026-05-12 — ADR-0040 Visual System v2 vom CEO mit sieben Decisions ACCEPTED — alle Engineering-Defaults aus dem PROPOSED-ADR ohne Substanz-Änderung übernommen (TV-A1 lane_height_px = 72; TV-A2 card-width-thresholds 160/96 px; TV-A3 hover-zoom-factor = 1.10; TV-A4 sidebar-collapsed-width = 56 px; TV-A5 hero-slot-height = 280 px; TV-A6 hero tie-break = most-recently-started via `streamers.last_live_at`; TV-A7 live-state indicator = `#d4a14a` accent dot als brand-touchpoint). CTO wählte **Forward-as-is (Option 1)** über Subdivide (Option 3); tuning-vs-structural-Klassifikation des PROPOSED-Subdivide-Vorschlags erwies sich bei Scrutiny als unscharf (TV-Q7 ist Brand-Touchpoint nicht Tuning; TV-Q2 ist user-feel-prägend durch Compact/Sliver-Threshold-Impact). Block-Acceptance-Pattern aus Welle 1 (4 CEO-Decisions) und Welle 2 (3 CEO-Decisions) hat sich linear-skalierend für 7-OQ-Block bewährt. Decision-Log §8 dokumentiert pro-TV-A-Decision Statement / Begründung (CTO-Lesart) / Folge plus Schluss-Notiz zur Komplementarität-Pattern-Lehre der zwei Audit-Disziplinen (Engineering-Anti-Smell-Audit Pre-Delivery + Pre-Sign-off-CTO-Audit Pre-Sign-off). Zwei dokumentarische Pre-Sign-off-CTO-Audit-Findings eingearbeitet ohne TV-Decision-Substanz-Änderung: (1) Counting-Bug-Fix global („six" → „seven" — Mission-Brief sagte „six open questions" während tatsächlich sieben TV-Q1..TV-Q7 gelistet waren); (2) explizite TV-A1 × TV-A5 N_max-Compound-Risk-Notation als neuer Consequences > Risks #6-Eintrag (defaults 5/3; hypothetische 60+240 → 6/4; 80+320 → 4/2; Mitigation: joint-evaluation-requirement für künftige Re-Tunes). ADR-Status-Header auf „Accepted 2026-05-12"; Open Questions-Sektion zu CEO Decisions (2026-05-12) umbenannt mit pro Decision `### TV-AN`-Block; Escalation-Note for CTO zu „resolved 2026-05-12" umgestellt; CLAUDE.md Active-Decisions-Register synchron. Keine Produkt-Code-Änderungen; keine Token-Migration; keine `globals.css`-Edits; keine TV-Decision-Substanz berührt — reiner Status-Konvertierungs-Patch plus zwei dokumentarische Findings. Pattern-Lehre für bevorstehende R-ADR-Rules-Erweiterungs-Welle: Engineering-Anti-Smell-Audit + Pre-Sign-off-CTO-Audit werden als separate komplementäre Rules verankert (verschiedene Failure-Modes — Engineering-Wahl-Space vs. documentary-Konsistenz + cross-cutting-Risk-Notation). **v2.1-ADR-Phase strukturell komplett** bis auf Multiview-Pane-Expansion-ADR (parallele Welle, CEO-A1-Folge aus ADR-0038). Nach dessen Sign-off ist Wave-4-Engineering-Implementation entblockt. Ampel grün.
- 2026-05-12 — ADR-0041 ACCEPTED (Block-Acceptance, R-ADR-03 reine Status-Konvertierung). CEO concur mit allen fünf Engineering-Empfehlungen aus dem PROPOSED-ADR: M-A1 = (d) 2×2-with-empty, M-A2 = (α) Per-pane Close, M-A3 = 200 ms Audio-Switch-Delay, M-A4 = Hard-switch Audio-Switch-Strategy, M-A5 = Spring-back Audio-Return. Compound-Risk #4 (M-A3 × M-A4) als Paar acknowledged („gentle, responsive" UX-Charakter). Keine Substanz-Patches, keine in-flight-Korrekturen. Status-Header auf „Accepted 2026-05-12"; Acceptance-Block direkt unter Header; OQ-Block + Escalation-Note als RESOLVED markiert (Audit-Trail bleibt); Costs accepted #4/#5/#6 + Risks #4/#5 auf M-A-N umgestellt; Decision-Log §7 Sign-off-Sektion neu mit pro-M-A-Decision-Blöcken + Compound-Risk-Acknowledgment + Forward-as-is-Disziplin-Notiz + Phasen-Abschluss-Notiz; §1 M1/M3/M4/M7/M12 Acceptance-Notes; §4 AC-Tabelle erweitert; CHANGELOG/CLAUDE.md synchronisiert; HANDOFF-2026-05-12 als v2.1-ADR-Phasen-Abschluss-Brief geschrieben. **v2.1-ADR-Phase strukturell vollständig** — vier ACCEPTED ADRs (0038/0039/0040/0041) plus R-ADR-Rules verankert. Wave-4-Engineering-Implementation entblockt. Block-Acceptance-Pattern (4 → 3 → 7 → 5 CEO-Decisions Welle-übergreifend) hat sich linear-skalierend bewährt. Ampel grün.
- 2026-05-12 — ADR-0041 Multiview Pane Expansion proposed, awaiting CEO sign-off. **Erste ADR unter den R-ADR-Rules** (verankert 2026-05-12 nach Welle 3 retrospect), parallele v2.1-ADR-Welle aus CEO-A1 (ADR-0038 ACCEPTED 2026-05-12). Zwölf M-Decisions ausgeliefert: M1 Pane-Count Layout-Geometry (1/2/3/4-Pane Render-Logik mit drei CSS-Grid-Templates; 2×2-Quadrant Equal-size für N=4 per Anker Q-8 binding; Inter-pane gap `--space-1` 4 px aus ADR-0040 TV3); M2 Reshuffle on Pane-Add (deterministic slot-index order + 200ms opacity crossfade via `--motion-zoom-duration` aus TV6 + reduced-motion 100ms fade-instant); M3 Reshuffle on Pane-Remove (slot-preserving with re-pack at N→N−1 boundary; Leader-Re-Election = lowest remaining pane-index; Audio-Anchor re-routing back to leader if anchor removed); M4 Audio-Anchor Mechanic (Default Audio = Leader per ADR-0023 + Switch on pointer-hover non-Leader per Anker Q-8; Sub-Decisions M4a/M4b/M4c als Open Questions M-Q3/M-Q4/M-Q5; M4d viewport-exit same as M4c; M4e Lucide Volume2 glyph in pane-chrome per TV11; State-Diagram für Default/Hover-Switch/Return-Behavior); M5 Leader-Outline (2px `--color-outline-focus` rgba(255,255,255,0.85) + `--focus-outline-offset` 2px inside + `--z-card-hover` 20 + Lucide Crown als Leader-Badge; ersetzt v1 `★ Leader in amber pill` welches H-20/H-07 verletzt); M6 IPC additive widening (`openSyncGroup({ vodIds, layout })` von strict `vodIds.length === 2 + "split-50-50"` auf `vodIds.length ∈ [2,4] + "split-50-50" | "quadrant-2x2"`; Selection-Reihenfolge ↔ Pane-Index ascending; First-selected = Default-Leader); M7 Pane-Close-Affordance (Open Question M-Q2; Engineering-Empfehlung (α) Per-pane Close-Button mit Lucide X); M8 Transport-Bar strukturell unverändert (nur Wave-4 TV11-Lucide-Re-Skin); M9 per-pane Mute via routing-layer (`effective_audio = anchor_pane.volume × (anchor_pane.muted ? 0 : 1)`; preserves ADR-0021 binding); M10 Focus-Mode-Toggle deferred zu v2.x per Anker Q-8 binding ("future opt-in option, not the default" — Spec-binding-Defer); M11 Out-of-Range pro Pane ADR-0022-konform (Pane bleibt im Grid-Slot mit Overlay; auto-collapse rejected); M12 Token-Konsumption-Inventur aus ADR-0040 plus drei neue Multiview-Tokens (`--multiview-pane-gap` alias `--space-1`; `--motion-audio-switch-delay` per M-Q3; `--motion-audio-switch-duration` per M-Q4). **Fünf Open Questions for CEO:** M-Q1 (3-Pane Layout (a)/(b)/(c)/(d) — Engineering-Empfehlung (d) 2×2-with-empty), M-Q2 (Per-pane vs Group-only Close — Empfehlung (α) Per-pane), M-Q3 (Audio-Switch-Delay 0/200/800ms — Empfehlung 200ms), M-Q4 (Hard-switch vs Crossfade — Empfehlung Hard-switch), M-Q5 (Spring-back vs Sticky on Hover-Exit — Empfehlung Spring-back). R-ADR-01 Engineering-Anti-Smell-Audit präventiv (erste Vorwärtsanwendung nach Rules-Verankerung); 8 Mission-Risk-Kandidaten geprüft (5 OQs surfacd, 1 dropped als Duplikat M-Q6=M-Q2, 2 als Engineering-Defaults mit Spec-Source-Begründung: M-Q7 Mute-Semantik via ADR-0021 binding, M-Q8 Focus-Mode-Defer via Anker Q-8 binding) plus 13 weitere Sub-Decisions im Schreiben aufgetaucht, alle als Engineering-Defaults dokumentiert. Documentary-Konsistenz-Pass bestanden (5 OQs konsistent über alle Sektionen; Cross-References konsistent; Compound-Risk M-Q3 × M-Q4 + M-Q1 × M-Q2 Interaction explizit notiert). Keine Spec-Source-Diskrepanz gefangen (Welle-3-H-20-Pattern-Lehre nicht aufgetreten; Mission-Brief reproduzierte Anker §6.3 + Q-8 + ADR-0021/22/23/38/40 binding inputs faithfully). R-ADR-04 ≥ 5 OQ-Schwelle exact erreicht; Escalation-Note für CTO mit Forward-as-is-Empfehlung (Option 1) über Subdivide (Option 3 — M-Q1/M-Q4 structural, M-Q3 tuning, M-Q5 UX-feel mapped awkwardly) und Pre-process (Option 2 — würde CEO aus 5 UX-prägenden Decisions herausdrücken); per-OQ CTO-Empfehlung mit Rationale-Tag. Compound-Risk-Callout für M-Q3 × M-Q4 explizit in Escalation-Note. Keine Produkt-Code-Änderungen; keine Token-Werte; keine Multiview-Komponenten-Migration; keine `globals.css`-Edits; keine `lucide-react`-Installation; keine Backend-Schema-Änderungen. Nach Sign-off ist v2.1-ADR-Phase strukturell vollständig und Wave-4-Engineering-Implementation entblockt. Ampel grün.
- 2026-05-15 — ADR-0040 § TV1 + § Risks #1 Lc-correction docs patch (R-ADR-01-Befund aus Wave-4 Mission 1). Token-Substanz unverändert; über-optimistic Lc-88-Claim auf empirischen Lc-58 korrigiert (max achievable für Locked-Input-Accent `#d4a14a`), sekundärer `#f5f5f7`-Bullet von over-pessimistic 30 auf gemessenen 44; § Risks #1 Mitigation um Large-Text-Band-Klausel erweitert; § TV10 Hero-CTA `--font-size-2xl`-Verweis ergänzt; § Acceptance-Note hinzugefügt; Decision-Log §9. Erste recorded Post-Acceptance documentary correction. Single commit direkt auf main. Ampel grün.
