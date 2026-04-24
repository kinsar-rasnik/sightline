# ADR-0017 — `cargo audit` policy through Phase 6; hardening in Phase 7

- **Status.** Accepted
- **Date.** 2026-04-24 (Phase 4 hotfix)
- **Informs.** CI audit job in `.github/workflows/ci.yml`. Does not
  supersede any prior ADR.

## Context

The `audit` CI job ran `cargo audit --deny warnings`, which elevates every
`unmaintained` / `unsound` warning to an error. On the Phase 4 merge the job
failed with:

- **20 `unmaintained`/`unsound` warnings** that are all transitive through
  the Tauri 2 dependency tree — the GTK3 bindings (`gtk`, `gdk`, `atk`,
  `glib`, …), `fxhash`, `paste`, `proc-macro-error`, and the `unic-*`
  family via `urlpattern → tauri-utils`. None are reachable through code
  we own. None have an upstream fix we can pick up; they are blocked on
  Tauri's own upgrade cycle.
- **1 real `vulnerability`** — RUSTSEC-2023-0071, "Marvin Attack" in
  `rsa 0.9.10`, transitively via `sqlx-mysql 0.8.6`. No fixed upgrade is
  published upstream. We do not compile MySQL support — see below.
- **1 new `unsound` warning** — RUSTSEC-2026-0097 for `rand 0.7.3` via
  `phf_generator → selectors → kuchikiki → tauri-utils`. Also upstream-blocked.

Because the audit job was configured `continue-on-error: true` from Phase 1
the merge was not actually gated, but the job showed red in the PR checks
and was counted among the "three failing jobs" the CTO flagged.

## Decision

Two changes, plus a defence-in-depth Cargo change:

1. **Drop `--deny warnings` from the audit command.** `cargo audit`'s
   default behaviour is to exit non-zero only on `vulnerability`-level
   advisories, which is the bar we actually care about. `unmaintained` and
   `unsound` warnings stay visible in the job log as informational output.

2. **Ignore RUSTSEC-2023-0071 (rsa 0.9.10)** via `cargo audit --ignore`.
   The advisory is reachable only through `sqlx-mysql`, and we do not use
   MySQL. We verified this with `cargo tree -i rsa`, which returns empty
   after the defence-in-depth change below.

3. **Defence-in-depth: `sqlx` → `default-features = false`** with an
   explicit minimal feature list in `src-tauri/Cargo.toml`:

   ```toml
   sqlx = { version = "0.8", default-features = false,
            features = ["runtime-tokio", "sqlite", "macros", "migrate"] }
   ```

   This ensures `sqlx-mysql` is never compiled into the binary. `cargo tree`
   confirms rsa is not in the actual compile graph. (It remains in
   `Cargo.lock` as a dormant optional dep of `sqlx`; Cargo records all
   optional deps referenced by feature expressions even when deactivated —
   see `cargo-issue #13450`. This is why `cargo audit`, which scans the
   lockfile, still surfaces the advisory and why the `--ignore` above is
   still required.)

4. **Keep `continue-on-error: true` on the audit job.** Audit remains
   informational through Phase 6. Phase 7 ("release polish") will harden
   it to a curated allowlist: a `deny.toml` or `audit.toml` that names
   every acceptable exception with an owner and an expiry date, and a
   blocking gate on anything not on the list. This ADR is the bridge;
   ADR-00NN in Phase 7 will close it.

## Alternatives considered

### A. Patch `sqlx` via `[patch.crates-io]` to fork out `sqlx-mysql`

Rejected for a hotfix. Forking is a durable maintenance burden (rebase on
every sqlx release) and the payoff is only satisfying a scanner, not an
actually-reachable vulnerability. If upstream sqlx ever unconditionally
forces MySQL on, we revisit.

### B. `cargo audit --ignore` every transitive advisory from Tauri

Rejected. An `--ignore` list is how Phase 7 will look in production, but
building it now — before we've talked to Tauri upstream about their GTK3
timeline — would bake in a de facto allowlist without review. The
"informational job" posture keeps the noise visible so we notice when
something novel lands, without creating a review bottleneck every time the
Tauri tree shifts.

### C. Gate only on a specific advisory list (opposite of `--ignore`)

Rejected for the same reason as B — it's what Phase 7 will do, not a
hotfix move.

### D. Remove the audit job entirely

Rejected. The job is cheap, the signal is real (the `rsa` vuln is genuine
even if not exploitable for us), and having it in place makes the Phase 7
hardening a configuration change rather than a new job.

## Consequences

**Positive.**
- `audit` job turns green on this branch, unblocking the Phase 4 CI story.
- `sqlx-mysql` is no longer compiled; if a future feature *does* want MySQL
  the change is visible in `Cargo.toml` and triggers re-evaluation.
- The `--ignore RUSTSEC-2023-0071` line carries its own explanation in the
  workflow file, pointing at this ADR — future readers see why it is there
  and what it would take to remove it.

**Costs accepted.**
- `unmaintained`/`unsound` warnings from transitive Tauri deps are no
  longer failing CI. They remain visible in the job log for anyone who
  wants to triage them, but the job does not force action.
- One `--ignore` entry is now a maintenance item. When Phase 7 hardens
  audit to an allowlist, this ignore should move into the allowlist with
  an expiry date.
- We rely on `cargo tree` (actual compile graph) to attest that
  `sqlx-mysql` is not reachable; a future Cargo change to how it records
  optional deps in the lockfile could change this. The ADR would need to
  be revisited in that case.

**Risks.**
- If Tauri 3 (or a Tauri 2 minor update) changes the GTK3 tree such that
  the 20 tolerated warnings flip to a new set of *vulnerabilities*, our
  current policy will not catch the change before it merges (the job is
  `continue-on-error`). Low likelihood, and Phase 7 closes this gap.
- If upstream publishes a fixed `rsa` version we should drop the
  `--ignore` promptly. The Phase 7 allowlist review cycle will catch this;
  in the interim, a manual `cargo audit` dev-box run will show the
  advisory marked as resolved.

## Follow-ups

1. **Phase 7 — Audit allowlist.** Author a `deny.toml` / `audit.toml`
   with one entry per accepted advisory, each with an owner and an
   expiry date. Remove `continue-on-error`. File that ADR when the
   allowlist lands.
2. **Watch rsa.** When a fixed `rsa` version publishes, drop
   `--ignore RUSTSEC-2023-0071` and confirm the job still passes.
3. **Watch Tauri GTK3 migration.** Tauri's roadmap replaces GTK3 with
   GTK4; when that lands in a Tauri release we consume, most of the
   current `unmaintained` noise disappears on its own.
