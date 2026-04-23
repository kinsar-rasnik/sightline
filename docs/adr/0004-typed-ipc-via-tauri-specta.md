# ADR-0004 — Typed IPC via tauri-specta

- **Status.** Accepted
- **Date.** 2026-04-24
- **Phase.** 1
- **Deciders.** CTO, Senior Engineer

## Context

Tauri commands are Rust functions annotated with `#[tauri::command]`. On the frontend, calls go through an untyped `invoke(name, payload)`. Without discipline, this leads to:

- Hand-written TypeScript shapes that drift from the Rust types.
- Runtime surprises when a field gets renamed in Rust but not in TS.
- Errors modeled as stringly-typed exceptions on the frontend even though the Rust side has a well-typed enum.

Two serious options exist:

1. **tauri-specta** — a code generator that reads the `#[tauri::command]`-annotated functions plus types that derive `specta::Type`, and emits a TS file with typed wrappers. Active project, integrates with Tauri 2.
2. **Hand-written Zod schemas + runtime validation.** Workable, but verbose and still drifts — the Rust side has no way to enforce that the schema matches.

## Decision

We adopt **tauri-specta** and generate TS bindings at build time.

- Every command struct and enum that crosses the IPC boundary derives `specta::Type` in addition to `serde::Serialize`/`Deserialize` as needed.
- A `cargo build --features generate-bindings` step writes `src/ipc/bindings.ts`. The file begins with a "GENERATED — DO NOT EDIT" header.
- CI runs `pnpm run check:ipc`, which regenerates the file into a temp path and diffs it against the committed version. A drift fails the build.
- The frontend never uses `invoke()` directly. Feature code imports from `src/ipc` and gets strongly-typed wrappers.

## Consequences

Positive:

- **One source of truth.** Rust is canonical. TS follows.
- **Compile-time contract checks on both sides.** A rename in Rust breaks TS compilation, which is exactly what we want.
- **Typed errors.** `AppError` becomes a discriminated TS union; the frontend handles each variant explicitly.

Negative:

- **Build-time coupling.** The generator must run before the frontend typechecks. We integrate it into `pnpm tauri dev` and CI.
- **Generator bugs become our problem.** We pin tauri-specta in `Cargo.toml` and upgrade deliberately.
- **Slightly more boilerplate per type.** Every IPC-crossing type needs `#[derive(specta::Type)]`. Worth it.

## Mitigations

- The CI drift check catches human mistakes (forgetting to regenerate locally).
- A `docs/api-contracts.md` companion document captures intent, so a review can catch cases where the generator produced a technically-valid but semantically-wrong shape.
- Integration tests exercise at least one command + one event end-to-end to verify generation + runtime parity.

## Alternatives considered and rejected

- **No generation, hand-written bindings.** Rejected — drift is inevitable on a multi-month project.
- **OpenAPI-style JSON schema + openapi-typescript.** Rejected — indirection overhead, and the schema generation would duplicate what tauri-specta already does natively.
- **rspc.** Close to tauri-specta in spirit and would work, but introduces an additional runtime layer we do not need for a local IPC boundary.

## Supersedes / superseded by

- None.
