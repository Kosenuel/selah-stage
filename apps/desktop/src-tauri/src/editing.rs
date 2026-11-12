//! Commands that change the show, and the pickers that choose what to open.
//!
//! Each command is a thin wrapper: parse what the UI sent, ask the host to
//! apply an [`Edit`], and turn a refusal into a message for the operator.

use std::path::{Path, PathBuf};

use selah_core::{CueChange, CueId, Edit, NewAudio};
use selah_host::Command;
use serde::Deserialize;
use specta::Type;
use tauri::{AppHandle, State};
use tauri_plugin_dialog::DialogExt;

use crate::commands::{ControlType, CurveKind, FollowMode, GroupType};
use crate::error::AppError;
use crate::host::HostState;

/// One property of a cue, and its new value.
#[derive(Debug, Clone, PartialEq, Deserialize, Type)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum CueChangeInput {
    Name {
        value: String,
    },
    Number {
        value: String,
    },
    Notes {
        value: String,
    },
    Armed {
        value: bool,
    },
    PreWait {
        value: f64,
    },
    PostWait {
        value: f64,
    },
    Follow {
        value: FollowMode,
    },
    /// Decibels. Audio cues only.
    Level {
        value: f64,
    },
    Start {
        value: f64,
    },
    End {
        value: Option<f64>,
    },
    Loops {
        value: Option<u32>,
    },
    FadeIn {
        value: f64,
    },
    FadeOut {
        value: f64,
    },
    Curve {
        value: CurveKind,
    },
    /// The cue a control cue acts on.
    Target {
        value: String,
    },
    /// Fade cues: the level to fade to, in decibels.
    FadeTo {
        value: f64,
    },
    FadeDuration {
        value: f64,
    },
    StopWhenDone {
        value: bool,
    },
    /// Arm cues: true arms the target, false disarms it.
    ArmAction {
        value: bool,
    },
    GroupMode {
        value: GroupType,
    },
    Looped {
        value: bool,
    },
}

impl TryFrom<CueChangeInput> for CueChange {
    type Error = AppError;

    fn try_from(input: CueChangeInput) -> Result<Self, AppError> {
        Ok(match input {
            CueChangeInput::Target { value } => Self::Target(parse_cue(&value)?),
            CueChangeInput::Name { value } => Self::Name(value),
            CueChangeInput::Number { value } => Self::Number(value),
            CueChangeInput::Notes { value } => Self::Notes(value),
            CueChangeInput::Armed { value } => Self::Armed(value),
            CueChangeInput::PreWait { value } => Self::PreWait(value),
            CueChangeInput::PostWait { value } => Self::PostWait(value),
            CueChangeInput::Follow { value } => Self::Follow(value.into()),
            CueChangeInput::Level { value } => Self::Level(value),
            CueChangeInput::Start { value } => Self::Start(value),
            CueChangeInput::End { value } => Self::End(value),
            CueChangeInput::Loops { value } => Self::Loops(value),
            CueChangeInput::FadeIn { value } => Self::FadeIn(value),
            CueChangeInput::FadeOut { value } => Self::FadeOut(value),
            CueChangeInput::Curve { value } => Self::Curve(value.into()),
            CueChangeInput::FadeTo { value } => Self::FadeTo(value),
            CueChangeInput::FadeDuration { value } => Self::FadeDuration(value),
            CueChangeInput::StopWhenDone { value } => Self::StopWhenDone(value),
            CueChangeInput::ArmAction { value } => Self::ArmAction(value),
            CueChangeInput::GroupMode { value } => Self::GroupMode(value.into()),
            CueChangeInput::Looped { value } => Self::Looped(value),
        })
    }
}

fn parse_cue(id: &str) -> Result<CueId, AppError> {
    id.parse()
        .map_err(|_| AppError::invalid_input("That cue can't be found."))
}

fn parse_before(id: Option<&str>) -> Result<Option<CueId>, AppError> {
    id.map(parse_cue).transpose()
}

fn apply(host: &HostState, edit: Edit) -> Result<selah_core::Outcome, AppError> {
    host.get()?
        .edit(edit)
        .map_err(|e| AppError::invalid_input(e.to_string()))
}

fn ids(outcome: &selah_core::Outcome) -> Vec<String> {
    outcome
        .created
        .iter()
        .map(|id| id.as_uuid().to_string())
        .collect()
}

#[tauri::command]
#[specta::specta]
pub fn edit_cue(
    host: State<'_, HostState>,
    cue: String,
    change: CueChangeInput,
) -> Result<(), AppError> {
    apply(
        &host,
        Edit::Change {
            id: parse_cue(&cue)?,
            change: change.try_into()?,
        },
    )
    .map(|_| ())
}

#[tauri::command]
#[specta::specta]
pub fn delete_cues(host: State<'_, HostState>, cues: Vec<String>) -> Result<(), AppError> {
    let ids = cues
        .iter()
        .map(|c| parse_cue(c))
        .collect::<Result<Vec<_>, _>>()?;
    apply(&host, Edit::Delete { ids }).map(|_| ())
}

/// Moves a cue to just before `before`, or to the end.
#[tauri::command]
#[specta::specta]
pub fn move_cue(
    host: State<'_, HostState>,
    cue: String,
    before: Option<String>,
) -> Result<(), AppError> {
    apply(
        &host,
        Edit::Move {
            id: parse_cue(&cue)?,
            before: parse_before(before.as_deref())?,
        },
    )
    .map(|_| ())
}

/// Returns the id of the copy.
#[tauri::command]
#[specta::specta]
pub fn duplicate_cue(host: State<'_, HostState>, cue: String) -> Result<Vec<String>, AppError> {
    apply(
        &host,
        Edit::Duplicate {
            id: parse_cue(&cue)?,
        },
    )
    .map(|o| ids(&o))
}

