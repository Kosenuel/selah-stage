//! Contract tests for the IPC surface: what the UI can call and the shapes it
//! receives.
//!
//! Tests for this crate live here rather than beside the code; `Cargo.toml`
//! explains why.

use std::path::Path;

use selah_desktop::AppError;
use specta_typescript::Typescript;

const COMMITTED_BINDINGS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../src/lib/bindings.ts");

/// Fails when the committed bindings no longer match the Rust types.
#[test]
fn bindings_are_up_to_date() {
    let fresh_path = std::env::temp_dir().join(format!("selah-bindings-{}.ts", std::process::id()));
    export_bindings(&fresh_path).unwrap();
    let fresh = std::fs::read_to_string(&fresh_path).unwrap();
    let _ = std::fs::remove_file(&fresh_path);

    let committed = std::fs::read_to_string(COMMITTED_BINDINGS).unwrap_or_default();

    assert!(
        normalise(&fresh) == normalise(&committed),
        "apps/desktop/src/lib/bindings.ts is out of date. Run `pnpm bindings` and commit the result."
    );
}

/// Regenerates the committed bindings. Run with `pnpm bindings`.
#[test]
#[ignore = "writes into the source tree; run through `pnpm bindings`"]
fn write_bindings() {
    export_bindings(Path::new(COMMITTED_BINDINGS)).unwrap();
}

/// The UI depends on this exact JSON shape; changing it is an IPC change.
#[test]
fn app_error_serialises_to_the_shape_the_ui_expects() {
    let json = serde_json::to_value(AppError::internal("Something went wrong.")).unwrap();

    assert_eq!(
        json,
        serde_json::json!({ "kind": "internal", "message": "Something went wrong." })
    );
}

/// Writes the TypeScript bindings for the whole IPC surface to `path`.
fn export_bindings(path: &Path) -> Result<(), String> {
    // The generated helpers trip our stricter unused-code checks, so the file
    // opts out of type checking. Its exported types still apply to every
    // caller.
    let typescript = Typescript::default().header("// @ts-nocheck");
    selah_desktop::ipc::builder()
        .export(typescript, path)
        .map_err(|error| error.to_string())
}

/// Checkouts on Windows may convert line endings.
fn normalise(text: &str) -> String {
    text.replace("\r\n", "\n")
}
