//! A turn ended: its row in `turns` (`waiting` while an approval card it opened is open), for a bot its `turn.done` (or `routine.done`) event and the commit of what it changed in its folder and the shared one.
use super::price::cost;
use super::record::Turn;
use super::settle::settle;
use super::versions::versions;
use crate::app::{now_iso, Shared};
use crate::server::s;
use rusqlite::params;
use serde_json::json;

/// Keeps the turn's row: `ok` and `answer` when its caller knows them, else what its lines said (Anthropic's `is_error`, the text so far). Its id.
pub fn finish(app: &Shared, mut t: Turn, ok: Option<bool>, answer: Option<&str>) -> i64 {
    super::interrupted::clear(app, &t);
    settle(app, &mut t);
    let cost_usd = t
        .usage
        .cost
        .or_else(|| cost(&app.cfg().raw, &t.model, &t.usage));
    let ok = ok.unwrap_or(!t.usage.failed);
    let whole = answer.unwrap_or(&t.text).to_string(); // how it went is said at its end
    let answer: String = whole.chars().take(4000).collect();
    let waits = super::approval::waits(app, &t);
    let judged = (!waits).then(|| crate::expected::verdict(app, &t, ok, &whole));
    let why = judged.flatten(); // a routine's run that did not do its task
    let event = event(app, &t, &whole, why.as_deref());
    let (factory, role_hash) = app.stamp_for(&t.role);
    let u = &t.usage;
    let ms = t.started.elapsed().as_millis() as i64;
    let (skills, tools) = (json!(t.skills).to_string(), json!(t.tools).to_string());
    let versions = versions(app, &t).to_string();
    let status = match (waits, ok && why.is_none()) {
        (true, _) => "waiting", // its approval card is still open: the owner's answer starts its next turn
        (_, true) => "ok",
        _ => "failed",
    };
    let id = app.insert("INSERT INTO turns (conv, role, \"trigger\", routine, started_at, ended_at, ms, status, agent, model, effort, tokens_in, tokens_out, tokens_cache_read, tokens_cache_write, cost_usd, steps, skills, tools, versions, answer, event, factory, role_hash) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22,?23,?24)", params![t.conv, t.role, t.trigger, t.routine, t.started_at, now_iso(), ms, status, t.agent, t.model, t.effort, u.input, u.output, u.cache_read, u.cache_write, cost_usd, t.steps, skills, tools, versions, answer, event, factory, role_hash]);
    crate::expected::ended(
        app,
        (&t.conv, &t.role, &t.routine),
        id,
        status,
        why.as_deref(),
    );
    super::signals::keep(app, id, &t, &answer);
    let own = app
        .cfg()
        .factory
        .role::<String>(&t.role, "scope")
        .as_deref()
        == Some("own");
    let _ = (own && t.role == t.conv).then(|| {
        super::capture_keep::keep(app, id, &t); // what it read, for a replay
        crate::bots::versioned(app, &t.conv, &format!("turn {id}")) // a bot's folder and the shared one, committed when this turn changed them
    });
    app.emit("turn", json!({"id": id, "conv": t.conv, "status": status}));
    super::quality::catch_up(app, id, event); // after that line: a judge that was quicker may end it failed
    crate::lab::finished(app, id, &t.conv, &t.routine); // a run of a live experiment: its variant, its comparison
    let _ = (t.role == t.conv).then(|| crate::bots::untune(app, &t.conv)); // the routine's own effort or model ends with its turn
    id
}

/// A bot's own turn (a role with `scope = "own"` and `session = "chat"`, in its conversation) emits `turn.done`, or `routine.done` when a routine started it or it read a skill; a turn role's (the CEO's) emits only a routine's, so that is judged too: its id, else none.
fn event(app: &Shared, t: &Turn, answer: &str, why: Option<&str>) -> Option<i64> {
    // Only the bot's own role: a judge's turn in its conversation emits nothing (an eval on turn.done would judge itself forever).
    (t.role == t.conv).then_some(())?;
    let bot = crate::bots::bot(app, &t.conv)?;
    let routine = bot["routines"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|r| !t.routine.is_empty() && r["id"] == t.routine.as_str());
    (bot["session"] == "chat" || routine.is_some()).then_some(())?;
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
    why.iter().for_each(|w| text += &format!(" · failed: {w}"));
    let approval = super::approval::context(app, t);
    // A routine's turn, or a chat turn that read a skill (graded with its «Cómo validar» too); a chat turn that read none is not graded.
    let (on, text) = if routine.is_some() || !t.skills.is_empty() {
        let text = super::quality::routine_text(app, t, &text, answer, &approval);
        ("routine.done", text)
    } else {
        let start = answer.chars().take(200).collect::<String>();
        ("turn.done", format!("{text}{approval} · {start}"))
    };
    Some(crate::manager::emitted(app, &t.conv, on, &text, None))
}
