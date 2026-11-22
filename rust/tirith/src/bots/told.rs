//! What a bot is handed when one of its routines is due.
use crate::server::s;
use serde_json::Value;

/// The turn's text: the routine, its delay (`late`) and how to report.
pub(super) fn told(bot: &Value, title: &str, task: &str, late: &str) -> String {
    let its = if s(bot, "session") == "turn" {
        " It is a task, not a review."
    } else {
        ""
    };
    format!("Routine «{title}» is due now{late}.{its} Do it and report the result here in a few lines ({}); ask nothing: {task}", super::REPORT)
}
