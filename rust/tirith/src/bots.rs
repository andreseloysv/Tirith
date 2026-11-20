//! Bots: the owner's own agents, with routines, a memory and skills (docs/DAEMON.md).

use crate::app::{bad, now_iso, App, Shared, R};
use crate::client::{call, print, Flag};
use crate::server::s;
use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::{DateTime, Datelike, Local, NaiveTime};
use serde_json::{json, Value};
use std::path::{Path as FsPath, PathBuf};
use std::sync::Mutex;

/// A bot's skills, in its folder: the agents read skills there (or, through a link, in their own .<command>/skills).
const SKILLS: &str = ".agents/skills";
static ONE: Mutex<()> = Mutex::new(());

pub fn all(app: &App) -> Vec<Value> {
    app.kv("bots").and_then(|j| serde_json::from_str(&j).ok()).unwrap_or_default()
}
/// The bot whose conversation is `key` (`bot-<id>`).
pub fn bot(app: &App, key: &str) -> Option<Value> {
    key.strip_prefix("bot-").and_then(|id| all(app).into_iter().find(|b| b["id"] == id))
}
pub fn keys(app: &App) -> Vec<String> {
    all(app).iter().map(|b| format!("bot-{}", s(b, "id"))).collect()
}
pub fn prompt(app: &App, key: &str) -> Option<String> {
    bot(app, key).map(|b| app.cfg().factory.prompt("bot").replace("{name}", &s(&b, "name")).replace("{shared}", &shared(&app.home).display().to_string()))
}

/// A bot's folder, where its sessions run: memory.md, its working files and its skills.
pub fn folder(app: &App, key: &str) -> PathBuf {
    app.home.join("manager").join(key)
}
fn shared(home: &FsPath) -> PathBuf {
    home.join("bots/skills")
}
/// The one part of Tirith's home besides the lanes where agents write: the bots' folders and the shared skills. In a bot's folder a hidden folder is written only
/// as a skills folder: an agent's settings there could turn Tirith's hooks off.
pub fn writable(home: &FsPath, path: &FsPath) -> bool {
    let parts: Vec<String> = path.strip_prefix(home.join("manager")).map(|r| r.iter().map(|c| c.to_string_lossy().into_owned()).collect()).unwrap_or_default();
    let own = parts.first().is_some_and(|b| b.starts_with("bot-")) && parts.windows(2).all(|w| !w[0].starts_with('.') || w[1] == "skills");
    own || path.starts_with(shared(home))
}
/// The skills of a folder of skills, by name with the text of their SKILL.md; `links` false leaves out the shared ones linked into a bot's.
fn skills(dir: &FsPath, links: bool) -> Vec<(String, String)> {
    let entries = std::fs::read_dir(dir).into_iter().flatten().flatten().filter(|e| links || !e.file_type().is_ok_and(|t| t.is_symlink()));
    let mut out: Vec<(String, String)> = entries.filter_map(|e| Some((e.file_name().into_string().ok()?, std::fs::read_to_string(e.path().join("SKILL.md")).ok()?))).collect();
    out.sort();
    out
}

/// What a bot is handed at the start of every turn and routine: its memory as it is now (the owner may have changed it) and the names of its skills.
/// The shared skills are linked into its skills first (a skill of its own with the same name wins), links to removed ones go, and the skills folder of the
/// bots' agent (.<its command>/skills) is linked to its skills.
pub fn context(app: &Shared, key: &str) -> Option<String> {
    bot(app, key)?;
    let (dir, common) = (folder(app, key).join(SKILLS), shared(&app.home));
    let _ = std::fs::create_dir_all(&dir);
    let gone = std::fs::read_dir(&dir).into_iter().flatten().flatten().filter(|e| e.file_type().is_ok_and(|t| t.is_symlink()) && !e.path().join("SKILL.md").exists());
    gone.for_each(|e| drop(std::fs::remove_file(e.path())));
    skills(&common, true).iter().for_each(|(name, _)| drop(std::os::unix::fs::symlink(common.join(name), dir.join(name))));
    let agent = app.cfg().factory.command(&crate::models::role(app, key).agent).and_then(|c| FsPath::new(&c).file_name().map(|n| folder(app, key).join(format!(".{}", n.to_string_lossy()))));
    agent.into_iter().for_each(|d| drop(std::fs::create_dir_all(&d).and_then(|_| std::os::unix::fs::symlink(format!("../{SKILLS}"), d.join("skills")))));
    let names = |v: Vec<(String, String)>| Some(v.into_iter().map(|s| s.0).collect::<Vec<_>>().join(", ")).filter(|n| !n.is_empty()).unwrap_or("none".into());
    let memory = std::fs::read_to_string(folder(app, key).join("memory.md")).unwrap_or_default().chars().take(12000).collect::<String>();
    let memory = if memory.trim().is_empty() { "(empty)" } else { memory.trim() };
    Some(format!("[tirith] Your memory.md as it is now:\n{memory}\n[tirith] Your skills: {}. Shared skills: {}.", names(skills(&dir, false)), names(skills(&common, true))))
}

