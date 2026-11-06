//! Chat attachments: files and images the owner drops or pastes into the chat (STUB: the misc agent replaces
//! the bodies, keeping every signature).

use crate::app::Shared;
use axum::Router;

/// The text appended to the owner's message so the manager can open the attached files (absolute paths).
pub fn describe(paths: &[String]) -> String {
    if paths.is_empty() {
        return String::new();
    }
    let list: Vec<String> = paths.iter().map(|p| format!("- {p}")).collect();
    format!("\n\nAttached files (open them with your file-reading tool):\n{}", list.join("\n"))
}

pub fn start(_app: Shared) {}

pub fn routes() -> Router<Shared> {
    Router::new()
}

pub fn cli(_pos: &[&str], _flag: crate::client::Flag) -> Option<Result<(), String>> {
    None
}
