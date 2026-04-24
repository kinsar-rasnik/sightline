#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Integration smoke test for the bundled sidecars.
//!
//! Confirms that the real yt-dlp + ffmpeg binaries produced by
//! `scripts/bundle-sidecars.sh` / `.ps1` are locatable at the path the
//! runtime resolver expects, exit 0 when invoked with a `--version`
//! flag, and emit a recognisable version string on stdout.
//!
//! Skipped when the host has not run `bundle-sidecars` yet (common on a
//! fresh clone where the developer only wants to iterate on docs or the
//! frontend). CI always runs `bundle-sidecars` before `cargo test`, so
//! the smoke check always fires there.
//!
//! This is the first time real sidecars execute under `cargo test`; see
//! docs/adr/0013-sidecar-bundling.md § "CI integration".

use std::path::PathBuf;
use std::process::Command;

fn target_triple() -> &'static str {
    env!("TARGET_TRIPLE")
}

fn sidecar_path(name: &str) -> PathBuf {
    let ext = if cfg!(target_os = "windows") {
        ".exe"
    } else {
        ""
    };
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("binaries");
    p.push(format!("{name}-{}{ext}", target_triple()));
    p
}

fn run_with_flag(path: &PathBuf, flag: &str) -> Result<String, String> {
    let out = Command::new(path)
        .arg(flag)
        .output()
        .map_err(|e| format!("spawn {}: {e}", path.display()))?;
    if !out.status.success() {
        return Err(format!(
            "{} {} exited {:?}: {}",
            path.display(),
            flag,
            out.status,
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    Ok(format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    ))
}

#[test]
fn yt_dlp_reports_a_version_string() {
    let path = sidecar_path("yt-dlp");
    if !path.exists() {
        eprintln!(
            "skipping: {} missing — run scripts/bundle-sidecars.sh",
            path.display()
        );
        return;
    }
    let stdout = run_with_flag(&path, "--version").unwrap_or_else(|e| panic!("{e}"));
    // yt-dlp `--version` prints a single line like "2026.03.17".
    let trimmed = stdout.trim();
    assert!(
        !trimmed.is_empty() && trimmed.chars().next().is_some_and(|c| c.is_ascii_digit()),
        "unexpected yt-dlp --version output: {trimmed:?}"
    );
}

#[test]
fn ffmpeg_reports_a_version_banner() {
    let path = sidecar_path("ffmpeg");
    if !path.exists() {
        eprintln!(
            "skipping: {} missing — run scripts/bundle-sidecars.sh",
            path.display()
        );
        return;
    }
    let out = run_with_flag(&path, "-version").unwrap_or_else(|e| panic!("{e}"));
    // ffmpeg `-version` always starts the first line with "ffmpeg version".
    assert!(
        out.contains("ffmpeg version"),
        "unexpected ffmpeg -version output: {out:?}"
    );
}
