use std::path::Path;

/// Messages are written for an operator: what happened and what to do next.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PersistError {
    #[error("There is no show in {0}. Choose a folder that has a show.json file.")]
    NotAShow(String),
    #[error("Can't read {0}. Check that the folder is still there and not in use.")]
    Unreadable(String),
    #[error("The show file in {0} is damaged, and no backup could be used.")]
    Corrupt(String),
    #[error(
        "This show was saved by a newer version of Selah Stage (format {found}; this version reads up to {supported}). Update the app to open it."
    )]
    TooNew { found: u32, supported: u32 },
    #[error("Can't save to {0}. Check that the disk has room and the folder isn't read-only.")]
    CannotWrite(String),
}

impl PersistError {
    pub(crate) fn path(path: &Path) -> String {
        path.display().to_string()
    }
}
