//! Checks that a show hangs together. Problems are reported, never fatal:
//! principle 1 says a service must still run.

use std::collections::HashSet;

use crate::cue::CueKind;
use crate::id::CueId;
use crate::show::Show;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IssueKind {
    /// Two cues share an id. The show cannot be trusted.
    DuplicateId,
    /// A control cue points at a cue that does not exist.
    MissingTarget,
    /// An audio cue routes to a patch that does not exist.
    MissingPatch,
    /// A wait, fade, or trim is negative or not a number.
    BadTime,
    /// Two cues share a number. A warning only.
    DuplicateNumber,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Issue {
    pub kind: IssueKind,
    pub cue: CueId,
}

pub fn validate(show: &Show) -> Vec<Issue> {
    let cues = show.all_cues();
    let ids: HashSet<CueId> = cues.iter().map(|c| c.id).collect();
    let patches: HashSet<_> = show.patches.iter().map(|p| p.id).collect();
    let mut issues = Vec::new();

    let mut seen = HashSet::new();
    let mut numbers = HashSet::new();
    for cue in &cues {
        let mut flag = |kind| issues.push(Issue { kind, cue: cue.id });
        if !seen.insert(cue.id) {
            flag(IssueKind::DuplicateId);
        }
        if !cue.number.is_empty() && !numbers.insert(cue.number.clone()) {
            flag(IssueKind::DuplicateNumber);
        }
        for target in cue.kind.targets() {
            if !ids.contains(&target) {
                flag(IssueKind::MissingTarget);
            }
        }
        let bad = |t: f64| !t.is_finite() || t < 0.0;
        let mut times = vec![cue.pre_wait, cue.post_wait];
        if let CueKind::Audio {
            patches: routed,
            start,
            end,
            fade_in,
            fade_out,
            ..
        } = &cue.kind
        {
            times.extend([*start, *fade_in, *fade_out]);
            times.extend(*end);
            if routed.iter().any(|p| !patches.contains(p)) {
                flag(IssueKind::MissingPatch);
            }
        }
        if let CueKind::Fade { duration, .. } = &cue.kind {
            times.push(*duration);
        }
        if times.into_iter().any(bad) {
            flag(IssueKind::BadTime);
        }
    }
    issues
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cue::Cue;
    use crate::show::{Item, ItemKind};

    fn show_with(cues: Vec<Cue>) -> Show {
        let mut show = Show::new("t");
        let mut item = Item::new("i", ItemKind::General);
        item.cues = cues;
        show.items.push(item);
        show
    }

    #[test]
    fn a_clean_show_has_no_issues() {
        assert!(validate(&show_with(vec![Cue::new("n", CueKind::Memo)])).is_empty());
    }

    #[test]
    fn reports_missing_targets_and_bad_times() {
        let mut stop = Cue::new(
            "stop",
            CueKind::Stop {
                target: CueId::new(),
            },
        );
        stop.pre_wait = -1.0;
        let kinds: Vec<_> = validate(&show_with(vec![stop]))
            .into_iter()
            .map(|i| i.kind)
            .collect();
        assert!(kinds.contains(&IssueKind::MissingTarget));
        assert!(kinds.contains(&IssueKind::BadTime));
    }

    #[test]
    fn duplicate_numbers_are_reported_once_per_extra_cue() {
        let (mut a, mut b) = (Cue::new("a", CueKind::Memo), Cue::new("b", CueKind::Memo));
        a.number = "1".into();
        b.number = "1".into();
        let issues = validate(&show_with(vec![a, b]));
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].kind, IssueKind::DuplicateNumber);
    }

    #[test]
    fn nan_waits_are_bad() {
        let mut cue = Cue::new("x", CueKind::Memo);
        cue.post_wait = f64::NAN;
        assert_eq!(validate(&show_with(vec![cue]))[0].kind, IssueKind::BadTime);
    }
}
