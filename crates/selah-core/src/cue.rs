//! Cues: the unit of operation.

use serde::{Deserialize, Serialize};

use crate::id::{CueId, PatchId};

/// Seconds, at every boundary.
pub type Seconds = f64;
/// Decibels. Gain is converted to linear only inside DSP code.
pub type Decibels = f64;

/// What happens to the cue after this one in the list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Follow {
    /// Wait for the operator's next GO.
    #[default]
    None,
    /// Start the next cue when this one starts (after the post-wait).
    Continue,
    /// Start the next cue when this one finishes.
    Follow,
}

/// The shape of a fade.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FadeCurve {
    Linear,
    /// Keeps perceived loudness steady when crossfading.
    #[default]
    EqualPower,
    SCurve,
}

/// How a group's children fire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum GroupMode {
    /// All children at once.
    #[default]
    Together,
    /// One after another.
    Sequence,
    /// One at random each time.
    Shuffle,
}

/// What a cue does. Later milestones add lyrics, video, tracks and so on.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum CueKind {
    /// Play a sound file.
    Audio {
        /// Path relative to the show folder.
        file: String,
        level: Decibels,
        /// Where in the file playback starts and ends, in seconds.
        start: Seconds,
        end: Option<Seconds>,
        /// How many times the trimmed region plays. `None` and `Some(1)` play
        /// once, `Some(n)` plays `n` times, and `Some(0)` loops forever.
        loops: Option<u32>,
        fade_in: Seconds,
        fade_out: Seconds,
        curve: FadeCurve,
        patches: Vec<PatchId>,
    },
    /// Change another cue's level over time.
    Fade {
        target: CueId,
        to: Decibels,
        duration: Seconds,
        curve: FadeCurve,
        stop_when_done: bool,
    },
    Stop {
        target: CueId,
    },
    Pause {
        target: CueId,
    },
    /// Move the playhead without firing.
    GoTo {
        target: CueId,
    },
    /// Arm or disarm another cue.
    Arm {
        target: CueId,
        armed: bool,
    },
    /// Holds cues that fire together, in sequence, or shuffled.
    Group {
        mode: GroupMode,
        children: Vec<Cue>,
        looped: bool,
    },
    /// A note in the list; does nothing.
    Memo,
}

impl CueKind {
    /// Cues this one acts on, for validation.
    pub fn targets(&self) -> Vec<CueId> {
        match self {
            Self::Fade { target, .. }
            | Self::Stop { target }
            | Self::Pause { target }
            | Self::GoTo { target }
            | Self::Arm { target, .. } => vec![*target],
            _ => Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Cue {
    pub id: CueId,
    /// A user-facing label. Not enforced unique; validation only warns.
    pub number: String,
    pub name: String,
    pub notes: String,
    pub color: Option<String>,
    pub armed: bool,
    pub pre_wait: Seconds,
    pub post_wait: Seconds,
    pub follow: Follow,
    pub kind: CueKind,
}

impl Cue {
    /// A new armed cue with no waits.
    pub fn new(name: impl Into<String>, kind: CueKind) -> Self {
        Self {
            id: CueId::new(),
            number: String::new(),
            name: name.into(),
            notes: String::new(),
            color: None,
            armed: true,
            pre_wait: 0.0,
            post_wait: 0.0,
            follow: Follow::None,
            kind,
        }
    }

    /// This cue and every cue inside it.
    pub fn walk<'a>(&'a self, visit: &mut impl FnMut(&'a Cue)) {
        visit(self);
        if let CueKind::Group { children, .. } = &self.kind {
            for child in children {
                child.walk(visit);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialises_with_camel_case_and_a_type_tag() {
        let cue = Cue::new(
            "Stop it",
            CueKind::Fade {
                target: CueId::new(),
                to: -60.0,
                duration: 2.0,
                curve: FadeCurve::Linear,
                stop_when_done: true,
            },
        );
        let json = serde_json::to_value(&cue).unwrap();
        assert_eq!(json["kind"]["type"], "fade");
        assert_eq!(json["kind"]["stopWhenDone"], true);
        assert_eq!(json["preWait"], 0.0);
        assert_eq!(serde_json::from_value::<Cue>(json).unwrap(), cue);
    }

    #[test]
    fn walk_visits_nested_cues() {
        let child = Cue::new("child", CueKind::Memo);
        let group = Cue::new(
            "group",
            CueKind::Group {
                mode: GroupMode::Together,
                children: vec![child],
                looped: false,
            },
        );
        let mut names = Vec::new();
        group.walk(&mut |c| names.push(c.name.clone()));
        assert_eq!(names, ["group", "child"]);
    }
}
