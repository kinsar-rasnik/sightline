# Installing Sightline

Sightline ships **unsigned binaries** on the GitHub Releases page. The OS will warn the first time you launch one because we don't pay for an Apple Developer ID or a Microsoft EV code-signing certificate. The warning isn't malware-specific — every unsigned indie app trips it. This page walks through the per-OS workaround.

If you'd rather build from source, the [Build from source](#build-from-source) section at the bottom covers that path.

---

## macOS

> **Intel Mac note.** GitHub retired the macos-13 hosted runner in
> late 2025, so v1.0 ships an Apple-Silicon-only `.dmg`.  Intel Mac
> users have a first-class build-from-source path — see
> [Build from source](#build-from-source) below; the same `pnpm
> tauri build` produces a working `.dmg` on an Intel host.

1. Download `Sightline_<version>_aarch64.dmg` (Apple Silicon: M1, M2, M3, M4).
2. Open the `.dmg` and drag **Sightline.app** to `/Applications`.
3. **First launch** — choose one of:
   - **Right-click → Open** on the app icon, then click **Open** in the warning dialog. macOS remembers the choice; subsequent launches don't prompt.
   - Or run this from Terminal once after installing:
     ```bash
     xattr -d com.apple.quarantine /Applications/Sightline.app
     ```
     This removes the quarantine flag macOS adds to anything downloaded from the internet.

If you see "Sightline is damaged and can't be opened" — that's macOS Gatekeeper on a recent update. The `xattr` command above resolves it.

> **macOS 15.3 (Sequoia) and later:** the right-click → Open path is sometimes hidden. Use **System Settings → Privacy & Security → Open Anyway** after a blocked launch.

---

## Windows

1. Download `Sightline_<version>_x64-setup.exe` (recommended — NSIS installer; portable) or `Sightline_<version>_x64_en-US.msi` (MSI; admin-friendly group-policy install).
2. Run the installer. **SmartScreen** will warn that "Windows protected your PC".
3. Click **More info → Run anyway** to proceed.

The installer adds Sightline to **Programs**. Uninstall via Settings → Apps as usual.

---

## Linux

### AppImage (any modern distro)

```bash
chmod +x sightline_*.AppImage
./sightline_*.AppImage
```

The first launch may take a moment to extract the embedded contents. Subsequent launches are instant.

> If the AppImage refuses to run with `dlopen() error`, your distro is missing one of:
> - libwebkit2gtk-4.1
> - libayatana-appindicator3
> - librsvg2
>
> Install via your package manager (`apt install libwebkit2gtk-4.1-0 libayatana-appindicator3-1 librsvg2-2` on Debian/Ubuntu).

### Debian / Ubuntu (.deb)

```bash
sudo dpkg -i sightline_*.deb
sudo apt-get install -f       # resolves any missing system deps
```

The `.deb` integrates with your desktop's app menu and respects the system's tray daemon (StatusNotifierItem on KDE / GNOME extensions / etc.).

---

## Verifying the binary

Each release page lists asset SHAs in the GitHub Actions workflow log under the **build** job. The bytes you download match those SHAs because the artifact uploader signs them in transit. If you'd rather verify locally, build from source:

```bash
git clone https://github.com/kinsar-rasnik/sightline
cd sightline
git checkout v<version>           # tag matches the release
pnpm install --frozen-lockfile
./scripts/bundle-sidecars.sh      # or .ps1 on Windows
pnpm tauri build
```

Compare your local bundle's contents against the release asset. Bit-identical builds are not guaranteed (Tauri's bundler isn't fully deterministic) but the sidecars + Rust outputs match by SHA-256.

---

## Build from source

This is the **recommended path** for security-conscious users. Sightline is MIT-licensed and the build pipeline is purely open-source toolchains.

### Prerequisites

- **Rust** 1.90+
- **Node.js** 24+
- **pnpm** 10+
- Tauri 2's platform deps:
  - macOS: Xcode Command Line Tools
  - Windows: Microsoft C++ Build Tools, WebView2 Runtime
  - Linux: see https://v2.tauri.app/start/prerequisites/#linux

### Steps

```bash
git clone https://github.com/kinsar-rasnik/sightline
cd sightline
pnpm install --frozen-lockfile
./scripts/bundle-sidecars.sh    # or scripts/bundle-sidecars.ps1 on Windows
pnpm tauri dev                  # development run
pnpm tauri build                # platform installer
```

The `bundle-sidecars` script fetches yt-dlp + ffmpeg, **verifies SHA-256 hashes against `scripts/sidecars.lock`**, and installs them into `src-tauri/binaries/`. A hash mismatch aborts the build with exit code 3 — see [ADR-0013](adr/0013-sidecar-bundling.md).

### CI parity

The same build runs in `.github/workflows/release.yml` for the published binaries. If `pnpm tauri build` succeeds locally, the release pipeline will succeed for the same commit.

---

## Troubleshooting

| Symptom | Likely cause | Fix |
| --- | --- | --- |
| macOS: "Sightline.app is damaged" | Gatekeeper quarantine | `xattr -d com.apple.quarantine /Applications/Sightline.app` |
| macOS: "Cannot be opened because Apple cannot check it for malicious software" | Standard Gatekeeper warning for unsigned apps | Right-click → Open, or System Settings → Privacy & Security → Open Anyway |
| Windows: SmartScreen blocks the installer | Standard SmartScreen warning for unsigned apps | More info → Run anyway |
| Linux: AppImage segfaults on launch | Missing webkit2gtk / libappindicator | Install the system deps listed under Linux above |
| Player can't decode files | OS video pipeline missing codecs | macOS / Windows: should "just work". Linux: install `gstreamer1.0-libav` and the `restricted-extras` package for your distro |
| Settings shows "Couldn't reach GitHub" | Update checker can't connect | Disable the updater toggle, or check firewall rules for `api.github.com` |
| Disk fills up after a few weeks of polling | Auto-cleanup not enabled | Settings → Storage → Enable auto-cleanup; preview the plan before confirming |

If none of the above match: open an issue at https://github.com/kinsar-rasnik/sightline/issues with the OS version, app version (Settings → Updates → Currently running …), and the message you saw.
