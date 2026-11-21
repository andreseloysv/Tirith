//! One rehearsal step: checked, handed to the extension through the relay, and kept with its result and screenshot.
use super::mask::{secret, MASK};
use super::rehearsal::push;
use super::state::now_ms;
use super::{relay, shots};
use crate::app::Shared;
use crate::server::s;
use axum::http::StatusCode;
use serde_json::{json, Value};
use std::time::Duration;
/// How long a step waits for the extension: its worker may be asleep until its next alarm (30 s).
const RELAY_S: u64 = 40;
/// The command for the extension, from the bot's `{kind, target, value}`.
fn command(b: &Value, until_ms: u64) -> Result<Value, String> {
    let (kind, target) = (s(b, "kind"), s(b, "target"));
    if !["open", "click", "type"].contains(&kind.as_str()) {
        return Err(format!("'{kind}' is not a step: open, click or type"));
    }
    if target.trim().is_empty() {
        return Err("a step needs what to open, click or type in".into());
    }
    Ok(json!({"kind": kind, "target": target, "value": s(b, "value"), "until": until_ms}))
}
/// Runs the step in the owner's Edge and keeps it: the extension's result, and where its screenshot is.
pub(super) async fn step(
    app: &Shared,
    key: &str,
    b: &Value,
    until_ms: u64,
) -> Result<Value, (StatusCode, String)> {
    let cmd = command(b, until_ms).map_err(|e| (StatusCode::BAD_REQUEST, e))?;
    let got = relay::ask(key, cmd.clone(), Duration::from_secs(RELAY_S))
        .await
        .map_err(|e| (StatusCode::GATEWAY_TIMEOUT, e))?;
    let shot = shots::save(app, key, &s(&got, "shot"));
    let hidden = secret(&json!({"label": s(&cmd, "target"), "name": s(&cmd, "target")}));
    let value = if hidden {
        MASK.into()
    } else {
        s(&cmd, "value")
    };
    let result: String = s(&got, "result").chars().take(500).collect();
    let ok = got["ok"] == true;
    let n = push(
        app,
        key,
        json!({"kind": s(&cmd, "kind"), "target": s(&cmd, "target"), "value": value, "ok": ok, "result": result, "shot": shot.as_ref().map(|s| s.0.clone()), "at": now_ms()}),
    );
    app.emit("teach", json!({"bot": key}));
    let path = shot.map(|s| s.1.display().to_string()).unwrap_or_default();
    Ok(json!({"ok": ok, "n": n, "result": result, "shot": path}))
}
