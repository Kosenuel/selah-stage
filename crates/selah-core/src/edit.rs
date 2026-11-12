//! Editing a show, with undo and redo.
//!
//! Every change to a show goes through [`Editor::apply`]. An edit either
//! succeeds completely or leaves the show untouched. Undo restores earlier
//! snapshots rather than computing inverses, which keeps it correct for every
//! edit, including ones added later. Shows are small, so this is cheap.

use crate::cue::{Cue, CueKind, Decibels, FadeCurve, Follow, GroupMode, Seconds};
use crate::id::CueId;
use crate::show::{Item, ItemKind, Show};

/// A change to one property of one cue.
#[derive(Debug, Clone, PartialEq)]
pub enum CueChange {
    Name(String),
    Number(String),
    Notes(String),
    Armed(bool),
    PreWait(Seconds),
    PostWait(Seconds),
    Follow(Follow),
    // The rest apply to audio cues only.
    Level(Decibels),
    Start(Seconds),
    End(Option<Seconds>),
    Loops(Option<u32>),
    FadeIn(Seconds),
    FadeOut(Seconds),
    Curve(FadeCurve),
    // Control cues (fade, stop, pause, go to, arm).
    Target(CueId),
    FadeTo(Decibels),
    FadeDuration(Seconds),
    StopWhenDone(bool),
    /// For an arm cue: whether it arms (true) or disarms (false) its target.
    ArmAction(bool),
    // Group cues.
    GroupMode(GroupMode),
    Looped(bool),
}

impl CueChange {
    /// Changes of the same kind to the same cue, made in quick succession (a
    /// slider being dragged, text being typed), share one undo step.
    fn mergeable(&self) -> bool {
        !matches!(
            self,
            Self::Armed(_)
                | Self::Follow(_)
                | Self::Curve(_)
                | Self::Loops(_)
                | Self::Target(_)
                | Self::StopWhenDone(_)
                | Self::ArmAction(_)
                | Self::GroupMode(_)
                | Self::Looped(_)
        )
    }

    fn tag(&self) -> u8 {
        match self {
            Self::Name(_) => 0,
            Self::Number(_) => 1,
            Self::Notes(_) => 2,
            Self::Armed(_) => 3,
            Self::PreWait(_) => 4,
            Self::PostWait(_) => 5,
            Self::Follow(_) => 6,
            Self::Level(_) => 7,
            Self::Start(_) => 8,
            Self::End(_) => 9,
            Self::Loops(_) => 10,
            Self::FadeIn(_) => 11,
            Self::FadeOut(_) => 12,
            Self::Curve(_) => 13,
            Self::Target(_) => 14,
            Self::FadeTo(_) => 15,
            Self::FadeDuration(_) => 16,
            Self::StopWhenDone(_) => 17,
            Self::ArmAction(_) => 18,
            Self::GroupMode(_) => 19,
            Self::Looped(_) => 20,
        }
    }
}

/// A file to add as an audio cue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewAudio {
    /// Path relative to the show folder.
    pub file: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Edit {
    /// Adds audio cues before `before`, or at the end.
    AddAudio {
        files: Vec<NewAudio>,
        before: Option<CueId>,
    },
    AddMemo {
        name: String,
        before: Option<CueId>,
    },
    Delete {
        ids: Vec<CueId>,
    },
    /// Moves a cue to just before `before`, or to the end.
    Move {
        id: CueId,
        before: Option<CueId>,
    },
    /// Copies a cue, with new ids, to just after the original.
    Duplicate {
        id: CueId,
    },
    Change {
        id: CueId,
        change: CueChange,
    },
    /// Numbers every top-level cue 1, 2, 3, and so on.
    Renumber,
    /// Adds a control cue that acts on `target`.
    AddControl {
        kind: ControlKind,
        target: CueId,
        before: Option<CueId>,
    },
    /// Puts main-list cues into a new group, where the first of them was.
    Group {
        ids: Vec<CueId>,
    },
    /// Replaces a group with the cues inside it.
    Ungroup {
        id: CueId,
    },
}

/// The kinds of cue that act on another cue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlKind {
    Fade,
    Stop,
    Pause,
    /// Moves the playhead without firing anything.
    GoTo,
    Arm,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum EditError {
    #[error("That cue can't be found. It may have been deleted.")]
    NoSuchCue,
    #[error("That can only be done to a cue in the main list, not one inside a group.")]
    NotTopLevel,
    #[error("That setting doesn't apply to this kind of cue.")]
    WrongKind,
    #[error("{0}")]
    InvalidValue(&'static str),
}

/// What an edit created, so the caller can select it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Outcome {
    pub created: Vec<CueId>,
}

/// The longest a wait, fade, or trim may be: ten hours.
const MAX_SECONDS: f64 = 36_000.0;
const MIN_LEVEL_DB: f64 = -120.0;
const MAX_LEVEL_DB: f64 = 24.0;
/// Changes closer together than this, to the same setting, are one undo step.
const MERGE_WINDOW: Seconds = 1.0;
const HISTORY_LIMIT: usize = 100;

struct Entry {
    before: Show,
    merge_key: Option<(CueId, u8)>,
    at: Seconds,
}

#[derive(Default)]
pub struct Editor {
    undo: Vec<Entry>,
    redo: Vec<Show>,
}

impl Editor {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    /// Forgets all history, for when a different show is loaded.
    pub fn clear(&mut self) {
        self.undo.clear();
        self.redo.clear();
    }

