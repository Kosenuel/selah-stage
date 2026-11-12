//! Command handlers. Keep them thin: validate, call into a crate, map errors.

use std::path::PathBuf;

use selah_engine::CueState;
use selah_host::{Command, Snapshot};
use serde::Serialize;
use specta::Type;
use tauri::State;

use crate::error::AppError;
use crate::host::HostState;

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

/// Where a cue is in its life, as the UI names it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum CueStatus {
    Idle,
    Waiting,
    Running,
    Paused,
    Done,
    Stopped,
}

impl From<CueState> for CueStatus {
    fn from(state: CueState) -> Self {
        match state {
            CueState::Idle => Self::Idle,
            CueState::PreWait => Self::Waiting,
            CueState::Running => Self::Running,
            CueState::Paused => Self::Paused,
            CueState::Done => Self::Done,
            CueState::Stopped => Self::Stopped,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CueView {
    pub id: String,
    pub number: String,
    pub name: String,
    pub status: CueStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct WarningView {
    pub cue: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DeviceView {
    pub name: String,
    pub sample_rate: u32,
    pub channels: u32,
    /// True when sound really goes to a device.
    pub audible: bool,
    /// Plain words about why there is no sound, if there is a problem.
    pub problem: Option<String>,
}

/// Everything the control window shows about the running service.
#[derive(Debug, Clone, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ServiceView {
    pub title: String,
    pub cues: Vec<CueView>,
    /// The cue that fires on the next GO.
    pub playhead: Option<String>,
    pub warnings: Vec<WarningView>,
    /// Peak level per output channel, 0 to 1, since the last read.
    pub peaks: Vec<f32>,
    pub underruns: u32,
    pub device: DeviceView,
}

impl From<Snapshot> for ServiceView {
    fn from(s: Snapshot) -> Self {
        Self {
            title: s.title,
            cues: s
                .cues
                .into_iter()
                .map(|c| CueView {
                    id: c.id,
                    number: c.number,
                    name: c.name,
                    status: c.state.into(),
                })
                .collect(),
            playhead: s.playhead,
            warnings: s
                .warnings
                .into_iter()
                .map(|w| WarningView {
                    cue: w.cue,
                    message: w.message,
                })
                .collect(),
            peaks: s.peaks,
            underruns: s.underruns,
            device: DeviceView {
                name: s.device.name,
                sample_rate: s.device.sample_rate,
                channels: u32::try_from(s.device.channels).unwrap_or(u32::MAX),
                audible: s.device.audible && !s.device.lost,
                problem: s.device.error,
            },
        }
    }
}

fn send(host: &HostState, command: Command) -> Result<(), AppError> {
    if host.get()?.send(command) {
        Ok(())
    } else {
        Err(AppError::internal(
            "The audio engine stopped. Restart Selah Stage.",
        ))
    }
}

/// The current state of the service, for the UI to draw. Cheap; poll it.
#[tauri::command]
#[specta::specta]
pub fn service_state(host: State<'_, HostState>) -> Result<ServiceView, AppError> {
    Ok(host.get()?.snapshot().into())
}

/// Loads every audio file in a folder as a list of cues. A quick way to try
/// the engine until shows can be built and saved.
#[tauri::command]
#[specta::specta]
pub fn load_folder(host: State<'_, HostState>, path: String) -> Result<(), AppError> {
    let (show, media_root) = selah_host::show_from_folder(&PathBuf::from(path.trim()))
        .map_err(|e| AppError::invalid_input(e.to_string()))?;
    send(
        &host,
        Command::Load {
            show: Box::new(show),
            media_root,
        },
    )
}

#[tauri::command]
#[specta::specta]
pub fn go(host: State<'_, HostState>) -> Result<(), AppError> {
    send(&host, Command::Go)
}

#[tauri::command]
#[specta::specta]
pub fn skip(host: State<'_, HostState>) -> Result<(), AppError> {
    send(&host, Command::Skip)
}

/// Fades everything out; pressed again, stops at once.
#[tauri::command]
#[specta::specta]
pub fn panic_stop(host: State<'_, HostState>) -> Result<(), AppError> {
    send(&host, Command::Panic)
}

#[tauri::command]
#[specta::specta]
pub fn reset_show(host: State<'_, HostState>) -> Result<(), AppError> {
    send(&host, Command::Reset)
}