/// Adds a note-only cue. Returns its id.
#[tauri::command]
#[specta::specta]
pub fn add_memo(
    host: State<'_, HostState>,
    before: Option<String>,
) -> Result<Vec<String>, AppError> {
    apply(
        &host,
        Edit::AddMemo {
            name: "Note".into(),
            before: parse_before(before.as_deref())?,
        },
    )
    .map(|o| ids(&o))
}

/// Adds the chosen audio files as cues, copying any that are outside the show
/// folder into it. Returns the new cue ids.
#[tauri::command]
#[specta::specta]
pub async fn add_audio(
    host: State<'_, HostState>,
    paths: Vec<String>,
    before: Option<String>,
) -> Result<Vec<String>, AppError> {
    let handle = host.get()?;
    let root = handle
        .snapshot()
        .media_root
        .ok_or_else(|| AppError::invalid_input("Open or create a show first."))?;
    let mut files = Vec::new();
    for path in &paths {
        let source = Path::new(path);
        let file = selah_persist::import_media(Path::new(&root), source)
            .map_err(|e| AppError::invalid_input(e.to_string()))?;
        let name = source
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Audio")
            .to_owned();
        files.push(NewAudio { file, name });
    }
    apply(
        &host,
        Edit::AddAudio {
            files,
            before: parse_before(before.as_deref())?,
        },
    )
    .map(|o| ids(&o))
}

#[tauri::command]
#[specta::specta]
pub fn renumber_cues(host: State<'_, HostState>) -> Result<(), AppError> {
    apply(&host, Edit::Renumber).map(|_| ())
}

#[tauri::command]
#[specta::specta]
pub fn undo(host: State<'_, HostState>) -> Result<(), AppError> {
    host.get()?
        .undo()
        .map_err(|e| AppError::invalid_input(e.to_string()))
}

#[tauri::command]
#[specta::specta]
pub fn redo(host: State<'_, HostState>) -> Result<(), AppError> {
    host.get()?
        .redo()
        .map_err(|e| AppError::invalid_input(e.to_string()))
}

/// Edit mode allows changes; show mode locks them out.
#[tauri::command]
#[specta::specta]
pub fn set_editing(host: State<'_, HostState>, editing: bool) -> Result<(), AppError> {
    if host.get()?.send(Command::SetEditing(editing)) {
        Ok(())
    } else {
        Err(AppError::internal(
            "The audio engine stopped. Restart Selah Stage.",
        ))
    }
}

/// Creates an empty show in a folder and opens it.
#[tauri::command]
#[specta::specta]
pub fn new_show(host: State<'_, HostState>, path: String) -> Result<(), AppError> {
    let folder = PathBuf::from(path.trim());
    let title = folder
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("New show")
        .to_owned();
    let show = selah_persist::create(&folder, &title)
        .map_err(|e| AppError::invalid_input(e.to_string()))?;
    if host.get()?.send(Command::Open {
        show: Box::new(show),
        folder,
        recovered_from: None,
    }) {
        Ok(())
    } else {
        Err(AppError::internal(
            "The audio engine stopped. Restart Selah Stage.",
        ))
    }
}

/// Asks the operator for a folder. `None` means they cancelled.
#[tauri::command]
#[specta::specta]
pub async fn pick_folder(app: AppHandle, title: String) -> Result<Option<String>, AppError> {
    let chosen = app.dialog().file().set_title(title).blocking_pick_folder();
    path_of(chosen)
}

/// Asks the operator for audio files. An empty list means they cancelled.
#[tauri::command]
#[specta::specta]
pub async fn pick_audio_files(app: AppHandle) -> Result<Vec<String>, AppError> {
    let chosen = app
        .dialog()
        .file()
        .set_title("Add audio")
        .add_filter(
            "Audio",
            &["wav", "mp3", "flac", "m4a", "aac", "ogg", "aif", "aiff"],
        )
        .blocking_pick_files();
    chosen
        .unwrap_or_default()
        .into_iter()
        .map(|f| path_of(Some(f)).map(Option::unwrap_or_default))
        .collect()
}

fn path_of(chosen: Option<tauri_plugin_dialog::FilePath>) -> Result<Option<String>, AppError> {
    chosen
        .map(|file| {
            file.into_path()
                .map(|p| p.display().to_string())
                .map_err(|_| AppError::invalid_input("That location can't be used."))
        })
        .transpose()
}

/// Adds a fade, stop, pause, go-to or arm cue that acts on `target`.
#[tauri::command]
#[specta::specta]
pub fn add_control(
    host: State<'_, HostState>,
    kind: ControlType,
    target: String,
    before: Option<String>,
) -> Result<Vec<String>, AppError> {
    apply(
        &host,
        Edit::AddControl {
            kind: kind.into(),
            target: parse_cue(&target)?,
            before: parse_before(before.as_deref())?,
        },
    )
    .map(|o| ids(&o))
}

/// Puts main-list cues into a new group. Returns the group's id.
#[tauri::command]
#[specta::specta]
pub fn group_cues(host: State<'_, HostState>, cues: Vec<String>) -> Result<Vec<String>, AppError> {
    let ids_in = cues
        .iter()
        .map(|c| parse_cue(c))
        .collect::<Result<Vec<_>, _>>()?;
    apply(&host, Edit::Group { ids: ids_in }).map(|o| ids(&o))
}

/// Replaces a group with the cues inside it.
#[tauri::command]
#[specta::specta]
pub fn ungroup(host: State<'_, HostState>, cue: String) -> Result<(), AppError> {
    apply(
        &host,
        Edit::Ungroup {
            id: parse_cue(&cue)?,
        },
    )
    .map(|_| ())
}
