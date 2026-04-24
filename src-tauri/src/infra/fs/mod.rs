//! Filesystem helpers used by the download + library-migration flows.
//!
//! Three concerns:
//!
//! * Atomic-or-copy move across filesystems (`atomic_move`).
//! * Free-space reporting for the disk preflight (`free_bytes`).
//! * Default staging path resolution per platform (`default_staging_dir`).
//!
//! Every public function is testable against `tempfile::tempdir()` —
//! no hidden singletons.

pub mod move_;
pub mod space;
pub mod staging;
