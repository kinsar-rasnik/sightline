# Hotfix: three Phase-4 CI failures surfaced by cross-platform CI

**Date:** 2026-04-24
**Branch:** `fix/phase-4-ci-failures`
**Base:** `main` @ [`a5edac7`](https://github.com/kinsar-rasnik/sightline/commit/a5edac7)
  (PR #12 — Phase 4 sidecar bundling — squash-merged with three red jobs;
  the erroneous `phase-04` tag has been deleted by the CTO)
**New ADRs:** [ADR-0016](../adr/0016-checks-job-bundles-sidecars.md),
  [ADR-0017](../adr/0017-audit-policy-until-release.md)

## TL;DR

Phase 4 landed real sidecar bundling (ADR-0013) and broke three CI
jobs in three different ways:

1. `scripts/bundle-sidecars.ps1` has a PowerShell string-interpolation
   bug — `"$url: expected..."` is parsed as a scope reference
   (`$url:expected`) and `expected` is not a valid scope name.
   Windows CI fails at parse time, before any download happens.
2. The `checks (fmt · lint · typecheck)` job on Linux runs
   `cargo clippy`, which triggers `tauri-build`. `tauri-build`
   validates that every `externalBin` path exists before the crate
   type-checks. `checks` did not bundle sidecars → path missing →
   clippy fails before it sees a single line of code.
3. The `audit` job ran `cargo audit --deny warnings`, which promotes
   every `unmaintained`/`unsound` advisory to an error. 20 warnings
   come through Tauri's GTK3 dep tree + fxhash + paste +
   proc-macro-error + unic-* + rand 0.7.3 via phf_generator, all
   upstream-blocked. Plus one real vulnerability
   (RUSTSEC-2023-0071, rsa 0.9.10 Marvin Attack) transitive via
   `sqlx-mysql` — a crate we do not actually use.

All three are fixed, one commit per defect, on
`fix/phase-4-ci-failures`. The full quality gate passes locally on
macOS (aarch64). CI confirmation is the next step.

## The three defects

### 1. PowerShell string interpolation fails on `$var:`

[`scripts/bundle-sidecars.ps1:161`](../../scripts/bundle-sidecars.ps1#L161)

```
ParserError: D:\a\sightline\sightline\scripts\bundle-sidecars.ps1:161
Line |
 161 |        throw "sha256 mismatch for $url: expected $sha, got $got"
     |                                   ~~~~~
     | Variable reference is not valid. ':' was not followed by a valid
     | variable name character.
```

PowerShell reads `$var:name` as a scope reference (like
`$env:VAR_NAME`, `$global:foo`). When the character after the colon
is not a valid variable name character (e.g. a space), parsing
fails. This is a parse-time error, so the script cannot even start
before it hits this line.

**Fix:** brace the variables: `${url}`, `${sha}`, `${got}`.

**Audit of the rest of the script:** grepped for `\$\w+:` in
`scripts/bundle-sidecars.ps1` and `scripts/bootstrap.ps1`. The only
other matches are legitimate scope references
(`$env:SIGHTLINE_SIDECAR_CACHE`, `$env:LOCALAPPDATA`). No further fixes
needed.

**Commit:** `fix(scripts): brace PowerShell variables before colons in
interpolation`

### 2. `checks` job missing sidecar bundle step

The Linux `checks` job runs `cargo clippy --all-targets --all-features`.
Clippy compiles `tauri-build`, which at compile time validates every
path in `tauri.conf.json::bundle.externalBin`:

```
resource path `binaries/yt-dlp-x86_64-unknown-linux-gnu` doesn't exist
```

Phase 3's `checks` job got away with this because `externalBin` paths
were stubbed; Phase 4 made them real (ADR-0013). The `test` matrix
bundles sidecars before `cargo test`; `checks` did not.

**Fix:** copy the `cache sidecar downloads` + `bundle sidecars` pair
into the `checks` job. The job runs on `ubuntu-latest` only, so no
matrix conditional is needed — bash path only.

**Alternatives rejected** (recorded in ADR-0016):
- Gate `externalBin` behind a cargo feature off during `checks`.
  Rejected — disabling the validator defeats its purpose.
- Move clippy into the `test` matrix.
  Rejected — wastes cross-platform CI budget on a platform-neutral
  concern and conflates the "fast checks" loop with the longer
  cross-platform tests.

**Commit:** `fix(ci): bundle sidecars in checks job before clippy`

### 3. `audit` job over-strict + one real vuln

`cargo audit --deny warnings` elevates every advisory severity to a
failing error. 20 warnings were tolerable nuisance; one real
vulnerability (`rsa 0.9.10`, transitive via `sqlx-mysql`) was not.

**Three changes:**

1. **Drop `--deny warnings`.** `cargo audit`'s default exit-code
   behaviour fails only on `vulnerability`-level advisories, which is
   the right bar. `unmaintained` / `unsound` warnings stay visible
   in the log.
2. **Disable MySQL in sqlx.** Set
   `default-features = false` on the `sqlx` dep in
   `src-tauri/Cargo.toml`, with an explicit feature list
   (`runtime-tokio`, `sqlite`, `macros`, `migrate`). After this,
   `cargo tree -i rsa` returns empty — rsa is not in the compile
   graph.
3. **`cargo audit --ignore RUSTSEC-2023-0071`.** See "Surprise" below —
   even with mysql disabled, the optional dep stays in Cargo.lock,
   so cargo-audit still surfaces it. The ignore is explicit and
   documented inline.

The audit job stays `continue-on-error: true` per the existing
Phase 1 design — it remains informational until Phase 7 hardens it
to a curated allowlist.

**Commit:** `fix(ci): audit policy — drop --deny warnings, ignore
transitive rsa`

## Surprise: `default-features = false` does not remove optional deps
## from `Cargo.lock`

The CTO prescribed "disable the `mysql` feature on sqlx … to drop
the rsa dependency from the tree entirely; verify with `cargo tree
-i rsa`". I did the first half — set `default-features = false`,
re-resolved with `cargo check`, and `cargo tree -i rsa` indeed
returns "nothing to print", confirming rsa is not compiled.

But `grep 'name = "rsa"' src-tauri/Cargo.lock` still finds it, and
so does `cargo audit`. Cargo records optional deps referenced by
`?`-conditional feature expressions (like `"migrate": ["sqlx-mysql?/migrate"]`)
in the lockfile even when no feature activates them — this is a
long-standing Cargo behaviour; cargo#13450 has a representative
discussion.

This means the CTO's verification step (`cargo tree -i rsa`) passes
but the downstream goal (`cargo audit` passes) does not, because
cargo-audit scans Cargo.lock, not the compile graph.

I kept the defence-in-depth change (it is cheap, it is correct, and
it means any future regression that quietly enables mysql shows up
in a `Cargo.toml` diff), and added `--ignore RUSTSEC-2023-0071` with
a five-line comment in `.github/workflows/ci.yml` explaining why it
is there and what it would take to remove it. Full rationale in
ADR-0017.

No upstream fix for `rsa 0.9.10` exists ("No fixed upgrade is
available!" — RustSec). The vulnerability is a timing side-channel
reachable only through RSA operations initiated by sqlx-mysql. We
do not compile sqlx-mysql. The ignore is safe.

## Evidence: local quality gate

All six gates green on macOS `aarch64-apple-darwin` at
`fix/phase-4-ci-failures` tip:

```
cargo fmt --all -- --check     → clean
cargo clippy --all-targets --all-features -- -D warnings → clean (~14s)
cargo test --all-features       → 24+ tests pass, sidecar smoke tests
                                   ffmpeg/yt-dlp report versions
pnpm typecheck                 → clean
pnpm lint                      → clean
pnpm test                      → 5 files / 24 tests, all pass

cargo audit --ignore RUSTSEC-2023-0071
  → 20 allowed warnings, exit 0
cargo tree -i rsa              → empty (rsa not in compile graph)
cargo tree -i sqlx-mysql       → empty (sqlx-mysql not in compile graph)
```

I could not run the Windows path locally; that one is proven only
by inspection (the `${url}:` rewrite is the standard PowerShell
interpolation escape) and by the three-OS CI matrix confirming.

## Commits on the branch

```
9a2d9b9 fix(ci): audit policy — drop --deny warnings, ignore transitive rsa
dd4e71e fix(ci): bundle sidecars in checks job before clippy
ae0dcfa fix(scripts): brace PowerShell variables before colons in interpolation
```

Three commits, one per defect, each self-contained and Conventional
Commits format. Each commit passes the quality gate independently;
the branch tip passes the full gate.

## PR and CI

**PR:** _filled in after push_
**CI runs:** _filled in after CI green_

## Follow-ups (Phase 7)

1. Replace `--ignore RUSTSEC-2023-0071` with an allowlist file
   (`audit.toml` or `deny.toml`) that names every tolerated advisory
   with an owner and an expiry date (ADR-0017).
2. Drop the ignore when upstream publishes a fixed `rsa` version.
3. Revisit the GTK3-unmaintained warnings when Tauri's roadmap lands
   GTK4 in a release we consume (most of the current 20 disappear
   on their own).
4. Drop the `default-features = false` workaround if Cargo ever
   changes how it records optional-but-not-activated deps in the
   lockfile (cargo#13450).
