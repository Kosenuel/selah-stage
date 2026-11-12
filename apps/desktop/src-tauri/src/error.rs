//! The one error type that crosses the IPC boundary.

use serde::Serialize;
use specta::Type;

/// What every command returns on failure.
///
/// Crates under `crates/` define their own `thiserror` types; command handlers
/// map those into an `AppError` so the UI handles a single, stable shape.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type, thiserror::Error)]
#[serde(rename_all = "camelCase")]
#[error("{message}")]
pub struct AppError {
    /// Lets the UI choose how to react without parsing the message.
    pub kind: AppErrorKind,
    /// Plain language that is safe to show to an operator: what happened and
    /// what to do next.
    pub message: String,
}

/// Broad error categories. Add a variant when the UI needs to react
/// differently to a new class of failure, not for every new error.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum AppErrorKind {
    /// Something went wrong inside the app that the operator cannot fix.
    Internal,
}

impl AppError {
    pub fn internal(message: impl Into<String>) -> Self {
        Self {
            kind: AppErrorKind::Internal,
            message: message.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The UI depends on this exact JSON shape; changing it is an IPC change.
    #[test]
    fn serialises_to_the_shape_the_ui_expects() {
        let json = serde_json::to_value(AppError::internal("Something went wrong.")).unwrap();
        assert_eq!(
            json,
            serde_json::json!({ "kind": "internal", "message": "Something went wrong." })
        );
    }
}
