//! The lab's commands and routes: `tirith lab start` and `tirith lab list` (the Coach's), POST /api/lab/start and GET /api/lab?bot=<id or key> (the bot's «Mejoras» tab).
use crate::app::Shared;
use crate::client::{call, call_json, Flag};
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::{json, Value};
use std::collections::HashMap;

const START: &str =
    "use: tirith lab start <bot> <branch> --routine <id> --mode live --hypothesis \"<text>\"";

/// `tirith lab start <bot> <branch> --routine <id> --mode live --hypothesis "<text>"` and `tirith lab list [<bot>]`.
pub fn cli(pos: &[&str], flag: Flag) -> Option<Result<(), String>> {
    let body = |bot: &str, branch: &str| json!({"bot": bot, "branch": branch, "routine": flag("--routine"), "mode": flag("--mode"), "hypothesis": flag("--hypothesis")});
    match pos {
        ["lab", "start", bot, branch] => Some(started(call_json(
            "POST",
            "/api/lab/start",
            Some(body(bot, branch)),
        ))),
        ["lab", "list"] => Some(call("GET", "/api/lab", None).map(|b| println!("{b}"))),
        ["lab", "list", bot] => {
            Some(call("GET", &format!("/api/lab?bot={bot}"), None).map(|b| println!("{b}")))
        }
        ["lab", ..] => Some(Err(format!("{START}\n   or: tirith lab list [<bot>]"))),
        _ => None,
    }
}

fn started(r: Result<Value, String>) -> Result<(), String> {
    let v = r?;
    let id = v["id"].as_i64().unwrap_or(0);
    println!("experiment {id} runs on routine {} of {}: «{}»; its runs alternate the bot's main and {} (in {})", v["routine"], v["bot"], v["change"].as_str().unwrap_or(""), v["branch"], v["tree"]);
    Ok(())
}

async fn post_start(State(app): State<Shared>, Json(b): Json<Value>) -> (StatusCode, Json<Value>) {
    match super::start::start(&app, &b) {
        Ok(v) => (StatusCode::OK, Json(v)),
        Err(e) => (StatusCode::BAD_REQUEST, Json(json!({"error": e}))),
    }
}

async fn get_list(
    State(app): State<Shared>,
    Query(q): Query<HashMap<String, String>>,
) -> Json<Value> {
    let who = q.get("bot").cloned().unwrap_or_default();
    let key = crate::bots::key_for(&app, &who).unwrap_or(who);
    Json(json!({"experiments": super::store::list(&app, &key), "rule": rule(&app)}))
}

/// The rule the page shows next to the experiments.
fn rule(app: &Shared) -> Value {
    let l = app.cfg().factory.lab.clone();
    json!({"inputs": l.inputs, "not_worse_share": l.not_worse_share, "min_saving": l.min_saving, "max_days": l.max_days})
}

pub fn routes() -> Router<Shared> {
    Router::new()
        .route("/api/lab/start", post(post_start))
        .route("/api/lab", get(get_list))
}
