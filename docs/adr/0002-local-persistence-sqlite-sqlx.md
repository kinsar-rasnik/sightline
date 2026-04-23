# ADR-0002 — Local persistence: SQLite via sqlx

- **Status.** Accepted
- **Date.** 2026-04-24
- **Phase.** 1
- **Deciders.** CTO, Senior Engineer

## Context

Sightline stores structured state (streamers, VODs, download tasks, watch progress) and references to binary assets (downloaded videos). The data volume is modest per user — low thousands of rows, tens to hundreds of gigabytes of video — and never leaves the user's machine. We need a storage layer that:

- Is embedded (no service to manage).
- Survives OS sleep, power loss, and sync-provider quirks (Proton Drive, OneDrive, iCloud).
- Has first-class async bindings in Rust.
- Supports migrations and compile-time-checked queries.

Options considered:

- **SQLite + sqlx.** Embedded, durable, universally supported, async bindings via `sqlx::SqlitePool`. Macros give compile-time query verification in dev (opt-in) and a pure-runtime path for CI without DBs.
- **SQLite + rusqlite.** Synchronous. We would need `spawn_blocking` wrappers everywhere, which is friction.
- **Sled / surrealdb-embedded.** Attractive for KV-style access; weak for the relational joins we need for library sorting and multi-view lookups.
- **RocksDB.** Overkill and awkward to query without a schema layer on top.

## Decision

We use **SQLite** via **sqlx** with the `sqlite`, `runtime-tokio`, `macros`, and `migrate` features.

- Schema lives in `src-tauri/migrations/NNNN_<slug>.sql`, append-only.
- App startup runs `sqlx::migrate!()`. A migration failure is a fatal, loud exit — no partial schemas.
- WAL journal mode is enabled at connection open. `synchronous = NORMAL`, `busy_timeout = 5000`.
- All timestamps are UTC integers. See `docs/data-model.md` for the invariants.
- The database file lives at `<library_root>/sightline.sqlite`.

## Consequences

Positive:

- **Zero-dependency deployment.** The DB is a file in the user's chosen library folder.
- **Rich query power.** Joins, window functions, CTEs — everything we need for library ordering and multi-view range queries.
- **Async-friendly.** `SqlitePool` plays well with tokio; no `spawn_blocking` gymnastics.
- **Trivial backup.** Users can copy the sqlite file (with WAL checkpoint) to back up their state.

Negative:

- **Sync-provider caveats.** Dropbox, OneDrive, iCloud, and Proton Drive occasionally lock files during sync. We handle `SQLITE_BUSY` with a retry loop at the infra boundary.
- **No cross-process concurrency.** Fine for a single desktop app; would be a blocker if we ever added a CLI companion.
- **Schema migrations are forward-only.** Down-migrations are explicitly out of scope; we prefer additive schema changes.

## Mitigations

- `infra::db` wraps every statement with a retry-on-busy helper.
- Integration tests write to a temp file on a local disk (never the sync-provider path) to keep CI fast.
- A Phase 7 diagnostic bundle includes a WAL-checkpointed copy of the DB file for support.

## Supersedes / superseded by

- None.
