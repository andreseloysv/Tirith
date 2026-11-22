//! A recording started and stopped: by the page, the extension, or its 10 minutes running out (also across a restart of the daemon); at stop the bot is handed it, once.
use super::hand::over;
use super::state::{change, left_ms, live, now_ms, read, MOST_S};
use crate::app::{now_iso, now_s, Shared};
use crate::server::s;
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
/// Stops the recording `since` after `secs` seconds (a restart of the daemon loses this; `start` arms it again).
fn arm(app: &Shared, key: &str, since: String, secs: u64) {
    let (app, key) = (app.clone(), key.to_string());
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(secs)).await;
        stop_now(&app, &key, &since);
    });
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
    arm(app, key, since.clone(), MOST_S);
    json!({"ok": true, "on": true, "since": since, "at": now_iso(), "left_ms": MOST_S * 1000})
}
/// After a start of the daemon: a recording whose 10 minutes ran out meanwhile is stopped and handed over (once: the stop takes it off), one still running is armed for what it has left.
pub(super) fn resume(app: &Shared) {
    for b in crate::bots::all(app) {
        let key = s(&b, "key");
        let rec = read(app, &key);
        if rec["on"] != true {
            continue;
        }
        if live(&rec) {
            arm(app, &key, s(&rec, "since"), left_ms(&rec) / 1000 + 1);
        } else {
            stop_now(app, &key, &s(&rec, "since"));
        }
    }
}
/// At start: recordings left on by a daemon that stopped.
pub fn start(app: Shared) {
    tokio::spawn(async move { resume(&app) });
}
