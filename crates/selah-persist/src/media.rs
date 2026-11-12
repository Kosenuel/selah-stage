use std::path::Path;

use selah_core::{CueId, CueKind, Show};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MissingMedia {
    pub cue: CueId,
    /// The path as written in the show, relative to the show folder.
    pub file: String,
}

/// Media files the show refers to that are not in `folder` (FILE-5).
pub fn missing_media(show: &Show, folder: &Path) -> Vec<MissingMedia> {
    show.all_cues()
        .into_iter()
        .filter_map(|cue| match &cue.kind {
            CueKind::Audio { file, .. } if !folder.join(file).is_file() => Some(MissingMedia {
                cue: cue.id,
                file: file.clone(),
            }),
            _ => None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use selah_core::{Cue, FadeCurve, Item, ItemKind};

    fn audio(file: &str) -> Cue {
        Cue::new(
            file,
            CueKind::Audio {
                file: file.into(),
                level: 0.0,
                start: 0.0,
                end: None,
                loops: None,
                fade_in: 0.0,
                fade_out: 0.0,
                curve: FadeCurve::Linear,
                patches: Vec::new(),
            },
        )
    }

    #[test]
    fn lists_only_the_files_that_are_not_there() {
        let d = std::env::temp_dir().join(format!("selah-media-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join("here.wav"), b"x").unwrap();
        let mut show = Show::new("t");
        let mut item = Item::new("i", ItemKind::General);
        item.cues = vec![audio("here.wav"), audio("gone.wav")];
        show.items.push(item);

        let missing = missing_media(&show, &d);

        assert_eq!(missing.len(), 1);
        assert_eq!(missing[0].file, "gone.wav");
    }
}
