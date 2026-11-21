//! Clearing a conversation so its bot starts from zero.
use crate::app::{now_iso, Shared};
use serde_json::{json, Value};

/// Forgets a conversation's saved agent sessions, one per agent alias, so its next session starts fresh instead of resuming what is gone.
pub(super) fn forget_session(app: &Shared, conv: &str) {
    let end = format!(":{conv}");
    app.query(
        "SELECT key FROM kv WHERE key LIKE 'manager_session:%'",
        [],
        |r| r.get::<_, String>(0),
    )
    .iter()
    .filter(|k| k.ends_with(&end))
    .for_each(|k| app.set_kv(k, None));
}

/// One sealed line to the paired rooms of a conversation, for the phone page to hide what is gone.
pub(super) fn to_phones(app: &Shared, conv: &str, body: Value) {
    crate::rooms::to_phones(app, conv, &format!("tirith-chat {body}"));
}

/// Clears one conversation: its rows go, its saved session is forgotten and its live one ends without cutting a running turn, so the bot starts from zero; its memory, skills and routines stay.
pub(super) fn clear(app: &Shared, project: &str) -> Result<Value, String> {
    (project.is_empty() || app.conv(project) == project)
        .then_some(())
        .ok_or(format!("unknown project '{project}'"))?;
    let conv = app.conv(project);
    let _ = app
        .db
        .lock()
        .unwrap()
        .execute("DELETE FROM chat WHERE project = ?1", [conv]);
    forget_session(app, conv);
    crate::manager::end_session(conv);
    app.emit("chat-clear", json!({"project": conv}));
    to_phones(app, conv, json!({"v": 1, "clear": now_iso()}));
    Ok(json!({"ok": true, "project": conv}))
}
