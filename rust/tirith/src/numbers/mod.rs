//! A bot's numbers, read from its rows in `turns` and its memory: `GET /api/bots/<id>/numbers` (docs/DAEMON.md). By routine: its runs, success rate, mean time, cost per good run, 👎 and corrections per 10, mean judge's score and the trend (the last 10 runs against the 10 before). By skill: its uses, its validated uses and its routine's tokens before its first use against after. Its memory: the tokens loading it costs, the notes its last 30 runs cited, the ones none cited and the memory misses.
mod memory;
mod routines;
mod rows;
mod skills;
use crate::app::{bad, Shared, R};
use crate::server::s;
use axum::extract::{Path, State};
use axum::routing::get;
use axum::{Json, Router};
use serde_json::{json, Value};

/// The bot whose page id is `id`: an owner's `bot-<id>`, or a system bot by its key.
fn bot_by_id(app: &Shared, id: &str) -> Option<Value> {
    crate::bots::bot(app, &format!("bot-{id}")).or_else(|| crate::bots::bot(app, id))
}

/// The numbers of bot `id`.
pub fn numbers(app: &Shared, id: &str) -> Result<Value, String> {
    let bot = bot_by_id(app, id).ok_or(format!("no bot '{id}'"))?;
    let key = s(&bot, "key");
    let rows = rows::rows(app, &key);
    let routines: Vec<(String, String)> = bot["routines"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|r| (s(r, "id"), s(r, "title")))
        .collect();
    let read = |p: std::path::PathBuf| std::fs::read_to_string(p).unwrap_or_default();
    let own = read(app.home.join("manager").join(&key).join("memory.md"));
    let shared = read(app.home.join("bots/memory.md"));
    Ok(json!({
        "bot": id,
        "runs": rows.len(),
        "routines": routines::by_routine(&rows, &routines),
        "skills": skills::by_skill(&rows),
        "memory": memory::memory(app, &key, &rows, (&own, &shared)),
    }))
}

async fn get_numbers(State(app): State<Shared>, Path(id): Path<String>) -> R {
    numbers(&app, &id).map(Json).map_err(bad)
}

pub fn routes() -> Router<Shared> {
    Router::new().route("/api/bots/{id}/numbers", get(get_numbers))
}

#[cfg(test)]
#[path = "../../checks/numbers.rs"]
mod tests;
