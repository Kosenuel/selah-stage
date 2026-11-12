//! The Selah Stage desktop shell.
//!
//! This crate is deliberately thin: it wires the `selah-*` crates to Tauri,
//! registers commands, and manages windows. Logic that can be tested without a
//! window belongs in a crate under `crates/`.

mod commands;
mod error;
mod ipc;

use std::process::ExitCode;

pub use commands::AppInfo;
pub use error::{AppError, AppErrorKind};

/// Starts the app and blocks until it exits.
pub fn run() -> ExitCode {
    init_logging();

    let ipc = ipc::builder();
    let result = tauri::Builder::default()
        .invoke_handler(ipc.invoke_handler())
        .run(tauri::generate_context!());

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            tracing::error!(%error, "Selah Stage could not start");
            ExitCode::FAILURE
        }
    }
}

/// Logs to stderr for now. Release builds on Windows have no console, so a
/// log file in the app data folder is needed before the first field test.
fn init_logging() {
    let subscriber = tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_max_level(tracing::Level::INFO)
        .finish();
    // A second initialisation (for example from a test harness) is harmless,
    // so the "already set" error is ignored on purpose.
    let _ = tracing::subscriber::set_global_default(subscriber);
}
