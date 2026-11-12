use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use selah_core::Show;

use crate::error::PersistError;
use crate::migrate;

pub const SHOW_FILE: &str = "show.json";
pub const BACKUPS_DIR: &str = "backups";
/// How many earlier versions are kept.
pub const BACKUPS_KEPT: usize = 20;

/// A show read from disk.
#[derive(Debug, Clone, PartialEq)]
pub struct Loaded {
    pub show: Show,
    /// Set when `show.json` was damaged and this backup was used instead.
    pub recovered_from: Option<PathBuf>,
}

pub fn is_show_folder(folder: &Path) -> bool {
    folder.join(SHOW_FILE).is_file()
}

/// Reads the show in `folder`, upgrading old formats and recovering from the
/// newest readable backup if the main file is damaged.
pub fn load(folder: &Path) -> Result<Loaded, PersistError> {
    let main = folder.join(SHOW_FILE);
    if !main.is_file() {
        return Err(PersistError::NotAShow(PersistError::path(folder)));
    }
    match read_show(&main) {
        Ok(show) => Ok(Loaded {
            show,
            recovered_from: None,
        }),
        // A newer format is not damage; do not paper over it with a backup.
        Err(error @ PersistError::TooNew { .. }) => Err(error),
        Err(_) => {
            for backup in backups_newest_first(folder) {
                if let Ok(show) = read_show(&backup) {
                    tracing::warn!(backup = %backup.display(), "recovered a show from a backup");
                    return Ok(Loaded {
                        show,
                        recovered_from: Some(backup),
                    });
                }
            }
            Err(PersistError::Corrupt(PersistError::path(folder)))
        }
    }
}

fn read_show(path: &Path) -> Result<Show, PersistError> {
    let unreadable = || PersistError::Unreadable(PersistError::path(path));
    let text = fs::read_to_string(path).map_err(|_| unreadable())?;
    let value: serde_json::Value =
        serde_json::from_str(&text).map_err(|_| PersistError::Corrupt(PersistError::path(path)))?;
    let value = migrate::to_current(value)?;
    serde_json::from_value(value).map_err(|_| PersistError::Corrupt(PersistError::path(path)))
}

/// Writes `show` to `folder`, creating it if needed. The previous version, if
/// different, is kept in `backups/`.
pub fn save(folder: &Path, show: &Show) -> Result<(), PersistError> {
    let cannot = || PersistError::CannotWrite(PersistError::path(folder));
    fs::create_dir_all(folder).map_err(|_| cannot())?;
    let json = serde_json::to_vec_pretty(show).map_err(|_| cannot())?;

    let main = folder.join(SHOW_FILE);
    if let Ok(previous) = fs::read(&main)
        && previous != json
    {
        // A failed backup must not stop the save itself.
        if let Err(error) = backup(folder, &previous) {
            tracing::warn!(%error, "could not back up the previous version");
        }
    }

    let temporary = folder.join(format!("{SHOW_FILE}.tmp"));
    let mut file = File::create(&temporary).map_err(|_| cannot())?;
    file.write_all(&json).map_err(|_| cannot())?;
    // Data must be on disk before the rename makes it the show.
    file.sync_all().map_err(|_| cannot())?;
    drop(file);
    fs::rename(&temporary, &main).map_err(|_| cannot())
}

fn backup(folder: &Path, previous: &[u8]) -> std::io::Result<()> {
    let dir = folder.join(BACKUPS_DIR);
    fs::create_dir_all(&dir)?;
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis());
    // Zero-padded so names sort in time order.
    let mut path = dir.join(format!("show-{stamp:020}.json"));
    let mut counter = 1;
    while path.exists() {
        path = dir.join(format!("show-{stamp:020}-{counter}.json"));
        counter += 1;
    }
    fs::write(&path, previous)?;
    let mut all = backups_newest_first(folder);
    for old in all.drain(BACKUPS_KEPT.min(all.len())..) {
        let _ = fs::remove_file(old);
    }
    Ok(())
}

fn backups_newest_first(folder: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(folder.join(BACKUPS_DIR)) else {
        return Vec::new();
    };
    let mut files: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "json"))
        .collect();
    files.sort();
    files.reverse();
    files
}

#[cfg(test)]
mod tests {
    use super::*;
    use selah_core::{Cue, CueKind, Item, ItemKind};

    fn dir(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("selah-persist-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    fn show(title: &str) -> Show {
        let mut show = Show::new(title);
        let mut item = Item::new("Worship", ItemKind::Song);
        item.cues.push(Cue::new("note", CueKind::Memo));
        show.items.push(item);
        show
    }

    #[test]
    fn a_saved_show_loads_back_identical() {
        let d = dir("roundtrip");
        let original = show("Sunday");
        save(&d, &original).unwrap();
        let loaded = load(&d).unwrap();
        assert_eq!(loaded.show, original);
        assert_eq!(loaded.recovered_from, None);
        assert!(is_show_folder(&d));
    }

    #[test]
    fn saving_leaves_no_temporary_file_behind() {
        let d = dir("tmp");
        save(&d, &show("a")).unwrap();
        assert!(!d.join("show.json.tmp").exists());
    }

    #[test]
    fn a_folder_without_a_show_says_so_plainly() {
        let d = dir("none");
        assert!(matches!(load(&d), Err(PersistError::NotAShow(_))));
    }

    #[test]
    fn each_change_keeps_the_previous_version_and_identical_saves_do_not() {
        let d = dir("backups");
        let original = show("one");
        save(&d, &original).unwrap();
        save(&d, &original).unwrap();
        assert_eq!(backups_newest_first(&d).len(), 0);
        let mut changed = original.clone();
        changed.meta.title = "two".into();
        save(&d, &changed).unwrap();
        assert_eq!(backups_newest_first(&d).len(), 1);
    }

    #[test]
    fn only_the_newest_backups_are_kept() {
        let d = dir("prune");
        for i in 0..(BACKUPS_KEPT + 8) {
            save(&d, &show(&format!("version {i}"))).unwrap();
        }
        assert_eq!(backups_newest_first(&d).len(), BACKUPS_KEPT);
    }

    #[test]
    fn a_damaged_file_is_recovered_from_the_newest_good_backup() {
        let d = dir("recover");
        save(&d, &show("first")).unwrap();
        save(&d, &show("second")).unwrap();
        fs::write(d.join(SHOW_FILE), b"{ this is not json").unwrap();

        let loaded = load(&d).unwrap();

        assert_eq!(loaded.show.meta.title, "first");
        assert!(loaded.recovered_from.is_some());
    }

    #[test]
    fn a_damaged_file_with_no_backup_is_reported_not_guessed() {
        let d = dir("corrupt");
        fs::write(d.join(SHOW_FILE), b"nonsense").unwrap();
        assert!(matches!(load(&d), Err(PersistError::Corrupt(_))));
    }

    #[test]
    fn a_newer_format_is_refused_and_never_replaced_by_a_backup() {
        let d = dir("newer");
        save(&d, &show("old")).unwrap();
        save(&d, &show("old2")).unwrap();
        fs::write(d.join(SHOW_FILE), br#"{"schemaVersion": 99}"#).unwrap();
        assert!(matches!(
            load(&d),
            Err(PersistError::TooNew { found: 99, .. })
        ));
    }

    #[test]
    fn an_unwritable_destination_is_reported() {
        let d = dir("blocked");
        let file = d.join("a-file");
        fs::write(&file, b"x").unwrap();
        assert!(matches!(
            save(&file.join("inside"), &show("x")),
            Err(PersistError::CannotWrite(_))
        ));
    }
}
