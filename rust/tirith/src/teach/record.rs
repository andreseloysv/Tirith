//! A recording started and stopped: by the page, the extension, or its 10 minutes running out; at stop the bot is handed it.
use super::hand::over;
use super::state::{change, now_ms, MOST_S};
use crate::app::{now_iso, now_s, Shared};
use serde_json::{json, Value};
/// Stops the recording `since` (any, when empty) and hands it over; false when none was on.
pub(super) fn stop_now(app: &Shared, key: &str, since: &str) -> bool {
    let rec = change(app, key, |rec| {
        let on = rec["on"] == true && (since.is_empty() || rec["since"] == since);
        rec["on"] = json!(false);
        on.then(|| rec.clone())
    });
    let _ = rec.as_ref().map(|r| over(app, key, r));
    app.emit("teach", json!({"bot": key}));
    rec.is_some()
}
pub(super) fn start_now(app: &Shared, key: &str) -> Value {
    let since = now_ms();
    change(
        app,
        key,
        |rec| *rec = json!({"on": true, "since": since, "since_s": now_s(), "steps": []}),
    );
    app.chat("tirith", "Recording in Edge: do the task once; what you write here meanwhile goes with the steps. It stops by itself in 10 minutes.", key);
    app.emit("teach", json!({"bot": key}));
    let (app, key2) = (app.clone(), key.to_string());
    let since2 = since.clone();
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(MOST_S)).await;
        stop_now(&app, &key2, &since2);
    });
    json!({"ok": true, "on": true, "since": since, "at": now_iso()})
}
