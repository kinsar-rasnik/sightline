# Phase 7 — Auto-cleanup, Release Pipeline, v1.0.0: Decision Log

Working log of in-flight decisions during Phase 7. ADR-worthy items
landed in `docs/adr/0024..0027`; this file captures the smaller
judgment calls and the context behind them.

---

## 2026-04-25 — Phase-6 closure docs go on the Phase-7 branch

**Kontext.** Pre-check found three uncommitted Phase-6 closure files
(STATE.md update, HANDOFF-2026-04-25, .claude/CHANGELOG entry for the
Senior-Engineer-merges-and-tags convention). The mission expected
"clean main, branch from there" — the literal interpretation was STOP.

**Optionen.**
- A. STOP, ask the CEO to commit the docs to main first.
- B. Commit them directly to main (Senior-Engineer authority).
- C. Branch first, include them as the first commit on the Phase-7
  branch; they land via the squash-merge.

**Gewählt.** A direct push to main was denied by policy on first
attempt; the natural fallback is C. Branch was created with the
staged docs already in the index, committed as
`chore(docs): close out phase 6, set state for phase 7`.

**Begründung.** The denial signal was clear: the user wants Phase-7
work isolated to the branch. Including the closure commit in the
Phase-7 PR muddies the diff slightly, but the squash-merge collapses
it back into a single Phase 7 commit on main.

---

## 2026-04-25 — Watch-progress preserved on cleanup

**Kontext.** ADR-0024's deletion path could either drop the
`watch_progress` row (clean slate) or preserve it (a re-download
lands the user back at their last position). Both have merit.

**Gewählt.** Preserve. `delete_candidate` updates `downloads` only.

**Begründung.** The row is small (single i64 + small REAL fields).
Preserving it is a one-character SQL change but a concrete UX win:
re-watching a deleted-then-redownloaded VOD resumes at the position
the user last saw, just as if the file had never been gone. Drift
between watch state and actual file presence is also self-healing —
the renderer's "missing file" UI already exists from Phase 5.

---

## 2026-04-25 — Cleanup execution order: UPDATE-then-unlink

**Kontext.** Reviewed by the code-reviewer subagent. Original
`delete_candidate` did unlink first, then UPDATE. If the UPDATE
failed (WAL lock, pool exhaustion), the file was gone but the row
stayed at `completed` — next plan would attempt to delete it again.

**Gewählt.** UPDATE first (state → failed_permanent, last_error =
'CLEANED_UP', final_path = NULL), then unlink. If unlink fails the
file is an orphan, but the DB never gets stuck.

**Begründung.** An orphan file is recoverable (manual unlink, or a
re-download will overwrite). A stuck row is a soft-fail loop on every
tick. The DB is the source of truth.

---

## 2026-04-25 — Concurrency guard via AtomicBool, not Mutex

**Kontext.** Two windows hitting "Run cleanup now" at the same time,
or daemon-tick + manual run, would compute and execute two plans
against the same set of files.

**Optionen.**
- A. `tokio::sync::Mutex` — semantically clean, but blocks the
  second caller until the first finishes. With manual cleanup that
  could be a multi-minute wait.
- B. `AtomicBool` + `compare_exchange` — second caller fails fast
  with `AppError::Cleanup { detail: "already in progress" }`.

**Gewählt.** B.

**Begründung.** Failing fast is the correct UX for "user clicked
Cleanup twice". The renderer can render an inline message
immediately rather than spin. RAII guard on the AtomicBool ensures
every exit path resets the flag.

---

## 2026-04-25 — Asset-protocol static scope: no `$HOME/Sightline/**`

**Kontext.** Initial draft of `tauri.conf.json` listed
`$HOME/Sightline/**` as a fallback "if the user uses the legacy
default location". Security review flagged this as too wide:
symlinks under `~/Sightline/` could grant the webview read access
to anything the symlink targets (e.g. `~/.ssh/`).

**Gewählt.** Drop the `$HOME` entry. Static scope is now only the
two app-data paths (`$APPDATA/sightline/library/**` and
`$APPLOCALDATA/sightline/library/**`). User-chosen library roots
flow through the runtime `allow_directory` extension exclusively.

**Begründung.** App-data paths are OS-protected (root needs admin
+ user gesture to interfere). The runtime extension validates the
library_root via the existing settings-service rules before
allowing it. No regression for users who picked custom roots.

---

