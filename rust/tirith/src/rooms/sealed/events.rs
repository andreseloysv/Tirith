//! Posting signed events, registering this OpenBot's key when the platform does not know it yet.
use super::{http::api, signed::event_id, text::canonical, text::sha_hex, Me};
use serde_json::{json, Value};
async fn register(base: &str, me: &Me) -> Result<(), String> {
    let card = json!({"name": me.name, "capabilities": ["am.mjs"], "tags": []});
    let ts = chrono::Utc::now().timestamp_millis() as u64;
    let sig = me.sign(&sha_hex(
        canonical(&json!({"card": card, "pubkey": me.public, "ts": ts})).as_bytes(),
    ));
    api(
        base,
        "/register",
        Some(&json!({"pubkey": me.public, "card": card, "ts": ts, "sig": sig})),
        30,
    )
    .await
    .map(|_| ())
    .map_err(|e| e.1)
}
/// One event of `kind` in `room`, signed by `me`, with no refs.
fn signed(me: &Me, kind: &str, room: &str, body: Value) -> Value {
    let mut e = json!({"author": me.public, "kind": kind, "tag": format!("#{room}"), "refs": [], "body": body, "ts": chrono::Utc::now().timestamp_millis() as u64});
    e["id"] = json!(event_id(&e));
    e["sig"] = json!(me.sign(e["id"].as_str().unwrap_or("")));
    e
}
/// A signed `request` event in `room` whose body is the JSON text of `body`, sent to `path` (as `/hooks`); the platform's answer. Checked there and never stored.
pub async fn request(
    base: &str,
    me: &Me,
    room: &str,
    path: &str,
    body: &Value,
) -> Result<Value, String> {
    let e = signed(me, "request", room, json!(body.to_string()));
    api(base, path, Some(&e), 60).await.map_err(|e| e.1)
}
/// Posts one signed event in `room` (registering the key when the platform does not know it yet).
pub async fn post(base: &str, me: &Me, kind: &str, room: &str, body: Value) -> Result<(), String> {
    let e = signed(me, kind, room, body);
    match api(base, "/events", Some(&e), 60).await {
        Err((403, _)) => {
            register(base, me).await?;
            api(base, "/events", Some(&e), 60)
                .await
                .map(|_| ())
                .map_err(|e| e.1)
        }
        Err((409, _)) if kind == "announce" => Ok(()), // already said "joined"
        r => r.map(|_| ()).map_err(|e| e.1),
    }
}
