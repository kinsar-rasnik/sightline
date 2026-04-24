---
name: add-tauri-command
description: Add a new Tauri IPC command end-to-end. Wires the thin command handler, the services entry point, the typed `AppError` mapping, registers the command in the tauri-specta Builder, regenerates the TS bindings, and scaffolds a matching React hook + test. Use whenever a new `#[tauri::command]` is needed.
---

# `add-tauri-command` skill

## When to invoke

- A new feature requires a webview → Rust round-trip (read, mutation, stream-emitting trigger).
- The request adds or changes the IPC surface. If you are only wiring an *event* (backend emits without a frontend call), use the `add-tauri-event` playbook instead.

## Inputs

- Command name in `snake_case` (canonical on the Rust side) and `camelCase` mirror (auto-derived on the TS side by serde + specta).
- Input shape (struct or `()`), output type, and the typed error variant that this command can return.
- Feature folder under `src-tauri/src/commands/` and `src/features/` (create if missing).

## Process

> **Before editing.** Read `docs/adr/0007-ipc-typegen.md` and `.claude/rules/rust-backend.md`. The command layer is thin by rule — 20 lines max.

### Rust side

1. **Domain type.** Add a struct or enum in `src-tauri/src/domain/<feature>.rs`.
   - Derive `Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Type` (specta).
   - `#[serde(rename_all = "camelCase")]` so the TS side gets idiomatic names.
   - Document each field with a doc comment (one line).

2. **Service method.** Add the method in `src-tauri/src/services/<feature>.rs`.
   - Signature: `pub async fn method(&self, input: Input) -> Result<Output, AppError>`.
   - Map lower-level errors (sqlx, reqwest, keyring) via the `From` impls in `src-tauri/src/error.rs`. Add a new `AppError::<Variant>` if no existing one fits.
   - Write the `#[cfg(test)] mod tests` unit test before wiring the command.

3. **Command handler.** Add the handler in `src-tauri/src/commands/<feature>.rs`.
   - Annotate with **both** `#[tauri::command]` and `#[specta::specta]`. Omitting the second annotation will make the command invisible to the generator.
   - Handler body: deserialize inputs, call `Service::method`, return the result. No business logic.
   - Re-export from `src-tauri/src/commands/mod.rs`.

4. **Register.** In `src-tauri/src/ipc.rs` add the handler to the `collect_commands![...]` list, preserving alphabetical order. Events go in `collect_events![...]`.

5. **Verify compile.** Run `cargo build --manifest-path src-tauri/Cargo.toml`.

6. **Regenerate bindings.** Run `cargo test --manifest-path src-tauri/Cargo.toml --test ipc_bindings`. This rewrites `src/ipc/bindings.ts` via `ipc::export_bindings`. Check the diff — the new command should appear under `commands.<camelCaseName>` and any new types under `/* Types */`.

### Frontend side

7. **Wrap.** In `src/ipc/index.ts` add a wrapper to the `commands` object. Use `unwrap(await generatedCommands.xxx(input))` so the component layer sees a throw-style API.

8. **Hook.** In `src/hooks/use-<feature>.ts` build a `useQuery` (read) or `useMutation` (write) around the wrapper. Invalidate the right query keys on mutation success.

9. **Component test.** Add a `.test.tsx` that mocks `commands.<name>` and covers loading, success, and error paths.

### Quality gate

10. Run `pnpm check:ipc` — the diff must be zero.
11. Run `cargo fmt --check && cargo clippy --all-targets --all-features -- -D warnings && cargo test` from `src-tauri/`.
12. Run `pnpm typecheck && pnpm lint && pnpm test`.
13. Commit in one logical change: domain + service + command + ipc registration + bindings + frontend hook + test. PR description notes any new `AppError` variants.

## Validation

Green flow leaves the tree with:

- `git diff --stat` showing edits to: one domain file, one service file, one command file, `ipc.rs`, `bindings.ts`, one hook, one component (or test).
- `pnpm check:ipc` printing nothing.
- `cargo test` reporting one additional unit test at minimum (the service test you wrote in step 2).

## Out of scope

- Data modeling changes that require a new migration — use `add-sqlx-migration` first, then come back.
- Breaking changes to existing command signatures — write an ADR and introduce `<name>_v2`; never edit the existing command in place without migration notes.
- Events without commands — use `add-tauri-event`.

## Examples

### Good — adds `cmd_list_streamers`

1. Adds `domain::streamer::StreamerRow` with `#[derive(Type, Serialize, Deserialize)]`.
2. Adds `services::streamers::StreamerService::list_streamers()` + unit test against in-memory sqlite.
3. Adds `commands::streamers::list_streamers` annotated `#[tauri::command] #[specta::specta]`.
4. Registers in `ipc::ipc_builder()`.
5. Runs `cargo test --test ipc_bindings` — `bindings.ts` diff shows the new command and its row type.
6. Adds `src/hooks/use-streamers.ts` with `useQuery`, and a component test.

### Bad — forgets `#[specta::specta]`

Build passes, runtime works, but the TS side sees no new command and the `check:ipc` drift test is silent (nothing changed in `bindings.ts`). Only manifests as a runtime error when the frontend finally calls it.

### Bad — edits `bindings.ts` by hand

The file starts with `AUTO-GENERATED — DO NOT EDIT`. Hand-edits survive until the next `cargo test --test ipc_bindings` or `pnpm tauri dev` rebuild, then vanish. Always edit on the Rust side and regenerate.
