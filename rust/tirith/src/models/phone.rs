//! A paired phone's model requests.
use super::{save::save, state, validate::token};
use crate::app::Shared;
use serde_json::{json, Value};
/// A paired phone's encrypted control request. Repeated save IDs cannot reapply old settings.
pub fn phone_response(app: &Shared, peer: &str, b: &Value) -> Value {
    let id = b["id"].as_str().unwrap_or("");
    let key = format!("models.rpc:{peer}:{id}");
    let result = if b["v"] != 1 || id.is_empty() || id.len() > 80 || !token(id) {
        Err("invalid model request".to_string())
    } else if let Some(cached) = app
        .kv(&key)
        .and_then(|s| serde_json::from_str::<Result<(), String>>(&s).ok())
    {
        cached
    } else {
        let result = run(app, b);
        let _ = (b["action"] != "get").then(|| app.set_kv(&key, Some(&json!(result).to_string())));
        result
    };
    json!({"v": 1, "id": id, "ok": result.is_ok(), "error": result.err(), "state": state(app)})
}
/// Runs the request's action: get, set or reset its role.
fn run(app: &Shared, b: &Value) -> Result<(), String> {
    let role = b["role"].as_str().unwrap_or("");
    match b["action"].as_str() {
        Some("get") => Ok(()),
        Some("set") if b["setting"].is_object() => save(app, role, Some(&b["setting"])).map(|_| ()),
        Some("reset") => save(app, role, None).map(|_| ()),
        _ => Err("unknown model action".to_string()),
    }
}
