//! The bots as the page sees them, read from the effective factory table, and how one is found.
use super::schedule::days;
use crate::app::App;
use crate::server::s;
use serde_json::{json, Value};
/// `x`, or `d` when it is empty.
fn or(x: String, d: &str) -> String {
    Some(x)
        .filter(|x| !x.is_empty())
        .unwrap_or_else(|| d.to_string())
}
/// A route that is one of a bot's routines: on `schedule` (a time) or on `webhook` (an event from outside).
pub(super) fn routine_on(r: &Value) -> bool {
    matches!(s(r, "on").as_str(), "schedule" | "webhook")
}
/// The bot's `schedule` and `webhook` routes as routines (`last` says a routine's last run).
fn routines_of(
    routes: &[Value],
    key: &str,
    id: &str,
    last: &dyn Fn(&str, &str) -> String,
) -> Vec<Value> {
    routes
        .iter()
        .filter(|r| routine_on(r) && s(r, "to") == key && !s(r, "task").is_empty())
        .enumerate()
        .map(|(i, r)| {
            let rid = or(s(r, "id"), &format!("r{i}"));
            json!({"id": rid.clone(), "title": or(s(r, "title"), &s(r, "task")), "prompt": s(r, "task"), "trigger": s(r, "on"), "when": s(r, "when"), "days": days(&r["days"]).unwrap_or_default(), "at": s(r, "at"), "catch_up": or(s(r, "catch_up"), "once"), "on": r["enabled"] != false, "last": last(id, &rid)})
        })
        .collect()
}
/// The bot's other routes, each as written.
fn hears_of(routes: &[Value], key: &str) -> Vec<Value> {
    routes
        .iter()
        .filter(|r| !routine_on(r) && s(r, "to") == key)
        .cloned()
        .collect()
}
/// One role as a bot, if its `scope` is "own".
fn bot_of(
    routes: &[Value],
    key: &str,
    role: &Value,
    last: &dyn Fn(&str, &str) -> String,
) -> Option<Value> {
    let id = (s(role, "scope") == "own").then(|| key.strip_prefix("bot-").unwrap_or(key))?;
    let routines = routines_of(routes, key, id, last);
    let hears = hears_of(routes, key);
    Some(
        json!({"id": id, "key": key, "system": !key.starts_with("bot-"), "show": role["show"] != false, "name": or(s(role, "name"), id), "shape": or(s(&role["face"], "shape"), "round"), "color": or(s(&role["face"], "color"), "#3b82f6"), "session": or(s(role, "session"), "chat"), "access": s(role, "access"), "chat_keep": s(role, "chat_keep"), "routes": hears, "routines": routines}),
    )
}
/// The bots as the page sees them: every role of `raw` with `scope = "own"` (an owner's `bot-<id>`, in config.toml, or a system bot by its key; its name, face and `show`), with its `schedule` routes as routines (`last` says a routine's last run), its other routes as `routes` (each as written; writing one back sets its `to` to this bot), and its `session` and `access`: the whole configuration its card shows and changes. `key` is its conversation; `id` is its page id (a system bot's key).
pub(super) fn bots_of(raw: &Value, last: &dyn Fn(&str, &str) -> String) -> Vec<Value> {
    // `all` reads them from the effective factory table: the owner's bots through config.toml's overlay, the system bots as factory roles
    let routes = raw["routes"].as_array().cloned().unwrap_or_default();
    raw["roles"]
        .as_object()
        .into_iter()
        .flatten()
        .filter_map(|(key, role)| bot_of(&routes, key, role, last))
        .collect()
}
pub fn all(app: &App) -> Vec<Value> {
    bots_of(
        &serde_json::to_value(&app.cfg().factory.raw).unwrap_or_default(),
        &|id, rid| app.kv(&format!("routine:{id}:{rid}")).unwrap_or_default(),
    )
}
/// The bot whose conversation is `key` (`bot-<id>`, or a system bot's key).
pub fn bot(app: &App, key: &str) -> Option<Value> {
    all(app).into_iter().find(|b| b["key"] == key)
}
pub fn key_for(app: &App, id: &str) -> Option<String> {
    all(app)
        .into_iter()
        .find(|b| b["id"] == id)
        .map(|b| s(&b, "key"))
}
