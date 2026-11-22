//! A note for a conversation's next message (`deliver::noted`) outlives a restart: it is in the store until a message hands it over.
use crate::app::Shared;

const KEY: &str = "noted:";

/// `text` waits for the next message of conversation `conv`.
pub(super) fn keep(app: &Shared, conv: &str, text: &str) {
    let key = format!("{KEY}{conv}");
    let mut all = read(app.kv(&key).as_deref().unwrap_or(""));
    all.push(text.to_string());
    let cut = all.len().saturating_sub(super::queue::KEPT);
    all.drain(..cut);
    app.set_kv(&key, Some(&serde_json::json!(all).to_string()));
}

fn read(kept: &str) -> Vec<String> {
    serde_json::from_str(kept).unwrap_or_default()
}

/// A message of conversation `conv` took its notes.
pub(super) fn handed(app: &Shared, conv: &str) {
    let key = format!("{KEY}{conv}");
    let _ = app.kv(&key).is_some().then(|| app.set_kv(&key, None));
}

/// At the daemon's start: each waits again for its conversation's next message.
pub fn start(app: Shared) {
    let sql = "SELECT key, value FROM kv WHERE key LIKE ?1";
    let all: Vec<(String, String)> =
        app.query(sql, [format!("{KEY}%")], |r| Ok((r.get(0)?, r.get(1)?)));
    for (key, kept) in all {
        let conv = key.trim_start_matches(KEY);
        for text in read(&kept) {
            super::queue::queue(&app, conv, &text, "note", false);
        }
    }
}