    /// Applies `edit` to `show`. `at` is the time in seconds, used only to
    /// decide whether quick successive changes merge.
    pub fn apply(
        &mut self,
        show: &mut Show,
        edit: Edit,
        at: Seconds,
    ) -> Result<Outcome, EditError> {
        let merge_key = match &edit {
            Edit::Change { id, change } if change.mergeable() => Some((*id, change.tag())),
            _ => None,
        };
        let mut next = show.clone();
        let outcome = perform(&mut next, edit)?;
        if next == *show {
            return Ok(outcome); // Nothing changed, so nothing to undo.
        }

        let merges = merge_key.is_some()
            && self
                .undo
                .last()
                .is_some_and(|last| last.merge_key == merge_key && at - last.at < MERGE_WINDOW);
        if merges {
            if let Some(last) = self.undo.last_mut() {
                last.at = at;
            }
        } else {
            self.undo.push(Entry {
                before: show.clone(),
                merge_key,
                at,
            });
            if self.undo.len() > HISTORY_LIMIT {
                self.undo.remove(0);
            }
        }
        self.redo.clear();
        *show = next;
        Ok(outcome)
    }

    pub fn undo(&mut self, show: &mut Show) -> bool {
        let Some(entry) = self.undo.pop() else {
            return false;
        };
        self.redo.push(std::mem::replace(show, entry.before));
        true
    }

    pub fn redo(&mut self, show: &mut Show) -> bool {
        let Some(after) = self.redo.pop() else {
            return false;
        };
        self.undo.push(Entry {
            before: std::mem::replace(show, after),
            merge_key: None,
            at: f64::NEG_INFINITY,
        });
        true
    }
}

fn perform(show: &mut Show, edit: Edit) -> Result<Outcome, EditError> {
    match edit {
        Edit::AddAudio { files, before } => {
            let mut slot = slot_before(show, before)?;
            let mut created = Vec::new();
            for file in files {
                let mut cue = Cue::new(
                    file.name,
                    CueKind::Audio {
                        file: file.file,
                        level: 0.0,
                        start: 0.0,
                        end: None,
                        loops: None,
                        fade_in: 0.0,
                        fade_out: 0.0,
                        curve: FadeCurve::EqualPower,
                        patches: Vec::new(),
                    },
                );
                cue.number = next_number(show);
                created.push(cue.id);
                insert_at(show, &mut slot, cue)?;
            }
            Ok(Outcome { created })
        }
        Edit::AddMemo { name, before } => {
            let mut slot = slot_before(show, before)?;
            let mut cue = Cue::new(name, CueKind::Memo);
            cue.number = next_number(show);
            let id = cue.id;
            insert_at(show, &mut slot, cue)?;
            Ok(Outcome { created: vec![id] })
        }
        Edit::AddControl {
            kind,
            target,
            before,
        } => {
            let target_name = show
                .all_cues()
                .into_iter()
                .find(|c| c.id == target)
                .map(|c| c.name.clone())
                .ok_or(EditError::NoSuchCue)?;
            if kind == ControlKind::GoTo
                && path_of(show, target).is_none_or(|(_, via)| via.len() > 1)
            {
                return Err(EditError::NotTopLevel);
            }
            let (verb, cue_kind) = match kind {
                ControlKind::Fade => (
                    "Fade",
                    CueKind::Fade {
                        target,
                        to: -60.0,
                        duration: 3.0,
                        curve: FadeCurve::EqualPower,
                        stop_when_done: true,
                    },
                ),
                ControlKind::Stop => ("Stop", CueKind::Stop { target }),
                ControlKind::Pause => ("Pause", CueKind::Pause { target }),
                ControlKind::GoTo => ("Go to", CueKind::GoTo { target }),
                ControlKind::Arm => (
                    "Arm",
                    CueKind::Arm {
                        target,
                        armed: true,
                    },
                ),
            };
            let mut slot = slot_before(show, before)?;
            let mut cue = Cue::new(format!("{verb} {target_name}"), cue_kind);
            cue.number = next_number(show);
            let id = cue.id;
            insert_at(show, &mut slot, cue)?;
            Ok(Outcome { created: vec![id] })
        }
        Edit::Delete { ids } => {
            for id in &ids {
                path_of(show, *id).ok_or(EditError::NoSuchCue)?;
            }
            for id in ids {
                // A group deleted earlier may have taken this cue with it.
                if let Some(path) = path_of(show, id) {
                    remove_at(show, &path);
                }
            }
            Ok(Outcome::default())
        }
        Edit::Move { id, before } => {
            if before == Some(id) {
                return Ok(Outcome::default());
            }
            let path = path_of(show, id).ok_or(EditError::NoSuchCue)?;
            if let Some(target) = before {
                let (_, via) = path_of(show, target).ok_or(EditError::NoSuchCue)?;
                if via.starts_with(&path.1) {
                    return Err(EditError::InvalidValue(
                        "A group can't be put inside itself.",
                    ));
                }
            }
            let cue = remove_at(show, &path);
            let mut slot = slot_before(show, before)?;
            insert_at(show, &mut slot, cue)?;
            Ok(Outcome::default())
        }
        Edit::Duplicate { id } => {
            let path = path_of(show, id).ok_or(EditError::NoSuchCue)?;
            let mut copy = fresh_ids(cue_at(show, &path).ok_or(EditError::NoSuchCue)?);
            copy.number = next_number(show);
            let created = copy.id;
            let mut slot = Slot {
                item: path.0,
                via: path.1[..path.1.len() - 1].to_vec(),
                index: path.1[path.1.len() - 1] + 1,
            };
            insert_at(show, &mut slot, copy)?;
            Ok(Outcome {
                created: vec![created],
            })
        }
        Edit::Change { id, change } => {
            check_target(show, id, &change)?;
            let cue = show.cue_mut(id).ok_or(EditError::NoSuchCue)?;
            change_cue(cue, change)?;
            Ok(Outcome::default())
        }
        Edit::Renumber => {
            let mut n = 0;
            for cue in show.items.iter_mut().flat_map(|i| i.cues.iter_mut()) {
                n += 1;
                cue.number = n.to_string();
            }
            Ok(Outcome::default())
        }
        Edit::Group { ids } => group(show, &ids),
        Edit::Ungroup { id } => {
            let path = path_of(show, id).ok_or(EditError::NoSuchCue)?;
            let cue = cue_at(show, &path).ok_or(EditError::NoSuchCue)?;
            let CueKind::Group { children, .. } = &cue.kind else {
                return Err(EditError::WrongKind);
            };
            let children = children.clone();
            remove_at(show, &path);
            let mut slot = Slot {
                item: path.0,
                via: path.1[..path.1.len() - 1].to_vec(),
                index: path.1[path.1.len() - 1],
            };
            for child in children {
                insert_at(show, &mut slot, child)?;
            }
            Ok(Outcome::default())
        }
    }
}

