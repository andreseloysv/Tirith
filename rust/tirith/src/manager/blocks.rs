//! An assistant message of a stream-json session: its text and its tool calls, in the chat and on the phones.
use super::progress::progress_line;
use super::talk::talk;
use super::typing::typing_line;
use crate::app::Shared;
use crate::{rooms, rules};
use serde_json::Value;
pub(super) fn assistant(app: &Shared, project: &str, msg: &Value) {
    msg["message"]["content"]
        .as_array()
        .into_iter()
        .flatten()
        .for_each(|block| match block["type"].as_str() {
            Some("text") => text(app, project, block["text"].as_str().unwrap_or("")),
            Some("tool_use") => tool_use(app, project, block),
            _ => {}
        })
}
fn text(app: &Shared, project: &str, text: &str) {
    app.chat("manager", text, project);
    let _ = talk(project, |t| t.phones).then(|| rooms::to_phones(app, project, text)); // on the phones only when this turn is marked (owner, or phone = true)
    talk(project, |t| {
        t.text = format!("{}\n\n{text}", t.text).trim_start().to_string()
    });
}
fn tool_use(app: &Shared, project: &str, block: &Value) {
    app.chat(
        "tool",
        &rules::summary(block["name"].as_str().unwrap_or(""), &block["input"]),
        project,
    );
    typing_line(
        project,
        &progress_line(block["name"].as_str().unwrap_or(""), &block["input"]),
    );
}
