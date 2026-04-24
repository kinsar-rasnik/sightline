fn main() {
    // Expose the Rust target triple (e.g. `aarch64-apple-darwin`) so runtime
    // code can resolve Tauri `externalBin` sidecars by their canonical
    // `<name>-<triple>[.exe]` on-disk name. See ADR-0013.
    let target = std::env::var("TARGET").unwrap_or_default();
    println!("cargo:rustc-env=TARGET_TRIPLE={target}");
    tauri_build::build();
}