fn group(show: &mut Show, ids: &[CueId]) -> Result<Outcome, EditError> {
    if ids.is_empty() {
        return Err(EditError::InvalidValue("Choose at least one cue to group."));
    }
    for id in ids {
        match path_of(show, *id) {
            None => return Err(EditError::NoSuchCue),
            Some((_, via)) if via.len() > 1 => return Err(EditError::NotTopLevel),
            Some(_) => {}
        }
    }
    // Take them in list order, so the group keeps the order they had.
    let mut taken: Vec<(usize, usize)> = ids
        .iter()
        .filter_map(|id| path_of(show, *id).map(|(item, via)| (item, via[0])))
        .collect();
    taken.sort_unstable();
    let (first_item, first_index) = taken[0];
    let mut children = Vec::new();
    for (item, index) in taken.iter().rev() {
        children.push(show.items[*item].cues.remove(*index));
    }
    children.reverse();
    let mut group = Cue::new(
        "Group",
        CueKind::Group {
            mode: GroupMode::Together,
            children,
            looped: false,
        },
    );
    group.number = next_number(show);
    let id = group.id;
    // Removing earlier cues from the same item shifts the position, but the
    // first taken cue is always the earliest, so its index is still right.
    show.items[first_item].cues.insert(first_index, group);
    Ok(Outcome { created: vec![id] })
}

/// For a change that names another cue: it must exist, and a go-to may only
/// point at a cue in the main list.
fn check_target(show: &Show, id: CueId, change: &CueChange) -> Result<(), EditError> {
    let CueChange::Target(target) = change else {
        return Ok(());
    };
    let Some(path) = path_of(show, *target) else {
        return Err(EditError::NoSuchCue);
    };
    let is_goto = show
        .all_cues()
        .into_iter()
        .find(|c| c.id == id)
        .is_some_and(|c| matches!(c.kind, CueKind::GoTo { .. }));
    if is_goto && path.1.len() > 1 {
        return Err(EditError::NotTopLevel);
    }
    Ok(())
}

fn change_cue(cue: &mut Cue, change: CueChange) -> Result<(), EditError> {
    match change {
        CueChange::Name(v) => cue.name = v,
        CueChange::Number(v) => cue.number = v,
        CueChange::Notes(v) => cue.notes = v,
        CueChange::Armed(v) => cue.armed = v,
        CueChange::PreWait(v) => cue.pre_wait = seconds(v)?,
        CueChange::PostWait(v) => cue.post_wait = seconds(v)?,
        CueChange::Follow(v) => cue.follow = v,
        CueChange::Level(v) => {
            let CueKind::Audio { level, .. } = &mut cue.kind else {
                return Err(EditError::WrongKind);
            };
            *level = level_db(v)?;
        }
        CueChange::Start(v) => {
            let CueKind::Audio { start, end, .. } = &mut cue.kind else {
                return Err(EditError::WrongKind);
            };
            let v = seconds(v)?;
            if end.is_some_and(|e| v >= e) {
                return Err(EditError::InvalidValue("The start must be before the end."));
            }
            *start = v;
        }
        CueChange::End(v) => {
            let CueKind::Audio { start, end, .. } = &mut cue.kind else {
                return Err(EditError::WrongKind);
            };
            if let Some(v) = v
                && seconds(v)? <= *start
            {
                return Err(EditError::InvalidValue("The end must be after the start."));
            }
            *end = v;
        }
        CueChange::Loops(v) => {
            let CueKind::Audio { loops, .. } = &mut cue.kind else {
                return Err(EditError::WrongKind);
            };
            *loops = v;
        }
        CueChange::FadeIn(v) => {
            let CueKind::Audio { fade_in, .. } = &mut cue.kind else {
                return Err(EditError::WrongKind);
            };
            *fade_in = seconds(v)?;
        }
        CueChange::FadeOut(v) => {
            let CueKind::Audio { fade_out, .. } = &mut cue.kind else {
                return Err(EditError::WrongKind);
            };
            *fade_out = seconds(v)?;
        }
        CueChange::Curve(v) => match &mut cue.kind {
            CueKind::Audio { curve, .. } | CueKind::Fade { curve, .. } => *curve = v,
            _ => return Err(EditError::WrongKind),
        },
        CueChange::Target(v) => match &mut cue.kind {
            CueKind::Fade { target, .. }
            | CueKind::Stop { target }
            | CueKind::Pause { target }
            | CueKind::GoTo { target }
            | CueKind::Arm { target, .. } => *target = v,
            _ => return Err(EditError::WrongKind),
        },
        CueChange::FadeTo(v) => {
            let CueKind::Fade { to, .. } = &mut cue.kind else {
                return Err(EditError::WrongKind);
            };
            *to = level_db(v)?;
        }
        CueChange::FadeDuration(v) => {
            let CueKind::Fade { duration, .. } = &mut cue.kind else {
                return Err(EditError::WrongKind);
            };
            *duration = seconds(v)?;
        }
        CueChange::StopWhenDone(v) => {
            let CueKind::Fade { stop_when_done, .. } = &mut cue.kind else {
                return Err(EditError::WrongKind);
            };
            *stop_when_done = v;
        }
        CueChange::ArmAction(v) => {
            let CueKind::Arm { armed, .. } = &mut cue.kind else {
                return Err(EditError::WrongKind);
            };
            *armed = v;
        }
        CueChange::GroupMode(v) => {
            let CueKind::Group { mode, .. } = &mut cue.kind else {
                return Err(EditError::WrongKind);
            };
            *mode = v;
        }
        CueChange::Looped(v) => {
            let CueKind::Group { looped, .. } = &mut cue.kind else {
                return Err(EditError::WrongKind);
            };
            *looped = v;
        }
    }
    Ok(())
}

