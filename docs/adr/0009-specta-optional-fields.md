# ADR-0009: Specta `#[specta(optional)]` on partial-input DTOs

- **Status.** Accepted (2026-04-24).
- **Phase.** 3 housekeeping (post Phase 2).
- **Deciders.** Senior Engineer (Claude), reviewing CTO.

## Context

`tauri-specta` generates TypeScript types from Rust `#[derive(Type)]`
structs. Its default emission for `Option<T>` on a named struct field is
`T | null` as a **required** key, not `T?: T` as an optional key.

That was already flagged as a usability wart in
`docs/session-reports/hotfix-camelcase.md` §Follow-ups: to call
`update_settings({ enabledGameIds })` the frontend had to spread a
six-key `EMPTY_PATCH` baseline with every other key explicitly set to
`null`. `VodFilters` had the same shape. Both are partial-input
command payloads; every field is semantically optional.

Two options were on the table:

1. Keep the Rust shape and paper over it on the frontend with a helper
   (`emptyPatch()`, `emptyFilters()`).
2. Change the Rust shape so the generator emits genuinely optional TS
   properties.

## Decision

**Option 2.** `SettingsPatch` and `VodFilters` annotate each
`Option<T>` field with `#[specta(optional)]` and add
`#[serde(default)]` at the struct level so deserializing JSON with
missing keys is accepted and yields `None` for those fields.

Concretely, the Rust type looks like:

```rust
#[derive(Debug, Default, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", default)]
pub struct SettingsPatch {
    #[specta(optional)]
    pub enabled_game_ids: Option<Vec<String>>,
    // ... remaining Option<T> fields
}
```

The generator then emits `enabledGameIds?: string[] | null`, which lets
the frontend write `update.mutate({ enabledGameIds })` without spread
boilerplate.

`#[serde(skip_serializing_if = "Option::is_none")]` was considered but
`specta` rejects it ("unified mode cannot represent conditional
omission") because the same type must represent both the
serializing and deserializing side. Since these structs only flow
frontend → Rust, the skip isn't actually needed — deserialization with
`default` already accepts missing keys.

## Consequences

### Positive

- Frontend call sites lose the `EMPTY_PATCH` / `{ gameIds: null, ... }`
  spread. Three call sites cleaned up
  (`SettingsPage::GameFilterSection`, `SettingsPage::PollIntervals`,
  `LibraryPage::input`).
- The TS shape accurately reflects the Rust semantics: "this field is
  optional, omit it if you don't care."
- `pnpm typecheck` still catches any typo that lands a wrong key —
  the optionality doesn't loosen correctness.

### Negative / accepted risks

- **Divergence from the other input types.** `SetTwitchCredentialsInput`
  has no `Option<T>` fields (both client id + secret are required) so
  it isn't affected. `AddStreamerInput`, `RemoveStreamerInput`,
  `GetVodInput`, `TriggerPollInput` were audited: only `TriggerPollInput`
  carries an `Option<String>`, and its semantics ("omit to poll every
  due streamer") are exactly the optional-key meaning. Left for a
  follow-up rather than expanding this ADR's scope — the user-visible
  impact is zero (the frontend currently passes `null` and the
  one-key baseline pattern is not in use there).
- **The drift test re-regenerates `src/ipc/bindings.ts`.** That's
  expected on any schema change; the test `tests/ipc_bindings.rs`
  keeps the committed file in sync on every CI run.

### Neutral

- Release builds read the committed `bindings.ts`; debug builds
  regenerate it on startup. No runtime behavior change from this ADR.

## Alternatives considered

1. **Emit a helper `emptyPatch()` / `emptyFilters()` from `@/ipc`.**
   Less work in Rust but ships a foot-gun: the helper returns
   `{ key1: null, ..., key6: null }` and every caller still has to
   remember to spread it. Rejected.
2. **Make the backend accept a top-level key map (`PATCH` semantics,
   one field at a time).** Heavier API surface, worse for atomicity of
   multi-field edits. Rejected.
3. **Use `#[specta(skip)]` and build a hand-written TS type for these
   two DTOs.** Breaks the drift-check invariant from ADR-0007.
   Rejected.

## References

- ADR-0004 — typed IPC via tauri-specta.
- ADR-0007 — IPC typegen drift-check test.
- `docs/session-reports/hotfix-camelcase.md` §Follow-up #3 — where
  this came up originally.
- specta's `#[specta(optional)]` attribute:
  <https://docs.rs/specta/latest/specta/attr.Type.html>.
