//! Saving in the background after every change, without ever blocking the
//! thread that made the change.

use std::path::PathBuf;
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use selah_core::Show;

use crate::error::PersistError;
use crate::files::save;

enum Message {
    Show(Box<Show>),
    /// Save what is pending now, then reply.
    Flush(Sender<()>),
}

/// Writes the latest submitted show to its folder shortly after the last
/// change. A burst of changes becomes one save, but no change waits more than
/// `MAX_WAIT`. Dropping the saver writes anything still pending.
pub struct Autosaver {
    sender: Option<Sender<Message>>,
    thread: Option<JoinHandle<()>>,
    last_error: Arc<Mutex<Option<PersistError>>>,
}

/// The longest a change can wait, however busy the editing is.
const MAX_WAIT: Duration = Duration::from_secs(5);

impl Autosaver {
    pub fn start(folder: PathBuf, quiet_for: Duration) -> Self {
        let (sender, receiver) = mpsc::channel();
        let last_error = Arc::new(Mutex::new(None));
        let errors = last_error.clone();
        let thread = std::thread::Builder::new()
            .name("selah-autosave".into())
            .spawn(move || {
                let mut pending: Option<Box<Show>> = None;
                let mut since = Instant::now();
                let write = |show: Box<Show>| {
                    let result = save(&folder, &show).err();
                    *errors.lock().unwrap_or_else(PoisonError::into_inner) = result;
                };
                loop {
                    let message = if pending.is_some() {
                        let waited = since.elapsed();
                        let budget = quiet_for.min(MAX_WAIT.saturating_sub(waited));
                        receiver.recv_timeout(budget)
                    } else {
                        receiver.recv().map_err(|_| RecvTimeoutError::Disconnected)
                    };
                    match message {
                        Ok(Message::Show(show)) => {
                            if pending.is_none() {
                                since = Instant::now();
                            }
                            pending = Some(show);
                            if since.elapsed() >= MAX_WAIT
                                && let Some(show) = pending.take()
                            {
                                write(show);
                            }
                        }
                        Ok(Message::Flush(done)) => {
                            if let Some(show) = pending.take() {
                                write(show);
                            }
                            let _ = done.send(());
                        }
                        Err(RecvTimeoutError::Timeout) => {
                            if let Some(show) = pending.take() {
                                write(show);
                            }
                        }
                        Err(RecvTimeoutError::Disconnected) => {
                            if let Some(show) = pending.take() {
                                write(show);
                            }
                            return;
                        }
                    }
                }
            })
            .ok();
        Self {
            sender: Some(sender),
            thread,
            last_error,
        }
    }

    /// Queues the latest version. Never blocks.
    pub fn submit(&self, show: Show) {
        if let Some(sender) = &self.sender {
            let _ = sender.send(Message::Show(Box::new(show)));
        }
    }

    /// Saves anything pending and waits until it is on disk.
    pub fn flush(&self) {
        if let Some(sender) = &self.sender {
            let (done, wait) = mpsc::channel();
            if sender.send(Message::Flush(done)).is_ok() {
                let _ = wait.recv_timeout(Duration::from_secs(10));
            }
        }
    }

    /// The error from the most recent save, if it failed.
    pub fn last_error(&self) -> Option<PersistError> {
        self.last_error
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

impl Drop for Autosaver {
    fn drop(&mut self) {
        // Closing the channel makes the thread save what is pending and stop.
        self.sender = None;
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::files::load;

    fn dir(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("selah-autosave-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    #[test]
    fn a_burst_of_changes_saves_only_the_last() {
        let d = dir("burst");
        let saver = Autosaver::start(d.clone(), Duration::from_millis(60));
        for i in 0..10 {
            saver.submit(Show::new(format!("v{i}")));
        }
        saver.flush();
        assert_eq!(load(&d).unwrap().show.meta.title, "v9");
        assert_eq!(saver.last_error(), None);
    }

    #[test]
    fn saves_by_itself_after_a_quiet_moment() {
        let d = dir("quiet");
        let saver = Autosaver::start(d.clone(), Duration::from_millis(30));
        saver.submit(Show::new("auto"));
        let deadline = Instant::now() + Duration::from_secs(3);
        while !d.join("show.json").exists() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(10));
        }
        assert_eq!(load(&d).unwrap().show.meta.title, "auto");
    }

    #[test]
    fn dropping_the_saver_writes_what_is_pending() {
        let d = dir("drop");
        {
            let saver = Autosaver::start(d.clone(), Duration::from_secs(60));
            saver.submit(Show::new("last words"));
        }
        assert_eq!(load(&d).unwrap().show.meta.title, "last words");
    }

    #[test]
    fn a_failed_save_is_reported() {
        let d = dir("fail");
        std::fs::create_dir_all(&d).unwrap();
        let file = d.join("a-file");
        std::fs::write(&file, b"x").unwrap();
        let saver = Autosaver::start(file.join("inside"), Duration::from_millis(10));
        saver.submit(Show::new("x"));
        saver.flush();
        assert!(matches!(
            saver.last_error(),
            Some(PersistError::CannotWrite(_))
        ));
    }
}
