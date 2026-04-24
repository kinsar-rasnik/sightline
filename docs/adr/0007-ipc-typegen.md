# ADR-0007: IPC type generation via `tauri-specta`

- **Status.** Accepted (2026-04-24).
- **Supersedes.** None. Operationalizes ADR-0004 (which committed to typed IPC but deferred the generator wiring to Phase 2).
- **Deciders.** Senior Engineer (Claude), reviewing CTO.

## Context

ADR-0004 accepted `tauri-specta` as the mechanism for generating TypeScript bindings from Rust `#[tauri::command]` signatures. Phase 1 deliberately deferred the wiring: a single `health` command did not justify the build-script complexity at the time, and a hand-maintained `src/ipc/bindings.ts` tracked the Rust shape with a prominent header warning.

Phase 2 adds ~10 new commands plus a family of events (`vod:*`, `streamer:*`, `poll:*`, `credentials:changed`) with non-trivial payloads (filter objects, discriminated error unions, ISO-8601 timestamps, enum-backed statuses). Hand-maintaining two sources of truth at that size is a liability — the first drift between Rust and TS is hours of debugging.

Three options were considered:

1. **`tauri-specta` 2.x (`2.0.0-rc.24` at time of writing).**
   - Uses `specta` for the Rust type model, `specta-typescript` for the emitter.
   - Canonical approach in the Tauri 2 ecosystem; used by ~100 open-source projects as of April 2026.
   - Still tagged `-rc` upstream, but `rc.24` has been API-stable since late 2025 and is the version every active Tauri 2 project ships against. Moving to a theoretical future stable will be a bump, not a rewrite.
   - Rich API: derives `specta::Type` on Rust types, `#[specta::specta]` on command fns, a `Builder` that composes commands + events and emits both `invoke_handler` + TS output.
   - Emits Result-style discriminated unions for `Result<T, AppError>` — the frontend handles `{ status: "ok", data } | { status: "error", error }` without custom wrapping.

2. **`tauri-bindgen` (`0.1.0-alpha02`).**
   - Wit-based approach, generates bindings from `.wit` files rather than Rust types.
   - Alpha, no 1.0 roadmap visible. Two orders of magnitude fewer production users than tauri-specta.
   - Adds a second schema source (the `.wit` file) which contradicts "Rust is canonical".

3. **Hand-maintained / custom codegen.**
   - Proven in Phase 1 for one command. Does not scale to a dozen commands with shared enums and error variants.
   - A custom codegen would re-derive what `specta` already does well; not a differentiator worth owning.

## Decision

Adopt **`tauri-specta` 2.x (`^2.0.0-rc.24`)** with `specta` + `specta-typescript`.

### Wiring

1. Rust types that cross IPC derive `specta::Type` in addition to the existing serde derives.
2. Command functions are annotated `#[specta::specta]` alongside `#[tauri::command]`.
3. A single `tauri_specta::Builder` is constructed in `src-tauri/src/lib.rs`, registering all commands and events.
4. The Builder is both (a) mounted on the Tauri `invoke_handler` and event system and (b) exported to TypeScript.

### When the export runs

- **Debug builds:** `builder.export(...)` runs during the Tauri `setup` hook under `#[cfg(debug_assertions)]`. Every `pnpm tauri dev` rebuild regenerates `src/ipc/bindings.ts` before the frontend sees it.
- **CI:** a `#[test]` in `src-tauri/tests/ipc_bindings.rs` (`cargo test --test ipc_bindings`) calls the same export function and writes to the same path. Combined with the `git diff --exit-code src/ipc/bindings.ts` step in CI, committing stale bindings becomes a hard failure.
- **Release builds:** the exported file is already committed, so release builds do not regenerate. This keeps release artifacts deterministic.

### File layout

- `src/ipc/bindings.ts` — **generated**. Starts with a `// AUTO-GENERATED — DO NOT EDIT` banner emitted by `specta-typescript`. Replaces the Phase 1 hand-maintained mirror.
- `src/ipc/index.ts` — **hand-written**, thin. Re-exports the generated `commands`, `events`, and types, plus a small `IpcError` + `unwrap` helper that converts the generated `Result<T, E>` shape into throw-style `Promise<T>` so React hooks stay idiomatic.
- `pnpm run check:ipc` — wrapper that runs `cargo test --test ipc_bindings` then `git diff --exit-code src/ipc/bindings.ts`. The phase-gate skill calls this as part of the IPC-drift step.

### Error representation on the wire

`AppError` stays a `#[serde(tag = "kind", rename_all = "snake_case")]` enum. `specta::Type` derives produce a matching TS discriminated union automatically. Frontend code narrows on `error.kind`; no `catch (e: any)`.

## Consequences

### Positive

- One source of truth for IPC types. The Rust signature is canonical; drift is detected in CI.
- New commands are a strictly-mechanical addition: derive `specta::Type`, annotate `#[specta::specta]`, register in the Builder. The `add-tauri-command` skill is updated to bake this in.
- Discriminated-union errors are propagated precisely, including nested variants like `TwitchRateLimitError { retry_after_seconds }`.
- The frontend drops the manual `invokeTyped` + stringly-typed `invoke<T>(cmd, args)` pattern. Generated commands are fully-typed, including input objects.

### Negative / accepted risks

- Dependency on an RC-tagged crate. Mitigation: pin minor-rc (`=2.0.0-rc.24` in Cargo.toml is considered; we pick `2.0.0-rc.24` as the minimum and allow patch-rc bumps via the caret); ADR is revisited when tauri-specta 2.0 stabilizes.
- Adds build-time codegen to the debug path. Risk: slower dev iteration. Mitigation: export runs at `setup` (once per process start), not per hot-reload; measured overhead under 200 ms in initial spike.
- Committing a generated file means any PR that changes the command signature must include the regenerated bindings. Mitigation: `check:ipc` as a standard phase-gate step + the stop-gate hook already nudges the developer to regenerate.

### Neutral

- The Phase 1 `IpcError` class is kept — slightly reshaped to wrap the generated error union. React hooks continue to `await` the unwrapping helper so the component-level API is unchanged.

## Migration

1. Add `specta`, `specta-typescript`, `tauri-specta` (feature `typescript`) to `src-tauri/Cargo.toml`.
2. Add `specta::Type` derives to `AppError`, `HealthReport`.
3. Annotate `commands::health::health` with `#[specta::specta]`.
4. Build `tauri_specta::Builder` in `lib.rs::run`; mount it on the Tauri builder.
5. Add `pub fn ipc_builder()` in `lib.rs` and `src-tauri/tests/ipc_bindings.rs` calling `ipc_builder().export(...)`.
6. Delete the hand-maintained `src/ipc/bindings.ts`; regenerate via the test.
7. Keep `src/ipc/index.ts` as the manual throw-style wrapper plus re-exports.
8. Update `.claude/skills/add-tauri-command/SKILL.md` so the playbook reflects the new flow.
9. Document the drift check in the phase-gate skill.

## References

- [tauri-specta](https://github.com/specta-rs/tauri-specta) — upstream repository and docs.
- [specta-rs/specta](https://github.com/specta-rs/specta) — core type model.
- ADR-0004 — committed to typed IPC; this ADR operationalizes it.
- Phase 1 session report (`docs/session-reports/phase-01.md`) — deferred the wiring; see §Deviated.
