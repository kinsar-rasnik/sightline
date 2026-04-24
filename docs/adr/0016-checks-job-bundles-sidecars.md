# ADR-0016 — The `checks` CI job bundles sidecars before clippy

- **Status.** Accepted
- **Date.** 2026-04-24 (Phase 4 hotfix)
- **Informs.** [ADR-0013](0013-sidecar-bundling.md) (operational detail of the
  bundling contract). Does not supersede.

## Context

The `checks` job runs `cargo clippy --all-targets --all-features -- -D warnings`
on `ubuntu-latest`. `cargo clippy` runs `tauri-build` as part of compilation,
and `tauri-build` validates that every path listed in
`tauri.conf.json::bundle.externalBin` exists on disk before it lets the crate
type-check:

```
resource path `binaries/yt-dlp-x86_64-unknown-linux-gnu` doesn't exist
```

The `test` matrix (three OS) runs `scripts/bundle-sidecars.sh` before
`cargo test`. The `checks` job did not — the original assumption was "clippy
doesn't need the binaries, only the test run does." ADR-0013 introduced real
sidecar bundling in Phase 4 but did not update the `checks` job, so the job
broke the moment Tauri's `externalBin` validator ran ahead of clippy.

Result: `checks` failed on `main` after Phase 4 merged. The CTO hotfix
instructed us to make `checks` bundle sidecars the same way the test matrix
does.

## Decision

Add `cache sidecar downloads` + `bundle sidecars` steps to the `checks` job,
using the same script (`scripts/bundle-sidecars.sh`) and the same per-OS
cache key that the test matrix uses. `checks` runs on `ubuntu-latest` only,
so only the Linux path is needed — no matrix, no `if: matrix.os == …` guard.

```yaml
- name: cache sidecar downloads
  uses: actions/cache@v4
  with:
    path: ~/.cache/sightline-sidecars
    key: sidecars-${{ runner.os }}-${{ hashFiles('scripts/sidecars.lock') }}

- name: bundle sidecars
  shell: bash
  run: ./scripts/bundle-sidecars.sh
```

Placed between "install linux webview deps" and "cargo fmt", because `cargo fmt`
is the first Rust-touching step.

## Alternatives considered

### A. Stub out `externalBin` behind a cargo feature that's off during `checks`

Rejected. Would require splitting `tauri.conf.json` behind a feature gate, a
separate build profile, or a `build.rs` mutation that writes the config on the
fly. Each of these adds two layers of indirection (the Tauri config DSL and a
feature flag) for the sole purpose of silencing a check that exists to catch
real missing-bundle bugs. ADR-0013's whole point is that externalBin
validation is a hard gate — we do not want a clippy-only escape hatch.

### B. Move clippy into the `test` matrix

Rejected. Clippy is a single-OS concern (platform-neutral lints); running it
three times wastes CI minutes and conflates the "fast checks" feedback loop
with the "cross-platform tests" feedback loop that takes longer.

### C. Run `cargo clippy --no-default-features` to skip `tauri-build`

Rejected. The Tauri build script runs regardless of crate features on the
workspace — it's a `[build-dependencies]` hook — and disabling it would turn
off the very check that guards `externalBin`. Plus `--all-features` is the
whole point of the clippy step.

## Consequences

**Positive.**
- `checks` is green again on `ubuntu-latest` after the Phase 4 `externalBin`
  validation was introduced.
- Cold runs cost one sidecar download per OS key, then every subsequent push
  reuses the cache keyed on `scripts/sidecars.lock` (identical key to the
  `test` matrix, so the two jobs can warm each other's cache when the key
  happens to match the per-runner partition).
- Clippy now sees the real `tauri-build` path, which means a future
  regression that invalidates `externalBin` (e.g. lockfile desync with
  `tauri.conf.json`) fails both `checks` and `test`, not just `test`.

**Costs accepted.**
- `checks` now downloads 80–140 MB of sidecars on cold cache. Amortised to
  zero on every run after the first for a given lockfile hash.
- Two more workflow steps to maintain in parallel with the test matrix.
  The verify step (`verify-sidecars.sh --smoke`) is intentionally *not*
  mirrored into `checks` — clippy doesn't run the binaries, it just needs
  the paths present.

**Risks.**
- A future refactor that moves clippy to a different job will need to copy
  these steps along with it. Low risk, caught by the same "missing path"
  error if forgotten.

## Follow-ups

None required. The change is mechanical and documented inline in
`.github/workflows/ci.yml` with a comment pointing back at this ADR.
