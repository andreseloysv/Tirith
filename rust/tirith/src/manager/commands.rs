//! The owner's commands that Tirith answers itself, with no turn: `/status`, `/models`, `/model`.
use super::deliver::report;
use crate::app::Shared;
use crate::{models, status};

/// Only the local owner chat and authenticated paired phones call this path.
pub fn owner_command(app: &Shared, project: &str, text: &str) -> bool {
    let answer = status::command(app, text).or_else(|| {
        models::command(app, text)
            .map(|r| r.unwrap_or_else(|e| format!("Model change failed: {e}")))
    });
    let Some(answer) = answer else {
        return false;
    };
    report(app, project, &answer);
    true
}