/// The page's Library: the bot's memory, its own skills and the shared ones.
fn library(app: &App, id: &str) -> Result<Value, String> {
    let key = format!("bot-{id}");
    bot(app, &key).ok_or(format!("no bot '{id}'"))?;
    let list = |v: Vec<(String, String)>, shared: bool| v.into_iter().map(move |(name, text)| json!({"name": name, "text": text, "shared": shared}));
    let all: Vec<Value> = list(skills(&folder(app, &key).join(SKILLS), false), false).chain(list(skills(&shared(&app.home), true), true)).collect();
    Ok(json!({"memory": std::fs::read_to_string(folder(app, &key).join("memory.md")).unwrap_or_default(), "skills": all}))
}
/// A change from the Library: the `memory`, or the skill `name` (`shared`: every bot's), gets `text`, or goes with `delete`.
fn learn(app: &App, id: &str, b: &Value) -> Result<Value, String> {
    let (key, name) = (format!("bot-{id}"), s(b, "name"));
    bot(app, &key).ok_or(format!("no bot '{id}'"))?;
    let named = name.len() <= 64 && name.starts_with(|c: char| c.is_ascii_lowercase()) && name.bytes().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-');
    let skill = (if b["shared"] == true { shared(&app.home) } else { folder(app, &key).join(SKILLS) }).join(&name);
    let dir = if b["memory"] == true { folder(app, &key) } else { skill };
    (b["memory"] == true || named).then_some(()).ok_or("a skill's name is lowercase letters, digits and dashes")?;
    let file = dir.join(if b["memory"] == true { "memory.md" } else { "SKILL.md" });
    let done = match (b["delete"] == true, b["memory"] == true) {
        (true, true) => std::fs::remove_file(&file),
        (true, false) => std::fs::remove_dir_all(&dir),
        _ => std::fs::create_dir_all(&dir).and_then(|_| std::fs::write(&file, s(b, "text"))),
    };
    done.map_err(|e| format!("{}: {e}", file.display()))?;
    app.emit("library", json!({"bot": id}));
    Ok(json!({"ok": true}))
}

/// Reads the bots, changes them with `f` and keeps the result, one change at a time; an Err keeps them as they were.
fn with<T>(app: &Shared, f: impl FnOnce(&mut Vec<Value>) -> Result<T, String>) -> Result<T, String> {
    let _one = ONE.lock().unwrap();
    let mut bots = all(app);
    let out = f(&mut bots)?;
    app.set_kv("bots", Some(&json!(bots).to_string()));
    app.emit("bots", json!({}));
    Ok(out)
}
fn find<'a>(bots: &'a mut [Value], id: &str) -> Result<&'a mut Value, String> {
    bots.iter_mut().find(|b| b["id"] == id).ok_or(format!("no bot '{id}'"))
}
/// Its routines (every bot is made with a list of them).
fn routines<'a>(bots: &'a mut [Value], id: &str) -> Result<&'a mut Vec<Value>, String> {
    find(bots, id)?["routines"].as_array_mut().ok_or(format!("bot '{id}' has no routines"))
}

