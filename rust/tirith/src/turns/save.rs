//! A turn ended: its row in `turns`, and for a bot its `turn.done` (or `routine.done`) event.
use super::price::cost;
use super::record::Turn;
use super::usage::from_muse_session;
use super::versions::versions;
use crate::app::{now_iso, Shared};
use crate::server::s;
use rusqlite::params;
use serde_json::json;

/// Keeps the turn's row: `ok` and `answer` when its caller knows them, else what its lines said (Anthropic's `is_error`, the text so far). Its id.
pub fn finish(app: &Shared, mut t: Turn, ok: Option<bool>, answer: Option<&str>) -> i64 {
    if !t.usage.seen && t.agent == "muse" && !t.session.is_empty() {
        t.usage = from_muse_session(&t.session, t.start_us);
    }
    let cost_usd = t
        .usage
        .cost
        .or_else(|| cost(&app.cfg().raw, &t.model, &t.usage));
    let ok = ok.unwrap_or(!t.usage.failed);
    let answer: String = answer.unwrap_or(&t.text).chars().take(4000).collect();
    let event = event(app, &t, &answer);
    let (factory, role_hash) = app.stamp_for(&t.role);
    let u = &t.usage;
    let ms = t.started.elapsed().as_millis() as i64;
    let (skills, tools) = (json!(t.skills).to_string(), json!(t.tools).to_string());
    let versions = versions(app, &t).to_string();
    let status = if ok { "ok" } else { "failed" };
    let id = app.insert("INSERT INTO turns (conv, role, \"trigger\", routine, started_at, ended_at, ms, status, agent, model, effort, tokens_in, tokens_out, tokens_cache_read, tokens_cache_write, cost_usd, steps, skills, tools, versions, answer, event, factory, role_hash) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22,?23,?24)", params![t.conv, t.role, t.trigger, t.routine, t.started_at, now_iso(), ms, status, t.agent, t.model, t.effort, u.input, u.output, u.cache_read, u.cache_write, cost_usd, t.steps, skills, tools, versions, answer, event, factory, role_hash]);
    super::signals::keep(app, id, &t, &answer);
    app.emit("turn", json!({"id": id, "conv": t.conv, "status": status}));
    id
}

/// A bot's turn (a role with `scope = "own"` and `session = "chat"`) emits `turn.done`, or `routine.done` when a routine started it: its id, else none.
fn event(app: &Shared, t: &Turn, answer: &str) -> Option<i64> {
    let bot = crate::bots::bot(app, &t.conv).filter(|b| b["session"] == "chat")?;
    let routine = bot["routines"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|r| !t.routine.is_empty() && r["id"] == t.routine.as_str());
    let names = |v: Vec<String>| Some(v.join(", ")).filter(|s| !s.is_empty());
    let tools = t.tools.iter().map(|c| s(c, "name")).collect::<Vec<_>>();
    let mut text = format!("{} · {}", s(&bot, "name"), t.trigger);
    routine
        .iter()
        .for_each(|r| text += &format!(" · routine «{}»: {}", s(r, "title"), s(r, "prompt")));
    text += &format!(" · tools: {}", names(tools).unwrap_or("none".into()));
    text += &format!(
        " · skills: {}",
        names(t.skills.clone()).unwrap_or("none".into())
    );
    text += &format!(" · {}", answer.chars().take(200).collect::<String>());
    let on = if routine.is_some() {
        "routine.done"
    } else {
        "turn.done"
    };
    Some(crate::manager::emitted(app, &t.conv, on, &text, None))
}
