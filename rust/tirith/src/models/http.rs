//! The daemon's /api/models routes and the `tirith models` command that calls them.
use super::{roles::RoleSetting, save::line, save::save, state};
use crate::app::{bad, Shared};
use crate::client::{call, Flag};
use axum::extract::{Path as UrlPath, State};
use axum::{routing::get as GET, Json, Router};
use serde_json::{json, Map, Value};
pub fn routes() -> Router<Shared> {
    Router::new().route("/api/models", GET(|State(app): State<Shared>| async move { Json(state(&app)) })).route(
        "/api/models/{role}",
        axum::routing::post(|State(app): State<Shared>, UrlPath(r): UrlPath<String>, Json(b): Json<Value>| async move { save(&app, &r, Some(&b)).map(|s| Json(json!(s))).map_err(bad) })
            .delete(|State(app): State<Shared>, UrlPath(r): UrlPath<String>| async move { save(&app, &r, None).map(|s| Json(json!(s))).map_err(bad) }),
    )
}
/// `tirith models`, `tirith models set <role> [--agent A] [--model M] [--effort E] [--fallback "a:m,b"]`, `tirith models reset <role>`; `None` when `pos` is not one of this module's commands.
pub fn cli(pos: &[&str], flag: Flag) -> Option<Result<(), String>> {
    Some(match pos {
        ["models"] => call("GET", "/api/models", None)
            .and_then(|b| serde_json::from_str::<Value>(&b).map_err(|e| e.to_string()))
            .map(|v| print_roles(&v)),
        ["models", "set", r] => {
            let body: Map<String, Value> = ["agent", "model", "effort", "fallback"]
                .iter()
                .filter_map(|k| flag(&format!("--{k}")).map(|v| (k.to_string(), json!(v))))
                .collect();
            show(call(
                "POST",
                &format!("/api/models/{r}"),
                Some(Value::Object(body)),
            ))
        }
        ["models", "reset", r] => show(call("DELETE", &format!("/api/models/{r}"), None)),
        _ => return None,
    })
}
/// One line per role: its setting and its source.
fn print_roles(v: &Value) {
    v["roles"]
        .as_object()
        .into_iter()
        .flatten()
        .for_each(|(r, rv)| {
            println!(
                "{r:<10} {}  [{}]",
                line(&serde_json::from_value(rv.clone()).unwrap_or_default()),
                rv["source"].as_str().unwrap_or("")
            )
        })
}
/// Prints the setting the daemon answered with.
fn show(body: Result<String, String>) -> Result<(), String> {
    serde_json::from_str::<RoleSetting>(&body?)
        .map_err(|e| e.to_string())
        .map(|s| println!("{}", line(&s)))
}
