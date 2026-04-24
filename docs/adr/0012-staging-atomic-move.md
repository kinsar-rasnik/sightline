# ADR-0012: Staging + atomic-move strategy for downloads

- **Status.** Accepted (2026-04-24).
- **Phase.** 3.
- **Deciders.** Senior Engineer (Claude), reviewing CTO.

## Context

yt-dlp writes `.part` files during a download and renames on
completion. Two problems with that default for Sightline:

1. **Proton Drive / iCloud / Dropbox** intercept `.part` files and
   `rename` syscalls in unpredictable ways. Phase 1 hit this already
   (`hotfix-ci.md`-adjacent — we pinned `modules-dir` outside the
   sync folder). A download that finishes at 99% and then stalls on
   rename for five minutes is a bad UX and an open question about
   whether the file landed safely.
2. **Cross-filesystem library roots.** The typical enthusiast's setup
   is a small fast local disk for working files and a large external
   drive (or NAS) for media. yt-dlp's `.part → .mp4` rename is
   atomic only on the same filesystem. If we let it rename across
   filesystems, it silently falls back to copy+delete without
   verification — i.e. a bit flip mid-copy produces a corrupted
   file with no way to notice.

## Decision

Every download flows through a **staging → post-process → atomic
move** pipeline:

1. **Staging directory** is always on local disk, outside any sync
   provider. Platform defaults:
   - macOS — `~/Library/Caches/sightline/staging`
   - Linux — `$XDG_CACHE_HOME/sightline/staging`
   - Windows — `%LOCALAPPDATA%\sightline\staging`
   A user override exists but is rejected if it lives under the
   library root (`validate_override` enforces this).
2. yt-dlp writes into staging with `--no-part` set when the staging
   path looks like a sync-provider mount. `--no-part` makes yt-dlp
   write directly to the final filename with no rename, which the
   sync layer handles better.
3. **Post-process:** if the output isn't `.mp4`, ffmpeg container-
   swaps with `-c copy`. A single-frame thumbnail is extracted at
   10% of the duration.
4. **Atomic move into the library:**
   - Same filesystem → `std::fs::rename`. This is atomic at the
     syscall level.
   - Cross-filesystem (`EXDEV`) → copy + fsync-verify-SHA256 +
     delete-source. Any hash mismatch aborts the move and reports an
     error; the staging file is left in place for the next retry.
5. **Sidecars** (NFO for Plex, thumbnail for both layouts) are
   written under the final directory after the move succeeds.
6. **Crash recovery:** on startup, any `downloads` row in
   `downloading` state is reset to `queued`. The partial staging
   file is treated as garbage (yt-dlp's resume flag is not robust
   across cloud filesystems), and the download restarts from zero.
   Stale staging files > 48 h old are swept by
   `infra::fs::staging::cleanup_stale`.

## Consequences

### Positive

- Cross-filesystem moves are verified, not silently corrupted.
- Proton Drive is first-class: `--no-part` + local staging keeps
  yt-dlp's behaviour predictable.
- Crash recovery is simple and deterministic. "Partial staging files
  are garbage" is a strict policy — no heuristics about whether to
  resume.
- Sidecars are written last, so a partial move never leaves
  orphaned NFOs pointing at a missing video.

### Negative / accepted risks

- **Double-write on cross-FS moves.** A full copy runs through CPU +
  memory. For a 5 GB VOD on a slow USB drive this can take minutes.
  Acceptable: users know when their library is on a slow target.
- **No resume across crashes.** A 3-hour download that crashes at
  95% starts from zero. We document this in the user guide under
  "what happens when my laptop sleeps".
- **Staging fills fast.** A batch of concurrent downloads at
  "source" quality can blow through a local-disk cache. We enforce
  a 1.2× disk-space preflight on the staging partition before
  enqueueing (see `check_preflight`), and the UI surfaces staging
  info in Settings.
- **SHA-256 hashing is CPU-bound.** For TB-scale migrations this
  would be a problem; for per-VOD copy-verify it's fine (a 2 GB
  file verifies in ~6 seconds on modern hardware).

### Neutral

- The staging path is user-overridable via Settings. The validator
  rejects paths inside the library root (would defeat the whole
  point) and paths that aren't writable.

## Alternatives considered

1. **Trust yt-dlp's internal rename.** The whole reason this ADR
   exists.
2. **Write directly into the library with no staging.** Faster, but
   leaves half-written `.part` files scattered across the user's
   media library and tools like Plex try to scan them.
3. **Skip SHA-256 verification; trust the copy.** A modern SSD is
   reliable enough that this would work 99.99% of the time. The
   0.01% case is silent data corruption, which we'd never diagnose.
   Rejected.
4. **Hardlink instead of copy when possible.** Only works on the
   same filesystem. The whole point of the copy path is to handle
   cross-FS, so hardlink optimisation is strictly an addition.
   Out of scope for Phase 3; could add later.

## References

- `src-tauri/src/infra/fs/move_.rs` — atomic_move + cross-FS fallback.
- `src-tauri/src/infra/fs/staging.rs` — default paths + sweep.
- `src-tauri/src/infra/fs/space.rs` — preflight checks.
- `src-tauri/src/services/downloads.rs` — pipeline integration.
- ADR-0003 — pinned sidecar binaries (yt-dlp + ffmpeg).