fn level_db(v: Decibels) -> Result<Decibels, EditError> {
    if v.is_finite() && (MIN_LEVEL_DB..=MAX_LEVEL_DB).contains(&v) {
        Ok(v)
    } else {
        Err(EditError::InvalidValue(
            "The level must be between -120 and +24 dB.",
        ))
    }
}

fn seconds(v: Seconds) -> Result<Seconds, EditError> {
    if v.is_finite() && (0.0..=MAX_SECONDS).contains(&v) {
        Ok(v)
    } else {
        Err(EditError::InvalidValue(
            "Times must be between 0 and 10 hours.",
        ))
    }
}

/// Where a cue is: its item, then its index at each level of nesting.
type Path = (usize, Vec<usize>);

fn path_of(show: &Show, id: CueId) -> Option<Path> {
    show.items
        .iter()
        .enumerate()
        .find_map(|(i, item)| path_in(&item.cues, id).map(|via| (i, via)))
}

fn path_in(cues: &[Cue], id: CueId) -> Option<Vec<usize>> {
    cues.iter().enumerate().find_map(|(j, cue)| {
        if cue.id == id {
            return Some(vec![j]);
        }
        match &cue.kind {
            CueKind::Group { children, .. } => path_in(children, id).map(|mut rest| {
                rest.insert(0, j);
                rest
            }),
            _ => None,
        }
    })
}

/// The list that holds the cues at `via` levels of nesting below `item`.
fn list_mut<'a>(show: &'a mut Show, item: usize, via: &[usize]) -> Option<&'a mut Vec<Cue>> {
    let mut list = &mut show.items.get_mut(item)?.cues;
    for &index in via {
        match &mut list.get_mut(index)?.kind {
            CueKind::Group { children, .. } => list = children,
            _ => return None,
        }
    }
    Some(list)
}

fn cue_at<'a>(show: &'a Show, path: &Path) -> Option<&'a Cue> {
    let mut cues = &show.items.get(path.0)?.cues;
    let (last, parents) = path.1.split_last()?;
    for &index in parents {
        match &cues.get(index)?.kind {
            CueKind::Group { children, .. } => cues = children,
            _ => return None,
        }
    }
    cues.get(*last)
}

fn remove_at(show: &mut Show, path: &Path) -> Cue {
    let (last, parents) = path.1.split_last().map_or((0, &[][..]), |(l, p)| (*l, p));
    match list_mut(show, path.0, parents) {
        Some(list) if last < list.len() => list.remove(last),
        _ => Cue::new("", CueKind::Memo), // Unreachable: callers took the path from the show.
    }
}

/// A place to insert a cue: in which list, and at what index.
struct Slot {
    item: usize,
    via: Vec<usize>,
    index: usize,
}

/// The slot just before `before`, wherever it is (so a cue placed before a
/// child lands inside its group), or the end of the main list.
fn slot_before(show: &mut Show, before: Option<CueId>) -> Result<Slot, EditError> {
    if show.items.is_empty() {
        show.items.push(Item::new("Cues", ItemKind::General));
    }
    match before {
        Some(id) => {
            let (item, mut via) = path_of(show, id).ok_or(EditError::NoSuchCue)?;
            let index = via.pop().ok_or(EditError::NoSuchCue)?;
            Ok(Slot { item, via, index })
        }
        None => {
            let item = show.items.len() - 1;
            Ok(Slot {
                item,
                via: Vec::new(),
                index: show.items[item].cues.len(),
            })
        }
    }
}

fn insert_at(show: &mut Show, slot: &mut Slot, cue: Cue) -> Result<(), EditError> {
    let list = list_mut(show, slot.item, &slot.via).ok_or(EditError::NoSuchCue)?;
    let index = slot.index.min(list.len());
    list.insert(index, cue);
    slot.index = index + 1;
    Ok(())
}

fn next_number(show: &Show) -> String {
    let highest = show
        .items
        .iter()
        .flat_map(|i| i.cues.iter())
        .filter_map(|c| c.number.trim().parse::<u64>().ok())
        .max()
        .unwrap_or(0);
    (highest + 1).to_string()
}

