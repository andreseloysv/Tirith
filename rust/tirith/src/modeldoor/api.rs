//! `/door/…` (the door itself), `GET /api/door` (the owner's: what the day has spent and the last calls) and `neobot door`.
use super::{on, record};
use crate::app::Shared;
use crate::client::{call_json, Flag};
use axum::{extract::State, routing::get, Json, Router};
use serde_json::{json, Value};

pub fn routes() -> Router<Shared> {
    super::route::routes().route("/api/door", get(status))
}

async fn status(State(app): State<Shared>) -> Json<Value> {
    Json(
        json!({"enabled": on::enabled(&app), "budget_usd": on::machine(&app), "spent_today_usd": record::spent_today(&app, None),
        "calls": record::recent(&app, 50)}),
    )
}

/// `neobot door`: whether it is on, the day's spend and the last calls.
pub fn cli(pos: &[&str], _flag: Flag) -> Option<Result<(), String>> {
    (pos == ["door"]).then(|| {
        let v = call_json("GET", "/api/door", None)?;
        let state = if v["enabled"] == true { "on" } else { "off" };
        let budget = v["budget_usd"]
            .as_f64()
            .map(|b| format!(" of ${b:.2}"))
            .unwrap_or_default();
        println!(
            "model door: {state}; spent today ${:.4}{budget}",
            v["spent_today_usd"].as_f64().unwrap_or(0.0)
        );
        for c in v["calls"].as_array().into_iter().flatten().rev().take(10) {
            let cost = c["cost_usd"]
                .as_f64()
                .map_or("no price".into(), |c| format!("${c:.4}"));
            println!(
                "{} {} {} {} {} {cost}",
                c["at"].as_str().unwrap_or(""),
                c["status"],
                c["who"].as_str().unwrap_or(""),
                c["model"].as_str().unwrap_or(""),
                c["tokens_out"]
            );
        }
        Ok(())
    })
}
