//! One rehearsal step: checked (a click that sends, buys, pays, deletes or publishes, or that the skill lists as needing the owner's yes, is refused here), handed to the extension through the relay, and kept with its result and screenshot.
use super::approvals::listed;
use super::mask::{secret, MASK};
use super::outward::{sends, STOP};
use super::rehearsal::{last, push};
use super::state::now_ms;
use super::{approve, relay, shots};
use crate::app::Shared;
use crate::server::s;
use axum::http::StatusCode;
use serde_json::{json, Value};
use std::time::Duration;
/// How long a step waits for the extension: its worker may be asleep until its next alarm (30 s).
const RELAY_S: u64 = 40;
/// The command for the extension, from the bot's `{kind, target, value}`; `left_ms` is how long the rehearsal has left.
fn command(b: &Value, until_ms: u64) -> Result<Value, String> {
    let (kind, target) = (s(b, "kind"), s(b, "target"));
    if !["open", "click", "type"].contains(&kind.as_str()) {
        return Err(format!("'{kind}' is not a step: open, click or type"));
    }
    if target.trim().is_empty() {
        return Err("a step needs what to open, click or type in".into());
    }
    let left = until_ms.saturating_sub(crate::app::now_s() * 1000);
    Ok(
        json!({"kind": kind, "target": target, "value": s(b, "value"), "until": until_ms, "left_ms": left}),
    )
}
/// Why a click is not run by code: its words send, buy, pay, delete or publish, or the skill lists it under what needs the owner's yes.
fn why(app: &Shared, key: &str, cmd: &Value) -> Option<String> {
    let target = s(cmd, "target");
    (cmd["kind"] == "click").then_some(())?;
    if sends(&target) {
        return Some(format!(
            "«{target}» sends, buys, pays, deletes or publishes"
        ));
    }
    let skill = approve::text_of(app, key, &s(&last(app, key)?, "skill"))?;
    listed(&skill, &target)
        .map(|l| format!("the skill lists «{target}» under what needs the owner's yes ({l})"))
}
/// The step kept with what it did, and what the bot reads back.
fn kept(app: &Shared, key: &str, cmd: &Value, got: &Value, ok: bool) -> Value {
    let shot = shots::save(app, key, &s(got, "shot"));
    let hidden = secret(&json!({"label": s(cmd, "target"), "name": s(cmd, "target")}));
    let value = if hidden { MASK.into() } else { s(cmd, "value") };
    let result: String = s(got, "result").chars().take(500).collect();
    let n = push(
        app,
        key,
        json!({"kind": s(cmd, "kind"), "target": s(cmd, "target"), "value": value, "ok": ok, "result": result, "shot": shot.as_ref().map(|s| s.0.clone()), "at": now_ms()}),
    );
    app.emit("teach", json!({"bot": key}));
    let path = shot.map(|s| s.1.display().to_string()).unwrap_or_default();
    json!({"ok": ok, "n": n, "result": result, "shot": path})
}
/// A step refused for `why`: kept as not done, with the result that tells the bot to stop there.
fn refused(app: &Shared, key: &str, cmd: &Value, why: &str) -> Value {
    let result = format!("Refused: {why}. It needs the owner's yes. {STOP}");
    kept(app, key, cmd, &json!({"result": result}), false)
}
/// Runs the step in the owner's Edge and keeps it: the extension's result, and where its screenshot is.
pub(super) async fn step(
    app: &Shared,
    key: &str,
    b: &Value,
    until_ms: u64,
) -> Result<Value, (StatusCode, String)> {
    let cmd = command(b, until_ms).map_err(|e| (StatusCode::BAD_REQUEST, e))?;
    let own =
        cmd["kind"] == "open" && super::own_page::own_page(&s(&cmd, "target"), app.cfg().port);
    let own =
        own.then(|| "that is OpenBot's own page, where a click is the owner's yes".to_string());
    if let Some(why) = own.or_else(|| why(app, key, &cmd)) {
        return Ok(refused(app, key, &cmd, &why));
    }
    let got = relay::ask(key, cmd.clone(), Duration::from_secs(RELAY_S))
        .await
        .map_err(|e| (StatusCode::GATEWAY_TIMEOUT, e))?;
    if got["refused"] == true {
        return Ok(refused(app, key, &cmd, &s(&got, "result")));
    }
    Ok(kept(app, key, &cmd, &got, got["ok"] == true))
}
