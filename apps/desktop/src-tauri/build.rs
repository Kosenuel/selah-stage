fn main() {
    tauri_build::build();
    attach_manifest_to_test_programs();
}

/// Gives the programs built from `tests/` a Windows manifest.
///
/// Tauri uses version 6 of the Windows common controls, which a program only
/// receives if its manifest asks for it. `tauri_build` embeds such a manifest
/// in the app itself but not in test programs, and without one Windows will
/// not start them (`STATUS_ENTRYPOINT_NOT_FOUND`).
fn attach_manifest_to_test_programs() {
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let target_env = std::env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
    if target_os != "windows" || target_env != "msvc" {
        return;
    }
    let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") else {
        return;
    };
    let manifest = std::path::Path::new(&manifest_dir).join("tests/windows.manifest");
    println!("cargo:rerun-if-changed={}", manifest.display());
    println!("cargo:rustc-link-arg-tests=/MANIFEST:EMBED");
    println!(
        "cargo:rustc-link-arg-tests=/MANIFESTINPUT:{}",
        manifest.display()
    );
}