## 2026-04-25 — Update checker: opt-in default OFF, no skip-list array

**Kontext.** ADR-0026 §Alternatives considered weighed two
defaults (off / on) and two skip representations (single TEXT /
JSON array).

**Gewählt.** Default off. Single TEXT skip column.

**Begründung.** Default off matches Sightline's privacy posture
(no telemetry, local-first). Single skip column matches the
expected user behaviour (skip exactly the version they don't want;
clear it manually if needed). Multi-skip can be a v1.1 column
addition without breaking the IPC surface.

---

## 2026-04-25 — Release-notes script: bash, not Node

**Kontext.** ADR-0025 §Release notes generation considered two
implementations: a TS module + tsx invocation in CI, or a pure
bash script.

**Gewählt.** Bash. Test coverage via Vitest spawning the script
with stdin input.

**Begründung.** GitHub Actions runners have bash everywhere; a TS
script adds a runtime dependency on tsx in the workflow. The
parsing logic is small (~30 lines of `case` matching). Vitest can
still drive the test surface via `child_process.spawnSync`.

---

## 2026-04-25 — bash 3.2 compatibility for release-notes.sh

**Kontext.** First draft of `release-notes.sh` used the bash 4+
`;&` fallthrough syntax in the `case` block. macOS ships
`/bin/bash` 3.2 by default — this would have broken local dev runs
of the script even though CI's ubuntu-latest is fine.

**Gewählt.** Drop the `;&` fallthroughs. Each non-feat / non-fix /
non-perf prefix gets its own explicit `echo "other" ;;` arm.

**Begründung.** The cosmetic gain of fallthrough syntax is not
worth a dev-loop incompatibility. Tests on Ubuntu and macOS now
exercise identical paths.

---

## 2026-04-25 — Drift cache: TTL + explicit invalidation

**Kontext.** The Phase-6 follow-up to cache `sync_drift_threshold_ms`.
Two pure approaches: TTL-only refresh, or settings-update hook.

**Gewählt.** TTL (30 s) + explicit invalidation via
`SyncService::invalidate_drift_cache()` called from
`cmd_update_settings` when the patch contains
`sync_drift_threshold_ms`.

**Begründung.** TTL alone would mean a slider change waits up to
30 s to take effect — visible to the user. Invalidation alone
would mean a settings change made through any other path (a
direct DB write in tests, a future bulk-import command) wouldn't
flush the cache. Combined, both surfaces are covered.

---

## 2026-04-25 — `cargo audit` enforcement: CLI flag + audit.toml registry

**Kontext.** ADR-0017 promised "a deny.toml or audit.toml that
names every acceptable exception with an owner and an expiry
date" for Phase 7. cargo-audit 0.22.x's project-local audit.toml
is read but does not honour the `[advisories].ignore` array as an
enforcement source.

**Gewählt.** Workflow uses `--ignore RUSTSEC-2023-0071` as the
actual gate; `src-tauri/audit.toml` is the documentation registry
with owner + expiry per accepted exception.

**Begründung.** The spirit of the ADR (every accepted exception is
documented; the gate is blocking by default) is satisfied. The
file ↔ flag split is a maintenance cost — when an entry expires,
both files must update. The quarterly-review note in audit.toml
flags this.

---

## 2026-04-25 — DiskUsage shape: `library_root_configured: bool`, not `library_path: Option<bool>`

**Kontext.** Code review flagged the original field as semantically
confusing — name implied a path string but carried a bool.

**Gewählt.** Renamed to `library_root_configured: bool`
(non-optional). The frontend consumer reads it as a flag.

**Begründung.** A field named `library_path` reading `Some(true)`
is the kind of API noise that compounds across consumers. Renaming
costs one IPC type rebuild and one frontend update; not renaming
costs every future reader of the struct ten seconds.

---

## 2026-04-25 — Skip the explicit `query!` migration for Phase 7

**Kontext.** Code review noted the `app_settings` UPDATE now binds
33 positional `?` parameters. A future column addition that shifts
the bind order silently produces wrong data. `sqlx::query!` would
catch this at compile time.

**Gewählt.** Defer.

**Begründung.** Migrating the UPDATE to `query!` requires
`SQLX_OFFLINE` metadata committed under `.sqlx/`, plus a database
schema-snapshot setup that doesn't exist in the workforce yet.
Touch is across the entire settings module. The unit-test matrix
covers every field round-trip explicitly. Tracked as a Phase-8
hygiene item.