/// Weekdays 1 (Monday) to 7, from `[1, 3]`, "daily", "weekdays", "weekends" or names such as "mon,wed".
pub fn days(v: &Value) -> Option<Vec<u64>> {
    let t = v.as_str().map(str::to_lowercase);
    let named = |d: &u64| t.as_deref().is_some_and(|t| t == "daily" || (t == "weekdays" && *d < 6) || (t == "weekends" && *d > 5) || t.contains(["mon", "tue", "wed", "thu", "fri", "sat", "sun"][*d as usize - 1]));
    v.as_array().map(|a| a.iter().filter_map(Value::as_u64).filter(|d| (1..=7).contains(d)).collect()).or_else(|| t.is_some().then(|| (1..=7).filter(named).collect()))
}

/// Due once a day: it is on, today is one of its days, its time has come and it has not run since then. A daemon that was down at that time runs it when it is back the same day.
pub fn due(r: &Value, now: DateTime<Local>) -> bool {
    let Ok(at) = NaiveTime::parse_from_str(&s(r, "at"), "%H:%M") else { return false };
    let today = now.date_naive().and_time(at);
    let last = DateTime::parse_from_rfc3339(&s(r, "last")).ok().map(|t| t.with_timezone(&Local).naive_local());
    r["on"] != false && r["days"].as_array().is_some_and(|d| d.contains(&json!(now.weekday().number_from_monday()))) && now.naive_local() >= today && last.is_none_or(|l| l < today)
}

/// Runs a routine now: the conversation shows it, and the bot is handed the task.
fn fire(app: &Shared, id: &str, rid: &str) -> Result<(), String> {
    let (title, task) = with(app, |bots| {
        let r = routines(bots, id)?.iter_mut().find(|r| r["id"] == rid).ok_or("no such routine")?;
        r["last"] = json!(now_iso());
        Ok((s(r, "title"), s(r, "prompt")))
    })?;
    app.chat("routine", &format!("Running routine · {title}"), &format!("bot-{id}"));
    crate::manager::emit_event(app, &format!("bot-{id}"), "schedule", &format!("Routine «{title}» is due now. Do it and report the result here in a few lines (when nothing is new, one line that starts with ∅); ask nothing: {task}"), None);
    Ok(())
}

pub fn start(app: Shared) {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(20)).await;
            all(&app).iter().for_each(|b| b["routines"].as_array().into_iter().flatten().filter(|r| due(r, Local::now())).for_each(|r| drop(fire(&app, &s(b, "id"), &s(r, "id")))));
        }
    });
}

/// A new bot (no `id`), or a new name, shape or colour for one.
fn save(app: &Shared, b: Value) -> Result<Value, String> {
    let (name, color) = (s(&b, "name").trim().to_string(), s(&b, "color"));
    (!name.is_empty() && color.len() == 7 && color.starts_with('#') && color[1..].bytes().all(|c| c.is_ascii_hexdigit())).then_some(()).ok_or("a bot needs a name and a colour #rrggbb")?;
    with(app, |bots| {
        let (id, fresh) = (s(&b, "id"), s(&b, "id").is_empty().then(|| format!("{:06x}", rand::random::<u32>() >> 8)));
        bots.extend(fresh.iter().map(|id| json!({"id": id, "routines": [], "created_at": now_iso()})));
        let bot = find(bots, fresh.as_deref().unwrap_or(&id))?;
        (bot["name"], bot["shape"], bot["color"]) = (json!(name), b["shape"].clone(), json!(color));
        Ok(bot.clone())
    })
}

