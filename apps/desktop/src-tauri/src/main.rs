//! Binary entry point. All logic lives in the library so it can be tested.

// Release builds on Windows must not open a console window behind the app.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::process::ExitCode;

fn main() -> ExitCode {
    selah_desktop::run()
}
