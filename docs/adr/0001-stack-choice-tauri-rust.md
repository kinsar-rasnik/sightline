# ADR-0001 — Stack choice: Tauri 2 + Rust + React/TS

- **Status.** Accepted
- **Date.** 2026-04-24
- **Phase.** 1
- **Deciders.** CEO, CTO

## Context

Sightline is a long-lived desktop application that polls third-party APIs in the background, orchestrates a download pipeline, and plays local video. It must run on macOS, Windows, and Linux, and be comfortable to leave running for hours without a noticeable memory or power footprint. It must also be comfortable to develop without a large binary bundle.

We surveyed three candidate stacks:

1. **Electron + Node/TS.** Mature ecosystem, rich native modules, but every app bundles its own Chromium — tens of megabytes per install and >200 MB RSS for an idle window. Background polling in Node requires a separate hidden window or a worker; neither is elegant.
2. **Flutter desktop.** First-class UI, single language, but the Rust backend we want (for sidecar control, sqlx, tokio ecosystem) would require FFI bridges we do not want to maintain. The desktop target is also not as polished as mobile.
3. **Tauri 2 + Rust + webview.** Uses the OS webview (no bundled browser). Rust process owns the heavy lifting with first-class tokio and sqlx support. Typed IPC through tauri-specta. Installer sizes measured in single-digit megabytes.

## Decision

We adopt **Tauri 2** with **Rust** as the backend language and **React 19 + TypeScript** as the frontend.

- Rust stable 1.90+, edition 2024.
- React 19, TypeScript 5, Tailwind CSS 4, shadcn/ui for primitives.
- pnpm as the Node package manager (see ADR-0006).
- tauri-specta for IPC type generation (see ADR-0004).
- sqlx for persistence (see ADR-0002).

## Consequences

Positive:

- **Small installers, small runtime.** The OS webview is reused; our bundle carries only the Rust binary and assets.
- **Strong backend ergonomics.** tokio, serde, sqlx, reqwest, thiserror are all first-class.
- **Typed end-to-end.** Commands, events, and errors share one source of truth via tauri-specta.
- **Native feel.** Per-platform webview honors system settings (reduced motion, font scaling, etc.) without configuration.

Negative:

- **Three webview engines to test.** WebKit (macOS), WebView2 (Windows), WebKitGTK (Linux) have subtle differences (flexbox bugs, codec coverage, dev-tools availability). CI must run against all three.
- **Smaller ecosystem than Electron.** Fewer drop-in integrations; more hand-rolled wrappers.
- **Rust learning cost.** Team members who came from Node need ramp-up for async Rust, lifetimes, and trait objects.

## Mitigations

- CI matrix covers macOS, Windows, Linux on every PR (see `.github/workflows/ci.yml`).
- Frontend deliberately avoids APIs with poor WebKit support (e.g., container queries are OK; cutting-edge `anchor-name` positioning is not).
- Rust layering (commands → services → domain → infra) is enforced by reviews and by clippy rules; this reduces cognitive load for new contributors.

## Alternatives considered and rejected

- **Electron.** Rejected for runtime footprint and the weak story for long-running background work inside a hidden renderer.
- **Flutter desktop.** Rejected because the pipeline components we need (sqlx, yt-dlp supervision, tokio scheduling) are Rust-native.
- **Pure native per-platform (Swift + Kotlin + GTK/Qt).** Rejected as an order-of-magnitude increase in surface area for a small team.

## Supersedes / superseded by

- None.
