//! A bot's own validation: `neobot turn checks <passed>/<total> "<names>"`, kept on the open turn of its conversation, or on the last one when it just closed.
use super::live::with;
use super::signals::last;
use crate::app::Shared;
use crate::client::{call_json, Flag};
use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::post;
use axum::{Json, Router};
use rusqlite::params;
use serde_json::{json, Value};

/// `neobot turn checks <passed>/<total> ["<names>"]`, on the conversation of the session that runs it (`TIRITH_PROJECT`). A lane has no turn of its own.
pub fn cli(pos: &[&str], flag: Flag) -> Option<Result<(), String>> {
    let (count, names) = match pos {
        ["turn", "checks", count] => (*count, ""),
        ["turn", "checks", count, names] => (*count, *names),
        ["turn", ..] => {
            return Some(Err(
                "use: neobot turn checks <passed>/<total> \"<names>\"".into()
            ))
        }
        _ => return None,
    };
    let lane = std::env::var("TIRITH_LANE").is_ok_and(|l| !l.is_empty());
    let conv = flag("--project").or_else(|| std::env::var("TIRITH_PROJECT").ok());
    let body = match (lane, conv) {
        (true, _) => Err("a lane has no turn: neobot turn checks is for a bot's session".into()),
        (_, None) => Err("no conversation: run it in a bot's session (TIRITH_PROJECT)".into()),
        (_, Some(conv)) => request(count, names, &conv),
    };
    let sent = body.and_then(|b| call_json("POST", "/api/turns/checks", Some(b)));
    let on = |v: Value| {
        Some(v["turn"].clone())
            .filter(|t| t != 0)
            .map_or("the turn running now".into(), |t| format!("turn {t}"))
    };
    Some(sent.map(|v| println!("checks kept on {}", on(v))))
}

/// The request of `neobot turn checks`: `3/4` read as passed and total.
pub fn request(count: &str, names: &str, conv: &str) -> Result<Value, String> {
    let bad = || format!("'{count}' is not <passed>/<total>, as 3/4");
    let (passed, total) = count.split_once('/').ok_or_else(bad)?;
    let n = |s: &str| s.trim().parse::<u32>().map_err(|_| bad());
    let (passed, total) = (n(passed)?, n(total)?);
    let Some(()) = (passed <= total).then_some(()) else {
        return Err(bad());
    };
    Ok(json!({"conv": conv, "passed": passed, "total": total, "names": names.trim()}))
}

/// Keeps the checks on the open turn of `conv` (its row gets them at its end), else on its last kept turn: that turn's id (0 while still open).
pub fn keep_checks(app: &Shared, conv: &str, checks: (i64, i64, String)) -> Result<i64, String> {
    let open = with(conv, |c| {
        c.open.as_mut().map(|t| t.checks = Some(checks.clone()))
    });
    let Some(()) = open.is_none().then_some(()) else {
        return Ok(0);
    };
    let id = last(app, conv).ok_or(format!("no turn in '{conv}' yet"))?;
    let (passed, total, names) = checks;
    app.insert(
        "UPDATE turns SET checks_passed = ?1, checks_total = ?2, checks = ?3 WHERE id = ?4",
        params![passed, total, names, id],
    );
    Ok(id)
}

async fn post_checks(
    State(app): State<Shared>,
    who: crate::server::Who,
    Json(b): Json<Value>,
) -> (StatusCode, Json<Value>) {
    let conv = app.conv(b["conv"].as_str().unwrap_or("")).to_string();
    if who.agent().is_some_and(|a| app.conv(&a.project) != conv) {
        let e = json!({"error": "a bot keeps checks on its own conversation's turns"});
        return (StatusCode::FORBIDDEN, Json(e));
    }
    let (passed, total) = (b["passed"].as_i64(), b["total"].as_i64());
    let names = b["names"].as_str().unwrap_or("").to_string();
    let kept = match (passed, total) {
        (Some(p), Some(t)) if p <= t && p >= 0 => keep_checks(&app, &conv, (p, t, names)),
        _ => Err("passed and total are counts, passed no more than total".into()),
    };
    match kept {
        Ok(turn) => (StatusCode::OK, Json(json!({"conv": conv, "turn": turn}))),
        Err(e) => (StatusCode::BAD_REQUEST, Json(json!({"error": e}))),
    }
}

pub fn routes() -> Router<Shared> {
    Router::new().route("/api/turns/checks", post(post_checks))
}
