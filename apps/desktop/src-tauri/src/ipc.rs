//! The IPC surface: every command (and, later, event) the UI can use.
//!
//! The TypeScript side of this contract is generated, never hand-written.
//! After changing a command, its arguments, or a type it returns, run
//! `pnpm bindings` and commit `apps/desktop/src/lib/bindings.ts`.

use tauri_specta::{Builder, ErrorHandlingMode, collect_commands};

use crate::commands;

/// Registers every command. This is the single list to extend.
///
/// When the first event is added, register it here with
/// `.events(collect_events![...])` and call `mount_events` from the Tauri
/// `setup` hook in `lib.rs`.
pub fn builder() -> Builder<tauri::Wry> {
    Builder::<tauri::Wry>::new()
        .commands(collect_commands![commands::app_info])
        // A failed call can carry our `AppError` or a failure from Tauri itself
        // (an unknown command, a bad argument), and the two look alike in
        // JavaScript. So nothing is thrown and the error is typed `unknown`;
        // `call` in `apps/desktop/src/lib/ipc.ts` narrows it in one place.
        .error_handling(ErrorHandlingMode::ResultUnknown)
        // Typed `mockCommands` helper for front-end tests.
        .mocks()
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use specta_typescript::Typescript;

    use super::*;

    const COMMITTED: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../src/lib/bindings.ts");

    /// Fails when the committed bindings no longer match the Rust types.
    #[test]
    fn bindings_are_up_to_date() {
        let fresh_path =
            std::env::temp_dir().join(format!("selah-bindings-{}.ts", std::process::id()));
        export_bindings(&fresh_path).unwrap();
        let fresh = std::fs::read_to_string(&fresh_path).unwrap();
        let _ = std::fs::remove_file(&fresh_path);

        let committed = std::fs::read_to_string(COMMITTED).unwrap_or_default();

        assert!(
            normalise(&fresh) == normalise(&committed),
            "apps/desktop/src/lib/bindings.ts is out of date. Run `pnpm bindings` and commit the result."
        );
    }

    /// Regenerates the committed bindings. Run with `pnpm bindings`.
    #[test]
    #[ignore = "writes into the source tree; run through `pnpm bindings`"]
    fn write_bindings() {
        export_bindings(Path::new(COMMITTED)).unwrap();
    }

    /// Writes the TypeScript bindings for the whole IPC surface to `path`.
    fn export_bindings(path: &Path) -> Result<(), String> {
        // The generated helpers trip our stricter unused-code checks, so the
        // file opts out of type checking. Its exported types still apply to
        // every caller.
        let typescript = Typescript::default().header("// @ts-nocheck");
        builder()
            .export(typescript, path)
            .map_err(|error| error.to_string())
    }

    /// Checkouts on Windows may convert line endings.
    fn normalise(text: &str) -> String {
        text.replace("\r\n", "\n")
    }
}
