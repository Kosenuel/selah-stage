//! Command handlers. Keep them thin: validate, call into a crate, map errors.

use serde::Serialize;
use specta::Type;

use crate::error::AppError;

/// Facts about the running app, shown on the About screen and attached to
/// diagnostics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub name: String,
    pub version: String,
    /// Operating system family, for example `windows` or `linux`.
    pub os: String,
    /// CPU architecture, for example `x86_64`.
    pub arch: String,
    /// Version of the system webview that renders the UI, if it can be read.
    pub webview_version: Option<String>,
    /// True for development builds, which are slower and log more.
    pub debug_build: bool,
}

#[tauri::command]
#[specta::specta]
pub fn app_info(app: tauri::AppHandle) -> Result<AppInfo, AppError> {
    let package = app.package_info();
    Ok(AppInfo {
        name: package.name.clone(),
        version: package.version.to_string(),
        os: std::env::consts::OS.to_owned(),
        arch: std::env::consts::ARCH.to_owned(),
        webview_version: tauri::webview_version().ok(),
        debug_build: cfg!(debug_assertions),
    })
}
