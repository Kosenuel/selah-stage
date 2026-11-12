//! Command handlers. Keep them thin: validate, call into a crate, map errors.

use std::path::PathBuf;

use selah_core::{ControlKind, FadeCurve, Follow, GroupMode};
use selah_engine::CueState;
use selah_host::{Command, Snapshot, SnapshotCue};
use serde::{Deserialize, Serialize};
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
    pub notes: String,
    /// Seconds to wait after the cue is fired before it starts.
    pub pre_wait: f64,
    /// Seconds after it starts before a "continue" cue starts the next one.
    pub post_wait: f64,
    pub follow: FollowMode,
    /// Present for audio cues.
    pub audio: Option<AudioView>,
    /// Present for fade, stop, pause, go-to and arm cues.
    pub control: Option<ControlView>,
    /// Present for group cues; the cues inside are in `children`.
    pub group: Option<GroupView>,
    pub children: Vec<CueView>,
}

/// The kinds of cue that act on another cue.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum ControlType {
    Fade,
    Stop,
    Pause,
    GoTo,
    Arm,
}

impl From<ControlKind> for ControlType {
    fn from(k: ControlKind) -> Self {
        match k {
            ControlKind::Fade => Self::Fade,
            ControlKind::Stop => Self::Stop,
            ControlKind::Pause => Self::Pause,
            ControlKind::GoTo => Self::GoTo,
            ControlKind::Arm => Self::Arm,
        }
    }
}

impl From<ControlType> for ControlKind {
    fn from(k: ControlType) -> Self {
        match k {
            ControlType::Fade => Self::Fade,
            ControlType::Stop => Self::Stop,
            ControlType::Pause => Self::Pause,
            ControlType::GoTo => Self::GoTo,
            ControlType::Arm => Self::Arm,
        }
    }
}

/// How the cues in a group start.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum GroupType {
    /// All at once.
    Together,
    /// One after another.
    Sequence,
    /// One at random each time.
    Shuffle,
}

impl From<GroupMode> for GroupType {
    fn from(m: GroupMode) -> Self {
        match m {
            GroupMode::Together => Self::Together,
            GroupMode::Sequence => Self::Sequence,
            GroupMode::Shuffle => Self::Shuffle,
        }
    }
}

impl From<GroupType> for GroupMode {
    fn from(m: GroupType) -> Self {
        match m {
            GroupType::Together => Self::Together,
            GroupType::Sequence => Self::Sequence,
            GroupType::Shuffle => Self::Shuffle,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct GroupView {
    pub mode: GroupType,
    /// Starts again when it finishes.
    pub looped: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ControlView {
    pub kind: ControlType,
    /// The cue it acts on.
    pub target: String,
    /// `null` when that cue has been deleted, so this one does nothing.
    pub target_name: Option<String>,
    /// Fade cues: the level to fade to (decibels) and how long it takes.
    pub to: f64,
    pub duration: f64,
    pub curve: CurveKind,
    /// Fade cues: stop the target once the fade is done.
    pub stop_when_done: bool,
    /// Arm cues: arms (true) or disarms (false) the target.
    pub arms: bool,
}

/// What happens to the next cue.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum FollowMode {
    /// Wait for the operator's next GO.
    None,
    /// Start the next cue when this one starts.
    Continue,
    /// Start the next cue when this one finishes.
    Follow,
}

impl From<Follow> for FollowMode {
    fn from(f: Follow) -> Self {
        match f {
            Follow::None => Self::None,
            Follow::Continue => Self::Continue,
            Follow::Follow => Self::Follow,
        }
    }
}

impl From<FollowMode> for Follow {
    fn from(f: FollowMode) -> Self {
        match f {
            FollowMode::None => Self::None,
            FollowMode::Continue => Self::Continue,
            FollowMode::Follow => Self::Follow,
        }
    }
}

/// The shape of a fade.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum CurveKind {
    Linear,
    EqualPower,
    SCurve,
}

impl From<FadeCurve> for CurveKind {
    fn from(c: FadeCurve) -> Self {
        match c {
            FadeCurve::Linear => Self::Linear,
            FadeCurve::EqualPower => Self::EqualPower,
            FadeCurve::SCurve => Self::SCurve,
        }
    }
}

impl From<CurveKind> for FadeCurve {
    fn from(c: CurveKind) -> Self {
        match c {
            CurveKind::Linear => Self::Linear,
            CurveKind::EqualPower => Self::EqualPower,
            CurveKind::SCurve => Self::SCurve,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AudioView {
    /// Path relative to the show folder.
    pub file: String,
    /// Decibels.
    pub level: f64,
    /// Where in the file playback starts and ends, in seconds.
    pub start: f64,
    pub end: Option<f64>,
    /// `null` or 1 plays once, `n` plays `n` times, 0 loops forever.
    pub loops: Option<u32>,
    pub fade_in: f64,
    pub fade_out: f64,
    pub curve: CurveKind,
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
    /// The folder the show's media lives in, once a show is open.
    pub media_root: Option<String>,
    /// False in show mode, when changes are locked out.
    pub editing: bool,
    pub can_undo: bool,
    pub can_redo: bool,
}

impl From<Snapshot> for ServiceView {
    fn from(s: Snapshot) -> Self {
        Self {
            title: s.title,
            cues: s.cues.into_iter().map(cue_view).collect(),
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
            media_root: s.media_root,
            editing: s.editing,
            can_undo: s.can_undo,
            can_redo: s.can_redo,
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

fn cue_view(c: SnapshotCue) -> CueView {
    CueView {
        id: c.id,
        number: c.number,
        name: c.name,
        status: c.state.into(),
        armed: c.armed,
        notes: c.notes,
        pre_wait: c.pre_wait,
        post_wait: c.post_wait,
        follow: c.follow.into(),
        audio: c.audio.map(|a| AudioView {
            file: a.file,
            level: a.level,
            start: a.start,
            end: a.end,
            loops: a.loops,
            fade_in: a.fade_in,
            fade_out: a.fade_out,
            curve: a.curve.into(),
        }),
        control: c.control.map(|k| ControlView {
            kind: k.kind.into(),
            target: k.target,
            target_name: k.target_name,
            to: k.to,
            duration: k.duration,
            curve: k.curve.into(),
            stop_when_done: k.stop_when_done,
            arms: k.arms,
        }),
        group: c.group.map(|g| GroupView {
            mode: g.mode.into(),
            looped: g.looped,
        }),
        children: c.children.into_iter().map(cue_view).collect(),
    }
}
