# STATE — Sightline

**Letzte Aktualisierung:** 2026-04-25
**System-Ampel:** 🟢 Grün — v1.0.0 released

---

## Kompass

**Projekt:** Lokale Cross-Plattform-Desktop-App (Tauri 2 + Rust + React) zur Aggregation von Twitch-GTA-RP-VODs mehrerer Streamer auf einer chronologischen Multi-Perspektiven-Timeline. Open-Source, MIT, GitHub-distributiert (unsigned binaries, Build-from-source als first-class Pfad).

**Repo:** github.com/kinsar-rasnik/sightline

**Status:** v1.0.0 live unter github.com/kinsar-rasnik/sightline/releases/tag/v1.0.0 — 5 Assets (macOS aarch64, Linux AppImage + Deb, Windows MSI + NSIS).

**Roadmap:**
- Phase 1–7 ✅
- Phase 8 — Storage-Aware Distribution (Quality-Pipeline + Pull-Modell + Storage-Forecast) ⏳ next, single-run-mission

---

## Ampel-Begründung

🟢 — v1.0.0 released, alle Phasen 1–7 abgeschlossen, alle Tags gesetzt, GitHub-Release publiziert mit auto-generierten Release-Notes. Repo clean, kein offener Blocker.

---

## IST-Zustand

**Repository:**
- Working-Directory: `~/Documents/sightline`
- Branch: `main`
- main HEAD: `66c8cf0` (Phase 7 + macos-13 Hotfix, PR #18)
- Working Tree: clean

**Tags:**
- `phase-1-complete` bis `phase-7-complete` ✅
- `v1.0.0` → `66c8cf01` (erstes Public Release)

**Letzte Merges:** PR #17 (Phase 7) + PR #18 (Hotfix macos-13 Runner-Retirement). Release-Workflow grün durchgelaufen, 5 Assets published.

---

## SOLL-Zustand (Phase 8)

**Storage-Aware Distribution.** Vom CEO als neue, jetzt letzte Phase definiert. Hintergrund: Default-Auto-Download mit Source-Quality skaliert nicht für 20-GB-Disk-User. Phase 8 baut die UX-Reality-Anpassung.

Drei Workstreams:
1. **Quality-Pipeline:** 720p30 H.265 als neuer Default, Hardware-Encode-Detection, CPU-Throttle/Background-Friendly Re-Encode, alle Quality-Stufen als User-Override mit konkreten Warnungen + Beispielrechnungen. Audio bleibt unverändert (GTA-RP-essentiell).
2. **Pull-Modell mit Sliding-Window:** Polling markiert VODs als "available" (Metadaten-only), User pickt manuell oder via Rule "lade die nächsten N". Auto-Cleanup nach watched-completed. Pre-Fetch der nächsten N beim Watching. Default N=2, konfigurierbar.
3. **Storage-Forecast-UI:** Vor Streamer-Add zeigt UI "geschätzt X GB/Woche bei deinen Settings", warnt bei Watermark-Überschreitung-Risiko. Library-UI zeigt available + downloaded VODs (UX-Re-Konzeption).

**Versionssprung:** v2.0.0 (Breaking-Change im Storage-Verhalten, Library-UX-Re-Konzeption).

Locked Decisions:
- 720p30 H.265 default, Hardware-Encode wenn verfügbar
- Audio: niemals reduzieren
- Re-Encode CPU-Throttle: nice-Priority + adaptive Throttling bei System-Load
- Pull-Modell ersetzt Push-Modell (Migrations-Pfad mit Backwards-Compat)
- Streaming-Hybrid-Idee verworfen (würde Multi-View kompromittieren)

---

## Aktive Autorun-Queue

**Aktuell laufend:** Keine. v1.0.0 released, Phase 8 wartet auf Mission-Start.

**Pipeline:**
1. Phase 8 (Storage-Aware Distribution → v2.0.0) — single-run-mission, vermutlich umfangreicher als Phase 7
2. Post-v2.0: Auto-Issue-Triage-Workflow via Claude Desktop + GitHub (eigenes Folge-Projekt)

---

## Cron / Scheduled Tasks

Keine aktiven Cron-Jobs. Workforce arbeitet ausschließlich im Push-Modus auf CTO-Anweisung.

---

## Blocker

Keine offenen Blocker. v1.0.0 ist live. Repo sauber.

---

## Offene Entscheidungen

Keine — alle Phase-8-Constraints geklärt:
- 720p30 H.265 default, alle Stufen als Override mit Warnung
- Audio nie reduzieren
- Hardware-Encode bevorzugt, Software nur als Fallback mit Warnung
- Pull-Modell ersetzt Push-Modell
- Streaming-Hybrid verworfen
- v2.0.0 als nächster Release-Tag

---

## Letzter Handoff

`docs/HANDOFF-2026-04-25.md` — dokumentiert Phase 6 Housekeeping + Phase 6 proper.
**Pflicht für Phase 7 + v1.0.0:** neuer HANDOFF nach Phase-8-Abschluss schreiben (`docs/HANDOFF-YYYY-MM-DD.md`), der Phase 7 + v1.0.0 + Phase 8 + v2.0.0 zusammenfasst.

---

## Änderungshistorie

- 2026-04-25 — STATE.md initialisiert, IST/SOLL aufgenommen.
- 2026-04-25 — Phase 6 Housekeeping (PR #15) und Phase 6 proper (PR #16) gemerged + getaggt.
- 2026-04-25 — Phase 7 (PR #17) + macos-13-Hotfix (PR #18) gemerged. v1.0.0 als erstes Public Release published. Phase 8 als neue, vorerst letzte Phase aufgenommen (Storage-Aware Distribution).
