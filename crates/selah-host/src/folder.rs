//! Building a show from a folder of audio files.

use std::path::{Path, PathBuf};

use selah_core::{Cue, CueKind, FadeCurve, Item, ItemKind, Show};

const AUDIO_EXTENSIONS: &[&str] = &["wav", "flac", "mp3", "m4a", "aac", "ogg", "aif", "aiff"];

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum FolderError {
    #[error("Can't open the folder {0}. Check the path and try again.")]
    Unreadable(String),
    #[error("There are no audio files in {0}.")]
    Empty(String),
}

/// One audio cue per file, in name order, with the folder as the media root.
pub fn show_from_folder(folder: &Path) -> Result<(Show, PathBuf), FolderError> {
    let shown = folder.display().to_string();
    let entries = std::fs::read_dir(folder).map_err(|_| FolderError::Unreadable(shown.clone()))?;
    let mut files: Vec<String> = entries
        .filter_map(Result::ok)
        .filter(|e| e.path().is_file())
        .filter_map(|e| e.file_name().into_string().ok())
        .filter(|name| {
            Path::new(name)
                .extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| AUDIO_EXTENSIONS.contains(&e.to_ascii_lowercase().as_str()))
        })
        .collect();
    if files.is_empty() {
        return Err(FolderError::Empty(shown));
    }
    files.sort_by_key(|name| name.to_lowercase());

    let title = folder
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("Folder")
        .to_owned();
    let mut item = Item::new(title.clone(), ItemKind::Media);
    for (i, file) in files.into_iter().enumerate() {
        let name = Path::new(&file)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(&file)
            .to_owned();
        let mut cue = Cue::new(
            name,
            CueKind::Audio {
                file,
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
        cue.number = (i + 1).to_string();
        item.cues.push(cue);
    }
    let mut show = Show::new(title);
    show.items.push(item);
    Ok((show, folder.to_path_buf()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dir(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("selah-folder-{tag}-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn builds_one_cue_per_audio_file_in_name_order() {
        let d = dir("order");
        for name in ["b.WAV", "a.mp3", "notes.txt", "C.flac"] {
            std::fs::write(d.join(name), b"x").unwrap();
        }
        let (show, root) = show_from_folder(&d).unwrap();
        let names: Vec<_> = show.all_cues().iter().map(|c| c.name.clone()).collect();
        assert_eq!(names, ["a", "b", "C"]);
        assert_eq!(show.all_cues()[0].number, "1");
        assert_eq!(root, d);
    }

    #[test]
    fn reports_an_empty_or_missing_folder_plainly() {
        let d = dir("empty");
        assert!(matches!(show_from_folder(&d), Err(FolderError::Empty(_))));
        assert!(matches!(
            show_from_folder(&d.join("nope")),
            Err(FolderError::Unreadable(_))
        ));
    }
}
