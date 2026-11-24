//! The bots as a TOML fragment: the owner's bots' roles, and every written bot's routes and routines.
use super::written::{has, key_of};
use crate::server::s;
use serde_json::{json, Map, Value};
/// An owner's bot as a role that extends the manager.
fn role_of(b: &Value) -> Value {
    let mut role = json!({"extends": "manager", "scope": "own", "name": s(b, "name"), "face": {"shape": s(b, "shape"), "color": s(b, "color")}, "prompt": ["prompts/bot.md"], "session": Some(s(b, "session")).filter(|x| !x.is_empty()).unwrap_or_else(|| "chat".into()), "access": s(b, "access")});
    let _ = (!s(b, "chat_keep").is_empty()).then(|| role["chat_keep"] = json!(s(b, "chat_keep"))); // the chat's longest life, kept as written (see chat.rs)
    super::look::face_into(&mut role, b);
    role
}
/// A routine as its route: on `schedule` with its days, its time and a `catch_up` that is not the default, or on `webhook` with its `when`.
fn routine_of(key: &str, r: &Value) -> Value {
    let mut route = json!({"on": "schedule", "to": key, "mode": "turn", "phone": true, "id": s(r, "id"), "title": s(r, "title"), "task": s(r, "prompt"), "enabled": r["on"] != false});
    if s(r, "trigger") == "webhook" {
        (route["on"], route["when"]) = (json!("webhook"), json!(s(r, "when")));
    } else {
        (route["days"], route["at"]) = (r["days"].clone(), json!(s(r, "at")));
        let _ = (r["catch_up"] == "skip").then(|| route["catch_up"] = json!("skip"));
    }
    route
}
/// A bot's routes (when it is written here), each with its `to` set to the bot, then its routines as `schedule` routes.
fn routes_of(bots: &[Value], b: &Value) -> Vec<Value> {
    let routes = b["routes"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|_| has(bots, &key_of(b), true))
        .cloned()
        .map(|mut r| (r["to"] = json!(key_of(b)), r).1);
    let routines = b["routines"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|r| routine_of(&key_of(b), r));
    routes.chain(routines).collect()
}
/// The bots' roles and routes as TOML; an empty half stays out of the fragment: a bare `routes = []` (or `roles`) would land inside the file's last section instead of at its root.
pub(super) fn fragment(bots: &[Value]) -> Result<String, String> {
    let (mut roles, mut routes) = (Map::new(), vec![]);
    for b in bots {
        let _ = (b["system"] != true).then(|| roles.insert(key_of(b), role_of(b)));
        routes.extend(routes_of(bots, b));
    }
    let parts: Map<String, Value> = [("roles", json!(roles)), ("routes", json!(routes))]
        .into_iter()
        .filter(|(_, v)| {
            !matches!(v, Value::Object(o) if o.is_empty())
                && !matches!(v, Value::Array(a) if a.is_empty())
        })
        .map(|(k, v)| (k.to_string(), v))
        .collect();
    if parts.is_empty() {
        return Ok(String::new());
    }
    toml::Value::try_from(Value::Object(parts))
        .and_then(|v| toml::to_string(&v))
        .map_err(|e| e.to_string())
}
