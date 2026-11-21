//! Dropping the chat rows older than their conversation's longest life.
use super::{clear::forget_session, keep::keep_s};
use crate::app::{now_s, Shared};
use serde_json::json;
use std::time::Duration;

/// Every conversation with a longest life loses its rows older than it (their `at` is ISO UTC, so text order is time order); a conversation that lost any forgets its saved session too, so its bot does not remember what is gone. `now` is injected for the test.
pub(super) fn sweep(app: &Shared, now: u64) {
    for conv in app.query("SELECT DISTINCT project FROM chat", [], |r| {
        r.get::<_, String>(0)
    }) {
        let Some(secs) = keep_s(app, &conv) else {
            continue;
        };
        let Some(cut) = chrono::DateTime::from_timestamp(now.saturating_sub(secs) as i64, 0) else {
            continue;
        };
        trim(app, &conv, &cut.format("%Y-%m-%dT%H:%M:%SZ").to_string());
    }
}

/// Drops one conversation's rows from `before` back; when any went, its saved session is forgotten and the page hears it.
fn trim(app: &Shared, conv: &str, before: &str) {
    let gone = app
        .db
        .lock()
        .unwrap()
        .execute(
            "DELETE FROM chat WHERE project = ?1 AND at < ?2",
            rusqlite::params![conv, before],
        )
        .unwrap_or(0);
    let _ = (gone > 0).then(|| {
        (
            forget_session(app, conv),
            app.emit("chat-trim", json!({"project": conv, "before": before})),
        )
    });
}

/// The minute sweep of every conversation's longest life.
pub fn start(app: Shared) {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(60)).await;
            sweep(&app, now_s());
        }
    });
}
