//! Command handlers. Keep them thin: validate, call into a crate, map errors.

use std::path::PathBuf;

use selah_core::CueId;
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
    /// A disarmed cue does nothing when it is reached, but GO still moves on.
    pub armed: bool,
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
    /// The show's folder, once it is saved.
    pub location: Option<String>,
    /// Why the last save failed, in plain words.
    pub save_problem: Option<String>,
    /// Something to tell the operator about how the show was opened.
    pub notice: Option<String>,
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
                    armed: c.armed,
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
            location: s.location,
            save_problem: s.save_problem,
            notice: s.notice,
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

/// Opens a saved show from its folder. Changes to it are saved automatically.
#[tauri::command]
#[specta::specta]
pub fn open_show(host: State<'_, HostState>, path: String) -> Result<(), AppError> {
    let folder = PathBuf::from(path.trim());
    let loaded =
        selah_persist::load(&folder).map_err(|e| AppError::invalid_input(e.to_string()))?;
    send(
        &host,
        Command::Open {
            show: Box::new(loaded.show),
            folder,
            recovered_from: loaded.recovered_from,
        },
    )
}

/// Saves the show now. The first save also turns on autosave.
#[tauri::command]
#[specta::specta]
pub fn save_show(host: State<'_, HostState>) -> Result<(), AppError> {
    send(&host, Command::SaveShow)
}

/// Arms or disarms a cue.
#[tauri::command]
#[specta::specta]
pub fn set_armed(host: State<'_, HostState>, cue: String, armed: bool) -> Result<(), AppError> {
    let cue: CueId = cue
        .parse()
        .map_err(|_| AppError::invalid_input("That cue can't be found."))?;
    send(&host, Command::SetArmed { cue, armed })
}