fn fresh_ids(cue: &Cue) -> Cue {
    let mut copy = cue.clone();
    renew(&mut copy);
    copy
}

fn renew(cue: &mut Cue) {
    cue.id = CueId::new();
    if let CueKind::Group { children, .. } = &mut cue.kind {
        for child in children {
            renew(child);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cue::GroupMode;

    fn audio(name: &str) -> NewAudio {
        NewAudio {
            file: format!("{name}.wav"),
            name: name.into(),
        }
    }

    fn names(show: &Show) -> Vec<String> {
        show.all_cues().iter().map(|c| c.name.clone()).collect()
    }

    fn with_cues(names: &[&str]) -> (Show, Editor, Vec<CueId>) {
        let mut show = Show::new("t");
        let mut editor = Editor::new();
        let outcome = editor
            .apply(
                &mut show,
                Edit::AddAudio {
                    files: names.iter().map(|n| audio(n)).collect(),
                    before: None,
                },
                0.0,
            )
            .unwrap();
        (show, editor, outcome.created)
    }

    #[test]
    fn adding_creates_an_item_numbers_cues_and_reports_what_it_made() {
        let (show, _, ids) = with_cues(&["a", "b"]);
        assert_eq!(show.items.len(), 1);
        assert_eq!(names(&show), ["a", "b"]);
        assert_eq!(show.all_cues()[1].number, "2");
        assert_eq!(ids.len(), 2);
    }

    #[test]
    fn adding_before_a_cue_inserts_there() {
        let (mut show, mut editor, ids) = with_cues(&["a", "b"]);
        editor
            .apply(
                &mut show,
                Edit::AddAudio {
                    files: vec![audio("x")],
                    before: Some(ids[1]),
                },
                1.0,
            )
            .unwrap();
        assert_eq!(names(&show), ["a", "x", "b"]);
    }

    #[test]
    fn delete_move_and_duplicate_work_on_the_main_list() {
        let (mut show, mut editor, ids) = with_cues(&["a", "b", "c"]);
        editor
            .apply(
                &mut show,
                Edit::Move {
                    id: ids[2],
                    before: Some(ids[0]),
                },
                1.0,
            )
            .unwrap();
        assert_eq!(names(&show), ["c", "a", "b"]);
        editor
            .apply(
                &mut show,
                Edit::Move {
                    id: ids[2],
                    before: None,
                },
                2.0,
            )
            .unwrap();
        assert_eq!(names(&show), ["a", "b", "c"]);
        editor
            .apply(&mut show, Edit::Duplicate { id: ids[0] }, 3.0)
            .unwrap();
        assert_eq!(names(&show), ["a", "a", "b", "c"]);
        assert_ne!(
            show.all_cues()[0].id,
            show.all_cues()[1].id,
            "the copy has its own id"
        );
        editor
            .apply(
                &mut show,
                Edit::Delete {
                    ids: vec![ids[1], ids[2]],
                },
                4.0,
            )
            .unwrap();
        assert_eq!(names(&show), ["a", "a"]);
    }

    #[test]
    fn a_failed_edit_leaves_the_show_and_history_alone() {
        let (mut show, mut editor, ids) = with_cues(&["a"]);
        let before = show.clone();
        let result = editor.apply(
            &mut show,
            Edit::Change {
                id: ids[0],
                change: CueChange::PreWait(-1.0),
            },
            1.0,
        );
        assert!(matches!(result, Err(EditError::InvalidValue(_))));
        assert_eq!(show, before);
        editor.undo(&mut show);
        assert_eq!(
            names(&show),
            Vec::<String>::new(),
            "undo goes back past the add, not the failed edit"
        );
    }

    #[test]
    fn unknown_cues_and_wrong_kinds_are_reported_plainly() {
        let (mut show, mut editor, _) = with_cues(&["a"]);
        let memo = editor
            .apply(
                &mut show,
                Edit::AddMemo {
                    name: "note".into(),
                    before: None,
                },
                1.0,
            )
            .unwrap()
            .created[0];
        assert_eq!(
            editor.apply(
                &mut show,
                Edit::Delete {
                    ids: vec![CueId::new()]
                },
                2.0
            ),
            Err(EditError::NoSuchCue)
        );
        assert_eq!(
            editor.apply(
                &mut show,
                Edit::Change {
                    id: memo,
                    change: CueChange::Level(-3.0)
                },
                3.0
            ),
            Err(EditError::WrongKind)
        );
    }

    #[test]
    fn values_are_validated() {
        let (mut show, mut editor, ids) = with_cues(&["a"]);
        let id = ids[0];
        for change in [
            CueChange::Level(40.0),
            CueChange::Level(f64::NAN),
            CueChange::FadeIn(-0.5),
            CueChange::PostWait(f64::INFINITY),
            CueChange::End(Some(0.0)),
        ] {
            assert!(
                matches!(
                    editor.apply(
                        &mut show,
                        Edit::Change {
                            id,
                            change: change.clone()
                        },
                        1.0
                    ),
                    Err(EditError::InvalidValue(_))
                ),
                "{change:?}"
            );
        }
        editor
            .apply(
                &mut show,
                Edit::Change {
                    id,
                    change: CueChange::End(Some(10.0)),
                },
                2.0,
            )
            .unwrap();
        assert!(matches!(
            editor.apply(
                &mut show,
                Edit::Change {
                    id,
                    change: CueChange::Start(10.0)
                },
                3.0
            ),
            Err(EditError::InvalidValue(_))
        ));
    }

    #[test]
    fn undo_and_redo_walk_back_and_forward() {
        let (mut show, mut editor, ids) = with_cues(&["a"]);
        editor
            .apply(
                &mut show,
                Edit::Change {
                    id: ids[0],
                    change: CueChange::Name("b".into()),
                },
                10.0,
            )
            .unwrap();
        editor
            .apply(&mut show, Edit::Duplicate { id: ids[0] }, 20.0)
            .unwrap();
        assert_eq!(names(&show), ["b", "b"]);

        assert!(editor.undo(&mut show));
        assert_eq!(names(&show), ["b"]);
        assert!(editor.undo(&mut show));
        assert_eq!(names(&show), ["a"]);
        assert!(editor.redo(&mut show));
        assert!(editor.redo(&mut show));
        assert_eq!(names(&show), ["b", "b"]);
        assert!(!editor.redo(&mut show));
    }

    #[test]
    fn a_new_edit_clears_redo() {
        let (mut show, mut editor, ids) = with_cues(&["a"]);
        editor
            .apply(
                &mut show,
                Edit::Change {
                    id: ids[0],
                    change: CueChange::Armed(false),
                },
                5.0,
            )
            .unwrap();
        editor.undo(&mut show);
        assert!(editor.can_redo());
        editor
            .apply(
                &mut show,
                Edit::Change {
                    id: ids[0],
                    change: CueChange::Notes("x".into()),
                },
                6.0,
            )
            .unwrap();
        assert!(!editor.can_redo());
    }

    #[test]
    fn a_drag_of_one_setting_is_one_undo_step_but_other_settings_are_separate() {
        let (mut show, mut editor, ids) = with_cues(&["a"]);
        let id = ids[0];
        for (i, db) in [-1.0, -2.0, -3.0, -4.0].into_iter().enumerate() {
            editor
                .apply(
                    &mut show,
                    Edit::Change {
                        id,
                        change: CueChange::Level(db),
                    },
                    100.0 + i as f64 * 0.1,
                )
                .unwrap();
        }
        editor
            .apply(
                &mut show,
                Edit::Change {
                    id,
                    change: CueChange::FadeIn(2.0),
                },
                100.5,
            )
            .unwrap();

        editor.undo(&mut show);
        let level = |s: &Show| match &s.all_cues()[0].kind {
            CueKind::Audio { level, fade_in, .. } => (*level, *fade_in),
            _ => unreachable!(),
        };
        assert_eq!(level(&show), (-4.0, 0.0), "the fade-in is undone alone");
        editor.undo(&mut show);
        assert_eq!(level(&show), (0.0, 0.0), "the whole level drag is one step");
    }

    #[test]
    fn changes_to_the_same_setting_far_apart_are_separate_steps() {
        let (mut show, mut editor, ids) = with_cues(&["a"]);
        let id = ids[0];
        editor
            .apply(
                &mut show,
                Edit::Change {
                    id,
                    change: CueChange::Level(-1.0),
                },
                10.0,
            )
            .unwrap();
        editor
            .apply(
                &mut show,
                Edit::Change {
                    id,
                    change: CueChange::Level(-2.0),
                },
                20.0,
            )
            .unwrap();
        editor.undo(&mut show);
        assert!(matches!(&show.all_cues()[0].kind, CueKind::Audio { level, .. } if *level == -1.0));
    }

    #[test]
    fn setting_the_same_value_records_nothing() {
        let (mut show, mut editor, ids) = with_cues(&["a"]);
        let depth = editor.undo.len();
        editor
            .apply(
                &mut show,
                Edit::Change {
                    id: ids[0],
                    change: CueChange::Armed(true),
                },
                5.0,
            )
            .unwrap();
        assert_eq!(editor.undo.len(), depth);
    }

    #[test]
    fn history_is_capped() {
        let (mut show, mut editor, ids) = with_cues(&["a"]);
        for i in 0..(HISTORY_LIMIT + 20) {
            editor
                .apply(
                    &mut show,
                    Edit::Change {
                        id: ids[0],
                        change: CueChange::Number(i.to_string()),
                    },
                    i as f64 * 10.0,
                )
                .unwrap();
        }
        assert_eq!(editor.undo.len(), HISTORY_LIMIT);
    }

    #[test]
    fn renumber_numbers_everything_in_order() {
        let (mut show, mut editor, ids) = with_cues(&["a", "b", "c"]);
        editor
            .apply(&mut show, Edit::Delete { ids: vec![ids[1]] }, 1.0)
            .unwrap();
        editor.apply(&mut show, Edit::Renumber, 2.0).unwrap();
        let numbers: Vec<_> = show.all_cues().iter().map(|c| c.number.clone()).collect();
        assert_eq!(numbers, ["1", "2"]);
    }

    fn kind_of(show: &Show, id: CueId) -> &CueKind {
        &show
            .all_cues()
            .into_iter()
            .find(|c| c.id == id)
            .unwrap()
            .kind
    }

    fn children_names(show: &Show, group: CueId) -> Vec<String> {
        match kind_of(show, group) {
            CueKind::Group { children, .. } => children.iter().map(|c| c.name.clone()).collect(),
            _ => panic!("not a group"),
        }
    }

    fn group_of(show: &mut Show, editor: &mut Editor, ids: &[CueId], at: f64) -> CueId {
        editor
            .apply(show, Edit::Group { ids: ids.to_vec() }, at)
            .unwrap()
            .created[0]
    }

    #[test]
    fn grouping_wraps_cues_where_the_first_was_and_keeps_their_order() {
        let (mut show, mut editor, ids) = with_cues(&["a", "b", "c", "d"]);
        let group = group_of(&mut show, &mut editor, &[ids[2], ids[1]], 1.0);

        assert_eq!(names(&show), ["a", "Group", "b", "c", "d"]);
        assert_eq!(show.items[0].cues.len(), 3);
        assert_eq!(show.items[0].cues[1].id, group);
        assert_eq!(children_names(&show, group), ["b", "c"]);
    }

    #[test]
    fn ungrouping_puts_the_children_back_in_the_list() {
        let (mut show, mut editor, ids) = with_cues(&["a", "b", "c"]);
        let group = group_of(&mut show, &mut editor, &[ids[0], ids[1]], 1.0);

        editor
            .apply(&mut show, Edit::Ungroup { id: group }, 2.0)
            .unwrap();

        assert_eq!(names(&show), ["a", "b", "c"]);
        assert_eq!(show.items[0].cues.len(), 3);
        assert_eq!(
            editor.apply(&mut show, Edit::Ungroup { id: ids[2] }, 3.0),
            Err(EditError::WrongKind)
        );
    }

    #[test]
    fn grouping_refuses_nothing_selected_unknown_cues_and_cues_already_in_a_group() {
        let (mut show, mut editor, ids) = with_cues(&["a", "b"]);
        assert!(matches!(
            editor.apply(&mut show, Edit::Group { ids: vec![] }, 1.0),
            Err(EditError::InvalidValue(_))
        ));
        assert_eq!(
            editor.apply(
                &mut show,
                Edit::Group {
                    ids: vec![CueId::new()]
                },
                2.0
            ),
            Err(EditError::NoSuchCue)
        );
        group_of(&mut show, &mut editor, &[ids[0]], 3.0);
        assert_eq!(
            editor.apply(&mut show, Edit::Group { ids: vec![ids[0]] }, 4.0),
            Err(EditError::NotTopLevel)
        );
    }

    #[test]
    fn cues_inside_groups_can_be_changed_moved_duplicated_and_deleted() {
        let (mut show, mut editor, ids) = with_cues(&["a", "b", "c"]);
        let group = group_of(&mut show, &mut editor, &[ids[0], ids[1], ids[2]], 1.0);

        editor
            .apply(
                &mut show,
                Edit::Change {
                    id: ids[1],
                    change: CueChange::Name("B".into()),
                },
                2.0,
            )
            .unwrap();
        editor
            .apply(
                &mut show,
                Edit::Move {
                    id: ids[2],
                    before: Some(ids[0]),
                },
                3.0,
            )
            .unwrap();
        assert_eq!(children_names(&show, group), ["c", "a", "B"]);

        editor
            .apply(&mut show, Edit::Duplicate { id: ids[0] }, 4.0)
            .unwrap();
        assert_eq!(children_names(&show, group), ["c", "a", "a", "B"]);

        editor
            .apply(&mut show, Edit::Delete { ids: vec![ids[1]] }, 5.0)
            .unwrap();
        assert_eq!(children_names(&show, group), ["c", "a", "a"]);
    }

    #[test]
    fn a_cue_moves_into_a_group_by_going_before_one_of_its_children() {
        let (mut show, mut editor, ids) = with_cues(&["a", "b", "c"]);
        let group = group_of(&mut show, &mut editor, &[ids[1], ids[2]], 1.0);

        editor
            .apply(
                &mut show,
                Edit::Move {
                    id: ids[0],
                    before: Some(ids[2]),
                },
                2.0,
            )
            .unwrap();
        assert_eq!(children_names(&show, group), ["b", "a", "c"]);

        // And out again, to the end of the main list.
        editor
            .apply(
                &mut show,
                Edit::Move {
                    id: ids[0],
                    before: None,
                },
                3.0,
            )
            .unwrap();
        assert_eq!(children_names(&show, group), ["b", "c"]);
        assert_eq!(names(&show).last().unwrap(), "a");
    }

    #[test]
    fn a_group_cannot_be_moved_inside_itself() {
        let (mut show, mut editor, ids) = with_cues(&["a", "b"]);
        let group = group_of(&mut show, &mut editor, &[ids[0], ids[1]], 1.0);
        let before = show.clone();

        let result = editor.apply(
            &mut show,
            Edit::Move {
                id: group,
                before: Some(ids[1]),
            },
            2.0,
        );

        assert!(matches!(result, Err(EditError::InvalidValue(_))));
        assert_eq!(show, before);
    }

    #[test]
    fn deleting_a_group_takes_its_children_and_undo_brings_them_back() {
        let (mut show, mut editor, ids) = with_cues(&["a", "b", "c"]);
        let group = group_of(&mut show, &mut editor, &[ids[0], ids[1]], 1.0);

        editor
            .apply(
                &mut show,
                Edit::Delete {
                    ids: vec![group, ids[0]],
                },
                2.0,
            )
            .unwrap();
        assert_eq!(names(&show), ["c"]);

        editor.undo(&mut show);
        assert_eq!(children_names(&show, group), ["a", "b"]);
    }

    #[test]
    fn group_settings_can_be_changed() {
        let (mut show, mut editor, ids) = with_cues(&["a", "b"]);
        let group = group_of(&mut show, &mut editor, &[ids[0], ids[1]], 1.0);

        editor
            .apply(
                &mut show,
                Edit::Change {
                    id: group,
                    change: CueChange::GroupMode(GroupMode::Sequence),
                },
                2.0,
            )
            .unwrap();
        editor
            .apply(
                &mut show,
                Edit::Change {
                    id: group,
                    change: CueChange::Looped(true),
                },
                3.0,
            )
            .unwrap();

        assert!(matches!(
            kind_of(&show, group),
            CueKind::Group {
                mode: GroupMode::Sequence,
                looped: true,
                ..
            }
        ));
        assert_eq!(
            editor.apply(
                &mut show,
                Edit::Change {
                    id: ids[0],
                    change: CueChange::Looped(true)
                },
                4.0
            ),
            Err(EditError::WrongKind)
        );
    }

    #[test]
    fn control_cues_are_added_with_sensible_defaults_and_named_after_their_target() {
        let (mut show, mut editor, ids) = with_cues(&["Pad", "Song"]);
        for (kind, name) in [
            (ControlKind::Fade, "Fade Pad"),
            (ControlKind::Stop, "Stop Pad"),
            (ControlKind::Pause, "Pause Pad"),
            (ControlKind::GoTo, "Go to Pad"),
            (ControlKind::Arm, "Arm Pad"),
        ] {
            let id = editor
                .apply(
                    &mut show,
                    Edit::AddControl {
                        kind,
                        target: ids[0],
                        before: Some(ids[1]),
                    },
                    1.0,
                )
                .unwrap()
                .created[0];
            let cue = show.all_cues().into_iter().find(|c| c.id == id).unwrap();
            assert_eq!(cue.name, name);
            assert!(cue.kind.targets().contains(&ids[0]));
        }
        let fade = show
            .all_cues()
            .into_iter()
            .find(|c| c.name == "Fade Pad")
            .unwrap();
        assert!(
            matches!(fade.kind, CueKind::Fade { to, duration, stop_when_done: true, .. } if to == -60.0 && duration == 3.0)
        );
        assert_eq!(
            names(&show)[..2],
            ["Pad", "Fade Pad"],
            "placed before the song"
        );
    }

    #[test]
    fn a_control_cue_needs_a_real_target_and_go_to_needs_a_main_list_target() {
        let (mut show, mut editor, ids) = with_cues(&["a", "b"]);
        assert_eq!(
            editor.apply(
                &mut show,
                Edit::AddControl {
                    kind: ControlKind::Stop,
                    target: CueId::new(),
                    before: None
                },
                1.0
            ),
            Err(EditError::NoSuchCue)
        );
        group_of(&mut show, &mut editor, &[ids[1]], 2.0);
        assert_eq!(
            editor.apply(
                &mut show,
                Edit::AddControl {
                    kind: ControlKind::GoTo,
                    target: ids[1],
                    before: None
                },
                3.0
            ),
            Err(EditError::NotTopLevel)
        );
        assert!(
            editor
                .apply(
                    &mut show,
                    Edit::AddControl {
                        kind: ControlKind::Stop,
                        target: ids[1],
                        before: None
                    },
                    4.0
                )
                .is_ok()
        );
    }

    #[test]
    fn control_cue_settings_can_be_changed_and_validated() {
        let (mut show, mut editor, ids) = with_cues(&["a", "b"]);
        let fade = editor
            .apply(
                &mut show,
                Edit::AddControl {
                    kind: ControlKind::Fade,
                    target: ids[0],
                    before: None,
                },
                1.0,
            )
            .unwrap()
            .created[0];
        let change = |editor: &mut Editor, show: &mut Show, change, at| {
            editor.apply(show, Edit::Change { id: fade, change }, at)
        };

        change(&mut editor, &mut show, CueChange::Target(ids[1]), 2.0).unwrap();
        change(&mut editor, &mut show, CueChange::FadeTo(-20.0), 3.0).unwrap();
        change(&mut editor, &mut show, CueChange::FadeDuration(8.0), 4.0).unwrap();
        change(&mut editor, &mut show, CueChange::StopWhenDone(false), 5.0).unwrap();
        change(
            &mut editor,
            &mut show,
            CueChange::Curve(FadeCurve::Linear),
            6.0,
        )
        .unwrap();

        assert!(matches!(
            kind_of(&show, fade),
            CueKind::Fade { target, to, duration, curve: FadeCurve::Linear, stop_when_done: false }
                if *target == ids[1] && *to == -20.0 && *duration == 8.0
        ));
        assert!(matches!(
            change(&mut editor, &mut show, CueChange::FadeTo(50.0), 7.0),
            Err(EditError::InvalidValue(_))
        ));
        assert_eq!(
            change(&mut editor, &mut show, CueChange::Target(CueId::new()), 8.0),
            Err(EditError::NoSuchCue)
        );
        assert_eq!(
            change(&mut editor, &mut show, CueChange::Level(-3.0), 9.0),
            Err(EditError::WrongKind)
        );
        assert_eq!(
            editor.apply(
                &mut show,
                Edit::Change {
                    id: ids[0],
                    change: CueChange::FadeTo(-3.0)
                },
                10.0
            ),
            Err(EditError::WrongKind)
        );
    }

    #[test]
    fn an_arm_cue_can_switch_between_arming_and_disarming() {
        let (mut show, mut editor, ids) = with_cues(&["a"]);
        let arm = editor
            .apply(
                &mut show,
                Edit::AddControl {
                    kind: ControlKind::Arm,
                    target: ids[0],
                    before: None,
                },
                1.0,
            )
            .unwrap()
            .created[0];
        editor
            .apply(
                &mut show,
                Edit::Change {
                    id: arm,
                    change: CueChange::ArmAction(false),
                },
                2.0,
            )
            .unwrap();
        assert!(matches!(
            kind_of(&show, arm),
            CueKind::Arm { armed: false, .. }
        ));
    }
}
