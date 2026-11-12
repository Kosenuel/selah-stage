//! The show: the saved document for one service.

use serde::{Deserialize, Serialize};

use crate::cue::Cue;
use crate::id::{ItemId, PatchId};

/// Bumped on every change to the saved format; each bump needs a migration
/// in `selah-persist` and a fixture of the old version.
pub const CURRENT_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ItemKind {
    #[default]
    General,
    Song,
    Scripture,
    Message,
    Announcement,
    Media,
}

/// A segment of the service plan. Holds cues.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Item {
    pub id: ItemId,
    pub title: String,
    pub kind: ItemKind,
    pub notes: String,
    /// Planned length in seconds, if any.
    pub planned_length: Option<f64>,
    pub cues: Vec<Cue>,
}

impl Item {
    pub fn new(title: impl Into<String>, kind: ItemKind) -> Self {
        Self {
            id: ItemId::new(),
            title: title.into(),
            kind,
            notes: String::new(),
            planned_length: None,
            cues: Vec::new(),
        }
    }
}

/// A named logical output such as "Main" or "In-ears". Which device channels
/// it maps to is per machine and is not stored in the show.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Patch {
    pub id: PatchId,
    pub name: String,
    /// How many channels the patch carries (1 mono, 2 stereo).
    pub channels: u8,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShowMeta {
    pub title: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Show {
    pub schema_version: u32,
    pub meta: ShowMeta,
    pub items: Vec<Item>,
    pub patches: Vec<Patch>,
}

impl Show {
    /// An empty show with one stereo "Main" patch.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            schema_version: CURRENT_SCHEMA_VERSION,
            meta: ShowMeta {
                title: title.into(),
            },
            items: Vec::new(),
            patches: vec![Patch {
                id: PatchId::new(),
                name: "Main".into(),
                channels: 2,
            }],
        }
    }

    /// Every top-level and nested cue in plan order.
    pub fn all_cues(&self) -> Vec<&Cue> {
        let mut out = Vec::new();
        for item in &self.items {
            for cue in &item.cues {
                cue.walk(&mut |c| out.push(c));
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cue::CueKind;

    #[test]
    fn a_new_show_has_a_main_patch_and_current_version() {
        let show = Show::new("Sunday");
        assert_eq!(show.schema_version, CURRENT_SCHEMA_VERSION);
        assert_eq!(show.patches.len(), 1);
        assert_eq!(show.patches[0].name, "Main");
    }

    #[test]
    fn round_trips_through_json() {
        let mut show = Show::new("Sunday");
        let mut item = Item::new("Worship", ItemKind::Song);
        item.cues.push(Cue::new("note", CueKind::Memo));
        show.items.push(item);
        let json = serde_json::to_string(&show).unwrap();
        assert!(json.contains("schemaVersion"));
        assert_eq!(serde_json::from_str::<Show>(&json).unwrap(), show);
        assert_eq!(show.all_cues().len(), 1);
    }
}
