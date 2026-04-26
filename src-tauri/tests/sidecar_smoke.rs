#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Integration smoke test for the bundled sidecars.
//!
//! Two layers:
//!
//! 1. **Real-binary smoke (legacy).** Confirms that the binaries
//!    produced by `scripts/bundle-sidecars.sh` / `.ps1` are locatable
//!    under `src-tauri/binaries/` and can answer `--version`.  Skipped
//!    on a fresh clone that hasn't run the bundler yet; CI always runs
//!    it.
//!
//! 2. **Bundle-layout simulation (v2.0.2).** Synthesises each OS's
//!    bundle directory layout in a tempdir and asserts that
//!    `sightline_lib::find_sidecar_in_dir` discovers the sidecar in
//!    the same place a real `pnpm tauri build` produces it.  Catches
//!    the regression that shipped on v2.0.1 — the macOS `.app` was
//!    built correctly but the runtime resolver looked in
//!    `Contents/Resources/` (Tauri 1 layout) instead of
//!    `Contents/MacOS/` (Tauri 2 layout).
//!
//! See `docs/adr/0013-sidecar-bundling.md` and `docs/adr/0034-tauri2-
//! sidecar-layout.md`.

use std::path::PathBuf;
use std::process::Command;

use sightline_lib::find_sidecar_in_dir;

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

// ---------------------------------------------------------------
// v2.0.2 bundle-layout simulation tests.
// ---------------------------------------------------------------
//
// These run on every OS in CI without needing a real bundle — they
// just synthesise the directory layout each platform's installer
// produces and verify that `find_sidecar_in_dir` looks in the right
// place. The smoke tests above prove the binaries actually start;
// these prove the resolver actually finds them.

/// Touch a zero-byte placeholder at `path`, creating parent dirs.
fn touch(path: &std::path::Path) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(path, b"").unwrap();
}

#[test]
fn macos_app_layout_resolves_sidecar_under_contents_macos() {
    // macOS .app bundle: every executable (main + sidecars) lives in
    // Contents/MacOS/. This is the layout that v2.0.1 shipped in but
    // the runtime resolver missed.
    let tmp = tempfile::tempdir().unwrap();
    let macos_dir = tmp
        .path()
        .join("Sightline.app")
        .join("Contents")
        .join("MacOS");
    let triple = "aarch64-apple-darwin";
    touch(&macos_dir.join("sightline"));
    touch(&macos_dir.join(format!("yt-dlp-{triple}")));
    touch(&macos_dir.join(format!("ffmpeg-{triple}")));

    let yt_dlp = find_sidecar_in_dir(&macos_dir, "yt-dlp", triple, "");
    assert_eq!(yt_dlp, Some(macos_dir.join(format!("yt-dlp-{triple}"))));
    let ffmpeg = find_sidecar_in_dir(&macos_dir, "ffmpeg", triple, "");
    assert_eq!(ffmpeg, Some(macos_dir.join(format!("ffmpeg-{triple}"))));
}

#[test]
fn linux_deb_layout_resolves_sidecar_alongside_main_binary() {
    // .deb installs to /usr/bin/ on most distros. The exact path is
    // OS-controlled; the invariant we care about is "same dir as the
    // launched binary".
    let tmp = tempfile::tempdir().unwrap();
    let bin_dir = tmp.path().join("usr").join("bin");
    let triple = "x86_64-unknown-linux-gnu";
    touch(&bin_dir.join("sightline"));
    touch(&bin_dir.join(format!("yt-dlp-{triple}")));
    touch(&bin_dir.join(format!("ffmpeg-{triple}")));

    let yt_dlp = find_sidecar_in_dir(&bin_dir, "yt-dlp", triple, "");
    assert_eq!(yt_dlp, Some(bin_dir.join(format!("yt-dlp-{triple}"))));
}

#[test]
fn linux_appimage_layout_resolves_sidecar_in_runtime_mount() {
    // AppImage mounts to /tmp/.mount_<random> at run time; the
    // structure inside is `<mount>/usr/bin/sightline` plus sidecars.
    let tmp = tempfile::tempdir().unwrap();
    let mount = tmp.path().join(".mount_sightABC").join("usr").join("bin");
    let triple = "aarch64-unknown-linux-gnu";
    touch(&mount.join("sightline"));
    touch(&mount.join(format!("yt-dlp-{triple}")));

    let resolved = find_sidecar_in_dir(&mount, "yt-dlp", triple, "");
    assert_eq!(resolved, Some(mount.join(format!("yt-dlp-{triple}"))));
}

#[test]
fn windows_msi_layout_resolves_sidecar_with_exe_extension() {
    // Windows installer drops everything in
    // C:\Program Files\Sightline\. Sidecars get an `.exe` suffix on
    // top of the canonical name-triple pattern.
    let tmp = tempfile::tempdir().unwrap();
    let install_dir = tmp.path().join("Program Files").join("Sightline");
    let triple = "x86_64-pc-windows-msvc";
    touch(&install_dir.join("sightline.exe"));
    touch(&install_dir.join(format!("yt-dlp-{triple}.exe")));
    touch(&install_dir.join(format!("ffmpeg-{triple}.exe")));

    let yt_dlp = find_sidecar_in_dir(&install_dir, "yt-dlp", triple, ".exe");
    assert_eq!(
        yt_dlp,
        Some(install_dir.join(format!("yt-dlp-{triple}.exe")))
    );
}

#[test]
fn missing_sidecar_returns_none() {
    // If neither `<name>-<triple>` nor `<name>` exists, the resolver
    // must signal absence so the caller can fall through to its next
    // candidate (legacy Resource lookup, dev-repo path).
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    touch(&dir.join("sightline"));
    let res = find_sidecar_in_dir(dir, "yt-dlp", "x86_64-unknown-linux-gnu", "");
    assert_eq!(res, None);
}

#[test]
fn name_only_fallback_resolves_when_triple_form_missing() {
    // Tauri's bundler sometimes strips the triple suffix from the
    // bundled filename (older Tauri 1 paths, or future bundle formats
    // that normalise per-platform). Fallback to bare `<name>[.exe]`.
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    touch(&dir.join("ffmpeg"));
    let res = find_sidecar_in_dir(dir, "ffmpeg", "aarch64-apple-darwin", "");
    assert_eq!(res, Some(dir.join("ffmpeg")));
}

#[test]
fn canonical_name_triple_takes_precedence_over_bare_name() {
    // Both forms present — the canonical name-triple path wins so a
    // multi-arch dev install with two `ffmpeg` binaries doesn't
    // accidentally pick the wrong architecture.
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let triple = "aarch64-apple-darwin";
    touch(&dir.join("ffmpeg"));
    touch(&dir.join(format!("ffmpeg-{triple}")));
    let res = find_sidecar_in_dir(dir, "ffmpeg", triple, "");
    assert_eq!(res, Some(dir.join(format!("ffmpeg-{triple}"))));
}
