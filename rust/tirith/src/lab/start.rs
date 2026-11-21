//! `tirith lab start`: an experiment of the Coach begins on one routine of a bot, its branch checked out in a worktree of the lab.
use super::git::{git, one_commit};
use super::store::{running, running_one, tree};
use crate::app::{now_iso, App};
use crate::server::s;
use rusqlite::params;
use serde_json::{json, Value};

/// The bot `who` names (its key, its id or its name) as (key, its routines).
fn bot_of(app: &App, who: &str) -> Result<(String, Vec<Value>), String> {
    let bots = crate::bots::all(app);
    let b = bots
        .iter()
        .find(|b| b["key"] == who || b["id"] == who || s(b, "name").eq_ignore_ascii_case(who))
        .ok_or(format!("no bot '{who}'"))?;
    let routines = b["routines"].as_array().cloned().unwrap_or_default();
    Ok((s(b, "key"), routines))
}

/// What `tirith lab start` asks for, checked: the bot, a routine of its own with no experiment running, the mode (live) and a branch with one commit over its main. Its row and its worktree, or why not.
pub fn start(app: &App, b: &Value) -> Result<Value, String> {
    let (key, routines) = bot_of(app, &s(b, "bot"))?;
    let (branch, routine) = (s(b, "branch"), s(b, "routine"));
    let hypothesis = s(b, "hypothesis").trim().to_string();
    routines
        .iter()
        .any(|r| r["id"] == routine.as_str())
        .then_some(())
        .ok_or(format!("'{key}' has no routine '{routine}'"))?;
    let mode = Some(s(b, "mode")).filter(|m| !m.is_empty());
    match mode.as_deref().unwrap_or("live") {
        "live" => Ok(()),
        "replay" | "shadow" => Err("only --mode live runs for now".to_string()),
        m => Err(format!("no mode '{m}': live, replay or shadow")),
    }?;
    (!hypothesis.is_empty())
        .then_some(())
        .ok_or("an experiment needs its hypothesis (--hypothesis \"<text>\")")?;
    let on = running(app, &key, &routine);
    on.map_or(Ok(()), |e| {
        Err(format!(
            "routine '{routine}' already runs experiment {} ({}): one at a time",
            e.id, e.branch
        ))
    })?;
    let dir = crate::bots::folder(app, &key);
    let change = one_commit(&dir, &branch)?;
    let (factory, _) = app.stamp_for(&key);
    let id = app.insert("INSERT INTO experiments (bot, routine, branch, change, hypothesis, mode, status, created_at, factory) VALUES (?1, ?2, ?3, ?4, ?5, 'live', 'running', ?6, ?7)", params![key, routine, branch, change, hypothesis, now_iso(), factory]);
    let e = running_one(app, id).ok_or("the experiment was not kept")?;
    let path = tree(app, &e);
    let _ = std::fs::create_dir_all(path.parent().unwrap_or(&path));
    let made = git(
        &dir,
        &[
            "worktree",
            "add",
            "-q",
            &path.display().to_string(),
            &branch,
        ],
    );
    if let Err(why) = made {
        app.insert("DELETE FROM experiments WHERE id = ?1", params![id]);
        return Err(why);
    }
    app.emit("lab", json!({"id": id, "bot": key, "status": "running"}));
    Ok(
        json!({"id": id, "bot": key, "routine": routine, "branch": branch, "change": change, "tree": path.display().to_string()}),
    )
}
