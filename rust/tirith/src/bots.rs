//! Bots: agents of their own — factory roles with `scope = "own"` (the owner's `bot-<id>` ones and the factory's system bots: CEO, judge, manager of no project) with `schedule` routes in `~/.tirith/config.toml` (written with toml_edit, keeping its comments; a change to a system bot is laid over the factory there), each with a memory and skills (docs/DAEMON.md). A routine's last run stays in kv: it is a stamp, not configuration. What makes a bot a CEO or a judge is only this configuration — its `[[routes]]` (which events it hears, as a turn or a note, on the phones or not), its `session` and its `access` — and its card on the page shows and changes all of it, saved here; routes config.toml writes to a role replace the factory's to that role (see factory.rs).
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
/// The bots as the page sees them: every role of `raw` with `scope = "own"` (an owner's `bot-<id>`, in config.toml, or a system bot by its key; its name, face and `show`), with its `schedule` routes as routines (`last` says a routine's last run), its other routes as `routes` (each as written; writing one back sets its `to` to this bot), and its `session` and `access`: the whole configuration its card shows and changes. `key` is its conversation; `id` is its page id (a system bot's key).
fn bots_of(raw: &Value, last: &dyn Fn(&str, &str) -> String) -> Vec<Value> {
    // `all` reads them from the effective factory table: the owner's bots through config.toml's overlay, the system bots as factory roles
    let routes = raw["routes"].as_array().cloned().unwrap_or_default();
    let or = |x: String, d: &str| Some(x).filter(|x| !x.is_empty()).unwrap_or_else(|| d.to_string());
    let bot = |(key, role): (&String, &Value)| {
        let id = (s(role, "scope") == "own").then(|| key.strip_prefix("bot-").unwrap_or(key))?;
        let routines: Vec<Value> = routes
            .iter()
            .filter(|r| s(r, "on") == "schedule" && s(r, "to") == *key && !s(r, "task").is_empty())
            .enumerate()
            .map(|(i, r)| {
                let rid = or(s(r, "id"), &format!("r{i}"));
                json!({"id": rid.clone(), "title": or(s(r, "title"), &s(r, "task")), "prompt": s(r, "task"), "days": days(&r["days"]).unwrap_or_default(), "at": s(r, "at"), "on": r["enabled"] != false, "last": last(id, &rid)})
            })
            .collect();
        let hears: Vec<Value> = routes.iter().filter(|r| s(r, "on") != "schedule" && s(r, "to") == *key).cloned().collect();
        Some(
            json!({"id": id, "key": key, "system": !key.starts_with("bot-"), "show": role["show"] != false, "name": or(s(role, "name"), id), "shape": or(s(&role["face"], "shape"), "round"), "color": or(s(&role["face"], "color"), "#3b82f6"), "session": or(s(role, "session"), "chat"), "access": s(role, "access"), "chat_keep": s(role, "chat_keep"), "routes": hears, "routines": routines}),
        )
    };
    raw["roles"].as_object().into_iter().flatten().filter_map(bot).collect()
}
pub fn all(app: &App) -> Vec<Value> {
    bots_of(&serde_json::to_value(app.cfg().factory.table()).unwrap_or_default(), &|id, rid| app.kv(&format!("routine:{id}:{rid}")).unwrap_or_default())
}
/// The bot whose conversation is `key` (`bot-<id>`, or a system bot's key).
pub fn bot(app: &App, key: &str) -> Option<Value> {
    all(app).into_iter().find(|b| b["key"] == key)
}
fn key_for(app: &App, id: &str) -> Option<String> {
    all(app).into_iter().find(|b| b["id"] == id).map(|b| s(&b, "key"))
}
/// A bot's folder, where its sessions run: memory.md, its working files and its skills.
pub fn folder(app: &App, key: &str) -> PathBuf {
    app.home.join("manager").join(key)
}
/// The one part of Tirith's home besides the lanes where agents write: the bots' folders and the shared skills. In a bot's folder a hidden folder is written only as a skills folder: an agent's settings there could turn Tirith's hooks off.
pub fn writable(home: &FsPath, path: &FsPath) -> bool {
    let parts: Vec<String> = path.strip_prefix(home.join("manager")).map(|r| r.iter().map(|c| c.to_string_lossy().into_owned()).collect()).unwrap_or_default();
    let own = parts.first().is_some_and(|b| b.starts_with("bot-") || crate::factory::current().role::<String>(b, "scope").as_deref() == Some("own")) && parts.windows(2).all(|w| !w[0].starts_with('.') || w[1] == "skills");
    own || path.starts_with(home.join("bots/skills"))
}
/// The skills of a folder of skills, by name with the text of their SKILL.md; `links` false leaves out the shared ones linked into a bot's.
fn skills(dir: &FsPath, links: bool) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| links || !e.file_type().is_ok_and(|t| t.is_symlink()))
        .filter_map(|e| Some((e.file_name().into_string().ok()?, std::fs::read_to_string(e.path().join("SKILL.md")).ok()?)))
        .collect();
    out.sort();
    out
}
/// What a bot is handed at the start of every turn and routine: its memory as it is now (the owner may have changed it) and the names of its skills. The shared skills are linked into its skills first (a skill of its own with the same name wins), links to removed ones go, and the skills folder of the bots' agent (.<its command>/skills) is linked to its skills.
pub fn context(app: &Shared, key: &str) -> Option<String> {
    bot(app, key)?;
    let (dir, common) = (folder(app, key).join(SKILLS), app.home.join("bots/skills"));
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
    let key = key_for(app, id).ok_or(format!("no bot '{id}'"))?;
    let list = |v: Vec<(String, String)>, shared: bool| v.into_iter().map(move |(name, text)| json!({"name": name, "text": text, "shared": shared}));
    let all: Vec<Value> = list(skills(&folder(app, &key).join(SKILLS), false), false).chain(list(skills(&app.home.join("bots/skills"), true), true)).collect();
    Ok(json!({"memory": std::fs::read_to_string(folder(app, &key).join("memory.md")).unwrap_or_default(), "skills": all}))
}
/// A change from the Library: the `memory`, or the skill `name` (`shared`: every bot's), gets `text`, or goes with `delete`.
fn learn(app: &App, id: &str, b: &Value) -> Result<Value, String> {
    let (key, name) = (key_for(app, id).ok_or(format!("no bot '{id}'"))?, s(b, "name"));
    let named = name.len() <= 64 && name.starts_with(|c: char| c.is_ascii_lowercase()) && name.bytes().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-');
    let skill = (if b["shared"] == true { app.home.join("bots/skills") } else { folder(app, &key).join(SKILLS) }).join(&name);
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
/// Writes the bots into config.toml and reloads it.
fn store(app: &App, bots: &[Value]) -> Result<(), String> {
    let _writing = crate::projects::WRITING.lock().unwrap();
    let path = app.home.join("config.toml");
    let fail = |e: std::io::Error| format!("{}: {e}", path.display());
    let text = std::fs::read_to_string(&path).or_else(|e| if e.kind() == std::io::ErrorKind::NotFound { Ok(String::new()) } else { Err(e) }).map_err(fail)?;
    std::fs::write(&path, written(&text, bots)?).map_err(fail)?;
    app.reload_config(true);
    Ok(())
}
/// The text of config.toml (`text` as it is) with the bots laid into it by toml_edit, so its other sections and their comments stay as they are: each of the owner's bots a `[roles.bot-<id>]` that extends the manager with its own scope, name, face, session and access, each routine a `[[routes]]` on `schedule` and each of its routes a `[[routes]]` of its own; a bot not in `bots` goes, with its routes. A system bot has no role of its own here: only what the owner changed of it (`dirty`: its name, face, show, session and access, and its routes with them) is laid into its factory role's table (a real table, created with `entry` — an inline one would make a later `[roles.bot-…]` header unparseable) as a layer over the factory, its other keys (a model, say) untouched; an untouched system bot's routes in the file stay as they are.
fn written(text: &str, bots: &[Value]) -> Result<String, String> {
    use toml_edit::DocumentMut;
    let mut doc: DocumentMut = text.parse().map_err(|e| format!("config.toml: {e}"))?;
    let key_of = |b: &Value| Some(s(b, "key")).filter(|k| !k.is_empty()).unwrap_or_else(|| format!("bot-{}", s(b, "id")));
    // `has(to, changed)`: `bots` has that bot (with `changed`, one whose routes are (re)written here: every owner's bot, and a system bot the owner changed). Every bot role and schedule route goes out of the file (and a changed bot's other routes, written again with it); the bots are written again after the owner's sections, together at the end, no empty header behind.
    let has = |to: &str, changed: bool| bots.iter().any(|b| (!changed || b["system"] != true || b["dirty"] == true) && key_of(b) == to);
    doc.get_mut("roles").and_then(|i| i.as_table_mut()).into_iter().for_each(|roles| roles.retain(|k, v| !(k.starts_with("bot-") && v.as_table().and_then(|t| t.get("scope")).and_then(|i| i.as_str()) == Some("own"))));
    doc.get_mut("routes")
        .and_then(|i| i.as_array_of_tables_mut())
        .into_iter()
        .for_each(|routes| routes.retain(|r| !r.get("to").and_then(|i| i.as_str()).is_some_and(|to| to.starts_with("bot-") || has(to, r.get("on").and_then(|i| i.as_str()) != Some("schedule")))));
    for b in bots.iter().filter(|b| b["system"] == true && b["dirty"] == true) {
        let key = key_of(b);
        let roles = doc.as_table_mut().entry("roles").or_insert_with(|| toml_edit::Item::Table(toml_edit::Table::new()));
        let role = roles.as_table_mut().map(|r| r.entry(&key).or_insert_with(|| toml_edit::Item::Table(toml_edit::Table::new())));
        let _ = role.map(|role| {
            (role["name"], role["show"], role["session"], role["access"], role["face"]["shape"], role["face"]["color"]) =
                (toml_edit::value(s(b, "name")), toml_edit::value(b["show"] != false), toml_edit::value(s(b, "session")), toml_edit::value(s(b, "access")), toml_edit::value(s(b, "shape")), toml_edit::value(s(b, "color")))
        });
    }
    ["roles", "routes"].into_iter().for_each(|k| drop(doc.get(k).and_then(|i| i.as_table().map(|t| t.is_empty()).or_else(|| i.as_array_of_tables().map(|t| t.is_empty()))).is_some_and(|e| e).then(|| doc.remove(k))));
    let (mut roles, mut routes) = (serde_json::Map::new(), vec![]);
    for b in bots {
        let _ = (b["system"] != true).then(|| {
            let mut role = json!({"extends": "manager", "scope": "own", "name": s(b, "name"), "face": {"shape": s(b, "shape"), "color": s(b, "color")}, "prompt": ["prompts/bot.md"], "session": Some(s(b, "session")).filter(|x| !x.is_empty()).unwrap_or_else(|| "chat".into()), "access": s(b, "access")});
            let _ = (!s(b, "chat_keep").is_empty()).then(|| role["chat_keep"] = json!(s(b, "chat_keep"))); // the chat's longest life, kept as written (see chat.rs)
            roles.insert(key_of(b), role)
        });
        routes.extend(b["routes"].as_array().into_iter().flatten().filter(|_| has(&key_of(b), true)).cloned().map(|mut r| (r["to"] = json!(key_of(b)), r).1));
        routes.extend(
            b["routines"]
                .as_array()
                .into_iter()
                .flatten()
                .map(|r| json!({"on": "schedule", "to": key_of(b), "mode": "turn", "phone": true, "id": s(r, "id"), "title": s(r, "title"), "days": r["days"], "at": s(r, "at"), "task": s(r, "prompt"), "enabled": r["on"] != false})),
        );
    }
    // An empty half stays out of the fragment: a bare `routes = []` (or `roles`) would land inside the file's last section instead of at its root.
    let parts: serde_json::Map<String, Value> =
        [("roles", json!(roles)), ("routes", json!(routes))].into_iter().filter(|(_, v)| !matches!(v, Value::Object(o) if o.is_empty()) && !matches!(v, Value::Array(a) if a.is_empty())).map(|(k, v)| (k.to_string(), v)).collect();
    let frag = if parts.is_empty() { String::new() } else { toml::Value::try_from(Value::Object(parts)).and_then(|v| toml::to_string(&v)).map_err(|e| e.to_string())? };
    let head = doc.to_string();
    Ok(if frag.is_empty() {
        head
    } else if head.trim().is_empty() {
        frag
    } else {
        format!("{}\n\n{frag}", head.trim_end())
    })
}
/// Reads the bots, changes them with `f` and keeps the result, one change at a time; an Err keeps them as they were.
fn with<T>(app: &Shared, f: impl FnOnce(&mut Vec<Value>) -> Result<T, String>) -> Result<T, String> {
    let _one = ONE.lock().unwrap();
    let mut bots = all(app);
    let out = f(&mut bots)?;
    store(app, &bots)?;
    app.emit("bots", json!({}));
    Ok(out)
}
fn find<'a>(bots: &'a mut [Value], id: &str) -> Result<&'a mut Value, String> {
    bots.iter_mut().find(|b| b["id"] == id).ok_or(format!("no bot '{id}'"))
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
    let mut bots = all(app);
    let bot = find(&mut bots, id)?;
    let (key, r) = (s(bot, "key"), bot["routines"].as_array().into_iter().flatten().find(|r| r["id"] == rid).ok_or("no such routine")?);
    let (title, task) = (s(r, "title"), s(r, "prompt"));
    app.set_kv(&format!("routine:{id}:{rid}"), Some(&now_iso()));
    app.chat("routine", &format!("Running routine · {title}"), &key);
    crate::manager::emit_event(app, &key, "schedule", &format!("Routine «{title}» is due now. Do it and report the result here in a few lines (when nothing is new, one line that starts with ∅); ask nothing: {task}"), None);
    Ok(())
}
/// At start, the bots that used to live in kv move to config.toml once: one that is already a role there wins, and their routines' last runs move to their own kv stamps; only then does the old entry go. The old key's name lives here alone, so nothing reads it as storage any more.
pub fn start(app: Shared) {
    const OLD: &str = "bots";
    all(&app).iter().map(|b| folder(&app, &s(b, "key"))).for_each(|d| drop((std::fs::create_dir_all(d.join(SKILLS)), std::fs::OpenOptions::new().create(true).write(true).truncate(false).open(d.join("memory.md"))))); // every bot (the system ones too): its folder under manager/, its memory.md and its skills folder, from the start
    let old: Vec<Value> = app.kv(OLD).and_then(|j| serde_json::from_str(&j).ok()).unwrap_or_default();
    let Some(old) = (!old.is_empty()).then_some(old) else { return };
    let mut bots = all(&app);
    let have: Vec<String> = bots.iter().map(|b| s(b, "id")).collect();
    bots.extend(old.iter().filter(|o| !have.contains(&s(o, "id"))).cloned());
    if store(&app, &bots).is_ok() {
        old.iter().for_each(|b| b["routines"].as_array().into_iter().flatten().filter(|r| !s(r, "last").is_empty()).for_each(|r| app.set_kv(&format!("routine:{}:{}", s(b, "id"), s(r, "id")), Some(&s(r, "last")))));
        app.set_kv(OLD, None);
    }
}
/// Fires the routines due now; the lanes scheduler calls it on its tick (every ~30 s), so the bots need no timer of their own.
pub fn tick(app: &Shared) {
    all(app).iter().for_each(|b| b["routines"].as_array().into_iter().flatten().filter(|r| due(r, Local::now())).for_each(|r| drop(fire(app, &s(b, "id"), &s(r, "id")))));
}
/// A new bot (no `id`), or a change to one's card: its name, shape and colour, its `session` and `access`, and its routes (each on an event of the closed list but `schedule` — those are its routines — as a turn or a note, with its chat line and phones as written). What the body does not carry stays as it was.
pub(crate) fn save(app: &Shared, b: Value) -> Result<Value, String> {
    let (name, color) = (s(&b, "name").trim().to_string(), s(&b, "color"));
    (!name.is_empty() && color.len() == 7 && color.starts_with('#') && color[1..].bytes().all(|c| c.is_ascii_hexdigit())).then_some(()).ok_or("a bot needs a name and a colour #rrggbb")?;
    let hears = b["routes"].is_null() || b["routes"].as_array().is_some_and(|rs| rs.iter().all(|r| crate::factory::EVENTS.split(';').any(|e| e == s(r, "on") && e != "schedule") && matches!(s(r, "mode").as_str(), "" | "turn" | "note")));
    (matches!(s(&b, "session").as_str(), "" | "chat" | "turn") && matches!(s(&b, "access").as_str(), "" | "read" | "write") && hears)
        .then_some(())
        .ok_or("a bot's session is chat or turn, its access read or write, and each route is on an event of the closed list (not schedule) as a turn or a note")?;
    with(app, |bots| {
        let (id, fresh) = (s(&b, "id"), s(&b, "id").is_empty().then(|| format!("{:06x}", rand::random::<u32>() >> 8)));
        bots.extend(fresh.iter().map(|id| json!({"id": id, "routines": [], "routes": []})));
        let bot = find(bots, fresh.as_deref().unwrap_or(&id))?;
        let keep = |k: &str| Some(s(&b, k)).filter(|x| !x.is_empty() || !b[k].is_null()).unwrap_or_else(|| s(bot, k)); // carried (even emptied) wins; uncarried stays
        let (session, access, routes) = (Some(keep("session")).filter(|x| !x.is_empty()).unwrap_or_else(|| s(bot, "session")), keep("access"), if b["routes"].is_null() { bot["routes"].clone() } else { b["routes"].clone() }); // a session never empties (a bot always runs somehow); an access does (back to writing)
        (bot["name"], bot["shape"], bot["color"], bot["session"], bot["access"], bot["routes"], bot["dirty"]) = (json!(name), b["shape"].clone(), json!(color), json!(session), json!(access), routes, json!(true)); // `dirty`: a system bot's change is written as a layer in config.toml
        Ok(bot.clone())
    })
}
/// A new routine, or a change to one (found by `id` or by its title): `title`, `prompt`, `days`, `at` ("HH:MM"), `on`; with `delete` it goes. The conversation says when one is made or removed.
fn routine(app: &Shared, id: String, b: Value) -> Result<Value, String> {
    let (rid, title) = (s(&b, "id"), s(&b, "title"));
    let (said, changed) = with(app, |bots| {
        let list = find(bots, &id)?["routines"].as_array_mut().ok_or(format!("bot '{id}' has no routines"))?;
        let i = list.iter().position(|r| (!rid.is_empty() && r["id"] == rid.as_str()) || (!title.is_empty() && s(r, "title").eq_ignore_ascii_case(&title)));
        let Some(()) = (b["delete"] != true).then_some(()) else { return Ok((format!("Removed routine · {}", s(&list.remove(i.ok_or("no such routine")?), "title")), String::new())) };
        list.extend(i.is_none().then(|| json!({"id": format!("r{:06x}", rand::random::<u32>() >> 8), "on": true})));
        let last = list.len() - 1;
        let r = &mut list[i.unwrap_or(last)];
        ["title", "prompt", "at", "on"].into_iter().filter(|k| !b[*k].is_null()).for_each(|k| r[k] = b[k].clone());
        r["days"] = days(&b["days"]).map_or(r["days"].clone(), |d| json!(d));
        let at = NaiveTime::parse_from_str(s(r, "at").trim(), "%H:%M").map_err(|_| "the time must be HH:MM, as 08:00")?;
        r["at"] = json!(at.format("%H:%M").to_string());
        let ok = !s(r, "title").trim().is_empty() && !s(r, "prompt").trim().is_empty() && r["days"].as_array().is_some_and(|d| !d.is_empty());
        ok.then_some(()).ok_or("a routine needs a title, what to do and at least one day")?;
        Ok((if i.is_none() { format!("Created routine · {}", s(r, "title")) } else { String::new() }, s(r, "id")))
    })?;
    // A changed routine does not run for a time already past today.
    let _ = (!changed.is_empty()).then(|| app.set_kv(&format!("routine:{id}:{changed}"), Some(&now_iso())));
    let _ = (!said.is_empty()).then(|| app.chat("routine", &said, &key_for(app, &id).unwrap_or_default()));
    Ok(json!({"ok": true}))
}
fn reply(res: Result<Value, String>) -> R {
    res.map(Json).map_err(bad)
}
pub fn routes() -> Router<Shared> {
    let delete = |State(app): State<Shared>, Path(id): Path<String>| async move {
        let key = key_for(&app, &id).filter(|k| k.starts_with("bot-")).ok_or_else(|| bad(format!("no bot '{id}' (a system bot stays)")))?;
        with(&app, |bots| bots.iter().position(|b| b["id"] == id.as_str()).map(|i| bots.remove(i)).ok_or(format!("no bot '{id}'"))).map_err(bad)?;
        crate::rooms::forget(&app, &key);
        let _ = std::fs::remove_dir_all(folder(&app, &key)); // its memory and its own skills
        reply(Ok(json!({"ok": true})))
    };
    let list = |State(app): State<Shared>, Path(id): Path<String>| async move { reply(key_for(&app, &id).and_then(|k| bot(&app, &k)).map(|b| b["routines"].clone()).ok_or(format!("no bot '{id}'"))) };
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
