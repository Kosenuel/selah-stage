use std::fs;
use std::path::Path;

use selah_core::{CueId, CueKind, Show};

use crate::error::PersistError;

/// Where imported media is kept, inside the show folder.
pub const MEDIA_DIR: &str = "media";

/// Makes `source` part of the show and returns the path to store in a cue,
/// relative to the show folder and always with `/` so shows move between
/// machines.
///
/// A file already inside the show folder is used where it is. Any other is
/// copied into `media/`, under a new name if that name is taken, so nothing is
/// ever overwritten.
pub fn import_media(show_folder: &Path, source: &Path) -> Result<String, PersistError> {
    let unreadable = || PersistError::Unreadable(PersistError::path(source));
    let cannot_write = || PersistError::CannotWrite(PersistError::path(show_folder));
    let source = fs::canonicalize(source).map_err(|_| unreadable())?;
    let root = fs::canonicalize(show_folder).map_err(|_| cannot_write())?;

    if let Ok(inside) = source.strip_prefix(&root) {
        return Ok(slashed(inside));
    }

    let name = source.file_name().ok_or_else(unreadable)?;
    let directory = root.join(MEDIA_DIR);
    fs::create_dir_all(&directory).map_err(|_| cannot_write())?;
    let stem = Path::new(name)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("audio")
        .to_owned();
    let extension = Path::new(name)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| format!(".{e}"))
        .unwrap_or_default();
    let mut target = directory.join(name);
    let mut counter = 2;
    while target.exists() {
        target = directory.join(format!("{stem} ({counter}){extension}"));
        counter += 1;
    }
    fs::copy(&source, &target).map_err(|_| cannot_write())?;
    let relative = target.strip_prefix(&root).map_err(|_| cannot_write())?;
    Ok(slashed(relative))
}

fn slashed(path: &Path) -> String {
    path.components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

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

    fn temp(tag: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("selah-import-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn a_file_outside_the_show_is_copied_into_media() {
        let (show, elsewhere) = (temp("show"), temp("elsewhere"));
        std::fs::write(elsewhere.join("song.wav"), b"audio").unwrap();

        let relative = import_media(&show, &elsewhere.join("song.wav")).unwrap();

        assert_eq!(relative, "media/song.wav");
        assert_eq!(
            std::fs::read(show.join("media/song.wav")).unwrap(),
            b"audio"
        );
    }

    #[test]
    fn a_file_already_in_the_show_is_used_where_it_is() {
        let show = temp("inside");
        std::fs::create_dir_all(show.join("music")).unwrap();
        std::fs::write(show.join("music/a.wav"), b"x").unwrap();

        assert_eq!(
            import_media(&show, &show.join("music/a.wav")).unwrap(),
            "music/a.wav"
        );
        assert!(!show.join("media").exists(), "nothing was copied");
    }

    #[test]
    fn a_name_clash_never_overwrites() {
        let (show, elsewhere) = (temp("clash"), temp("clash-src"));
        std::fs::write(elsewhere.join("a.wav"), b"new").unwrap();
        std::fs::create_dir_all(show.join("media")).unwrap();
        std::fs::write(show.join("media/a.wav"), b"old").unwrap();

        let relative = import_media(&show, &elsewhere.join("a.wav")).unwrap();

        assert_eq!(relative, "media/a (2).wav");
        assert_eq!(std::fs::read(show.join("media/a.wav")).unwrap(), b"old");
    }

    #[test]
    fn a_missing_source_is_reported() {
        let show = temp("nosource");
        assert!(matches!(
            import_media(&show, &show.join("nope.wav")),
            Err(PersistError::Unreadable(_))
        ));
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
