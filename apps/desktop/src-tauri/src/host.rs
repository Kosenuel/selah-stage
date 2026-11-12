//! The audio and cue host, owned by the app for its whole life.

use selah_host::{Host, Output};

use crate::error::AppError;

/// Holds the host, or the reason it could not start.
pub struct HostState(Result<Host, String>);

impl HostState {
    /// Starts the host on the default audio output. If that fails the app
    /// still opens and says so, rather than refusing to start.
    pub fn start() -> Self {
        Self(Host::spawn(Output::Default).map_err(|error| {
            tracing::error!(%error, "audio host failed to start");
            error.to_string()
        }))
    }

    pub fn get(&self) -> Result<&Host, AppError> {
        self.0
            .as_ref()
            .map_err(|reason| AppError::internal(format!("{reason} Restart Selah Stage.")))
    }
}