/// A new routine, or a change to one (found by `id` or by its title): `title`, `prompt`, `days`, `at` ("HH:MM"), `on`; with `delete` it goes. The conversation says when one is made or removed.
fn routine(app: &Shared, id: String, b: Value) -> Result<Value, String> {
    let (rid, title) = (s(&b, "id"), s(&b, "title"));
    let said = with(app, |bots| {
        let list = routines(bots, &id)?;
        let i = list.iter().position(|r| (!rid.is_empty() && r["id"] == rid.as_str()) || (!title.is_empty() && s(r, "title").eq_ignore_ascii_case(&title)));
        if b["delete"] == true {
            return Ok(format!("Removed routine · {}", s(&list.remove(i.ok_or("no such routine")?), "title")));
        }
        list.extend(i.is_none().then(|| json!({"id": format!("r{:06x}", rand::random::<u32>() >> 8), "on": true})));
        let last = list.len() - 1;
        let r = &mut list[i.unwrap_or(last)];
        ["title", "prompt", "at", "on"].into_iter().filter(|k| !b[*k].is_null()).for_each(|k| r[k] = b[k].clone());
        r["days"] = days(&b["days"]).map_or(r["days"].clone(), |d| json!(d));
        let at = NaiveTime::parse_from_str(s(r, "at").trim(), "%H:%M").map_err(|_| "the time must be HH:MM, as 08:00")?;
        // A changed routine does not run for a time already past today.
        (r["at"], r["last"]) = (json!(at.format("%H:%M").to_string()), json!(now_iso()));
        let ok = !s(r, "title").trim().is_empty() && !s(r, "prompt").trim().is_empty() && r["days"].as_array().is_some_and(|d| !d.is_empty());
        ok.then_some(()).ok_or("a routine needs a title, what to do and at least one day")?;
        Ok(if i.is_none() { format!("Created routine · {}", s(r, "title")) } else { String::new() })
    })?;
    let _ = (!said.is_empty()).then(|| app.chat("routine", &said, &format!("bot-{id}")));
    Ok(json!({"ok": true}))
}

fn reply(res: Result<Value, String>) -> R {
    res.map(Json).map_err(bad)
}

pub fn routes() -> Router<Shared> {
    let delete = |State(app): State<Shared>, Path(id): Path<String>| async move {
        with(&app, |bots| bots.iter().position(|b| b["id"] == id.as_str()).map(|i| bots.remove(i)).ok_or(format!("no bot '{id}'"))).map_err(bad)?;
        crate::rooms::forget(&app, &format!("bot-{id}"));
        let _ = std::fs::remove_dir_all(folder(&app, &format!("bot-{id}"))); // its memory and its own skills
        reply(Ok(json!({"ok": true})))
    };
    let list = |State(app): State<Shared>, Path(id): Path<String>| async move { reply(bot(&app, &format!("bot-{id}")).map(|b| b["routines"].clone()).ok_or(format!("no bot '{id}'"))) };
    let change = |State(app): State<Shared>, Path(id): Path<String>, Json(b): Json<Value>| async move { reply(routine(&app, id, b)) };
    let run = |State(app): State<Shared>, Path((id, rid)): Path<(String, String)>| async move { reply(fire(&app, &id, &rid).map(|_| json!({"ok": true}))) };
    let lib = |State(app): State<Shared>, Path(id): Path<String>, Json(b): Json<Value>| async move { reply(learn(&app, &id, &b)) };
    Router::new()
        .route("/api/bots", get(|State(app): State<Shared>| async move { Json(json!(all(&app))) }).post(|State(app): State<Shared>, Json(b): Json<Value>| async move { reply(save(&app, b)) }))
        .route("/api/bots/{id}/delete", post(delete))
        .route("/api/bots/{id}/routines", get(list).post(change))
        .route("/api/bots/{id}/run/{routine}", post(run))
        .route("/api/bots/{id}/library", get(|State(app): State<Shared>, Path(id): Path<String>| async move { reply(library(&app, &id)) }).post(lib))
}

/// A bot's own commands, in its conversation (`TIRITH_PROJECT` is `bot-<id>`): `tirith routine add|remove|list`.
pub fn cli(pos: &[&str], _: Flag) -> Option<Result<(), String>> {
    let key = std::env::var("TIRITH_PROJECT").unwrap_or_default();
    let path = format!("/api/bots/{}/routines", key.strip_prefix("bot-").unwrap_or(&key));
    match pos {
        ["routine", "add", title, days, at, task] => Some(print(call("POST", &path, Some(json!({"title": title, "days": days, "at": at, "prompt": task}))))),
        ["routine", "remove", title] => Some(print(call("POST", &path, Some(json!({"title": title, "delete": true}))))),
        ["routine", "list"] => Some(call("GET", &path, None).map(|b| println!("{b}"))),
        _ => None,
    }
}

#[cfg(test)]
#[path = "../checks/bots.rs"]
mod tests;
