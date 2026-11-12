//! Editing a show, with undo and redo.
//!
//! Every change to a show goes through [`Editor::apply`]. An edit either
//! succeeds completely or leaves the show untouched. Undo restores earlier
//! snapshots rather than computing inverses, which keeps it correct for every
//! edit, including ones added later. Shows are small, so this is cheap.

use crate::cue::{Cue, CueKind, Decibels, FadeCurve, Follow, Seconds};
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
}

impl CueChange {
    /// Changes of the same kind to the same cue, made in quick succession (a
    /// slider being dragged, text being typed), share one undo step.
    fn mergeable(&self) -> bool {
        !matches!(
            self,
            Self::Armed(_) | Self::Follow(_) | Self::Curve(_) | Self::Loops(_)
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum EditError {
    #[error("That cue can't be found. It may have been deleted.")]
    NoSuchCue,
    #[error("That can only be done to a cue in the main list, not one inside a group.")]
    NotTopLevel,
    #[error("That setting only applies to audio cues.")]
    NotAudio,
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
            let mut created = Vec::new();
            let mut at = insertion_point(show, before)?;
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
                insert(show, &mut at, cue);
            }
            Ok(Outcome { created })
        }
        Edit::AddMemo { name, before } => {
            let mut at = insertion_point(show, before)?;
            let mut cue = Cue::new(name, CueKind::Memo);
            cue.number = next_number(show);
            let id = cue.id;
            insert(show, &mut at, cue);
            Ok(Outcome { created: vec![id] })
        }
        Edit::Delete { ids } => {
            for id in &ids {
                locate(show, *id)?;
            }
            for item in &mut show.items {
                item.cues.retain(|c| !ids.contains(&c.id));
            }
            Ok(Outcome::default())
        }
        Edit::Move { id, before } => {
            if before == Some(id) {
                return Ok(Outcome::default());
            }
            let (item, index) = locate(show, id)?;
            let cue = show.items[item].cues.remove(index);
            let mut at = insertion_point(show, before)?;
            insert(show, &mut at, cue);
            Ok(Outcome::default())
        }
        Edit::Duplicate { id } => {
            let (item, index) = locate(show, id)?;
            let mut copy = fresh_ids(&show.items[item].cues[index]);
            copy.number = next_number(show);
            let created = copy.id;
            show.items[item].cues.insert(index + 1, copy);
            Ok(Outcome {
                created: vec![created],
            })
        }
        Edit::Change { id, change } => {
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
    }
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
        audio_only => {
            let CueKind::Audio {
                level,
                start,
                end,
                loops,
                fade_in,
                fade_out,
                curve,
                ..
            } = &mut cue.kind
            else {
                return Err(EditError::NotAudio);
            };
            match audio_only {
                CueChange::Level(v) => {
                    if !v.is_finite() || !(MIN_LEVEL_DB..=MAX_LEVEL_DB).contains(&v) {
                        return Err(EditError::InvalidValue(
                            "The level must be between -120 and +24 dB.",
                        ));
                    }
                    *level = v;
                }
                CueChange::Start(v) => {
                    let v = seconds(v)?;
                    if end.is_some_and(|e| v >= e) {
                        return Err(EditError::InvalidValue("The start must be before the end."));
                    }
                    *start = v;
                }
                CueChange::End(v) => {
                    if let Some(v) = v
                        && seconds(v)? <= *start
                    {
                        return Err(EditError::InvalidValue("The end must be after the start."));
                    }
                    *end = v;
                }
                CueChange::Loops(v) => *loops = v,
                CueChange::FadeIn(v) => *fade_in = seconds(v)?,
                CueChange::FadeOut(v) => *fade_out = seconds(v)?,
                CueChange::Curve(v) => *curve = v,
                _ => {}
            }
        }
    }
    Ok(())
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

fn locate(show: &Show, id: CueId) -> Result<(usize, usize), EditError> {
    for (i, item) in show.items.iter().enumerate() {
        if let Some(j) = item.cues.iter().position(|c| c.id == id) {
            return Ok((i, j));
        }
    }
    if show.all_cues().iter().any(|c| c.id == id) {
        Err(EditError::NotTopLevel)
    } else {
        Err(EditError::NoSuchCue)
    }
}

/// A position to insert at: (item, index). Ensures an item exists.
fn insertion_point(show: &mut Show, before: Option<CueId>) -> Result<(usize, usize), EditError> {
    if show.items.is_empty() {
        show.items.push(Item::new("Cues", ItemKind::General));
    }
    match before {
        Some(id) => locate(show, id),
        None => {
            let last = show.items.len() - 1;
            Ok((last, show.items[last].cues.len()))
        }
    }
}

fn insert(show: &mut Show, at: &mut (usize, usize), cue: Cue) {
    show.items[at.0].cues.insert(at.1, cue);
    at.1 += 1;
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
            Err(EditError::NotAudio)
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

    #[test]
    fn cues_inside_groups_can_be_changed_but_not_moved() {
        let child = Cue::new("child", CueKind::Memo);
        let child_id = child.id;
        let group = Cue::new(
            "g",
            CueKind::Group {
                mode: GroupMode::Together,
                children: vec![child],
                looped: false,
            },
        );
        let mut show = Show::new("t");
        let mut item = Item::new("i", ItemKind::General);
        item.cues.push(group);
        show.items.push(item);
        let mut editor = Editor::new();

        editor
            .apply(
                &mut show,
                Edit::Change {
                    id: child_id,
                    change: CueChange::Name("renamed".into()),
                },
                1.0,
            )
            .unwrap();
        assert!(show.all_cues().iter().any(|c| c.name == "renamed"));
        assert_eq!(
            editor.apply(
                &mut show,
                Edit::Delete {
                    ids: vec![child_id]
                },
                2.0
            ),
            Err(EditError::NotTopLevel)
        );
    }
}
