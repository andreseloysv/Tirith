//! What the owner last wrote in each conversation (kv `owner_said:<conversation>`), kept as it comes in from the page or a paired phone and never read back from the chat, where agents write too.
use crate::app::{now_iso, App};
use crate::server::s;
use serde_json::{json, Value};

/// At most this much is kept.
const MOST: usize = 600;

fn key(conv: &str) -> String {
    format!("owner_said:{conv}")
}

/// The owner wrote `text` in conversation `conv`, now.
pub(super) fn heard(app: &App, conv: &str, text: &str) {
    let text: String = text.trim().chars().take(MOST).collect();
    if !text.is_empty() {
        let kept = json!({"at": now_iso(), "text": text});
        app.set_kv(&key(conv), Some(&kept.to_string()));
    }
}

/// The owner's last words in `conv`, when they came after `since`.
pub(super) fn after(app: &App, conv: &str, since: &str) -> Option<String> {
    let kept: Value = serde_json::from_str(&app.kv(&key(conv))?).ok()?;
    let (at, text) = (s(&kept, "at"), s(&kept, "text"));
    (at.as_str() > since && !text.is_empty()).then_some(text)
}
