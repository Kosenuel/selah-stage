//! The IPC surface: every command (and, later, event) the UI can use.
//!
//! The TypeScript side of this contract is generated, never hand-written.
//! After changing a command, its arguments, or a type it returns, run
//! `pnpm bindings` and commit `apps/desktop/src/lib/bindings.ts`. The test in
//! `tests/ipc_contract.rs` fails when that file is out of date.

use tauri_specta::{Builder, ErrorHandlingMode, collect_commands};

use crate::{commands, editing};

/// Registers every command. This is the single list to extend.
///
/// When the first event is added, register it here with
/// `.events(collect_events![...])` and call `mount_events` from the Tauri
/// `setup` hook in `lib.rs`.
pub fn builder() -> Builder<tauri::Wry> {
    Builder::<tauri::Wry>::new()
        .commands(collect_commands![
            commands::app_info,
            commands::service_state,
            commands::load_folder,
            commands::go,
            commands::skip,
            commands::panic_stop,
            commands::reset_show,
            commands::open_show,
            commands::save_show,
            editing::edit_cue,
            editing::delete_cues,
            editing::move_cue,
            editing::duplicate_cue,
            editing::add_memo,
            editing::add_audio,
            editing::renumber_cues,
            editing::undo,
            editing::redo,
            editing::set_editing,
            editing::new_show,
            editing::pick_folder,
            editing::pick_audio_files
        ])
        // A failed call can carry our `AppError` or a failure from Tauri itself
        // (an unknown command, a bad argument), and the two look alike in
        // JavaScript. So nothing is thrown and the error is typed `unknown`;
        // `call` in `apps/desktop/src/lib/ipc.ts` narrows it in one place.
        .error_handling(ErrorHandlingMode::ResultUnknown)
        // Typed `mockCommands` helper for front-end tests.
        .mocks()
}
