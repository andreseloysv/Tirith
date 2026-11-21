//! Clearing a conversation and giving it a longest life: `clear` drops its chat rows, forgets its saved agent session (kv `manager_session:*:<conversation>`) and ends its live one without cutting a running turn, so its bot starts fresh (memory.md, skills and routines stay). Its life is `chat_keep`, the owner's own words in `~/.tirith/config.toml` — on the bot's role or on the project, written with toml_edit: every minute the rows older than it go, and a sweep that dropped any forgets the saved session too, so the bot cannot remember what is gone. The phones hear both as one sealed line, `tirith-chat {"v":1,...}`, like `tirith-models-state`.
use crate::app::{bad, now_iso, now_s, Shared};
use crate::client::Flag;
use crate::server::s;
use axum::extract::{Query, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::{json, Value};
use std::time::Duration;
/// The owner's words for how long a chat lives (`30m`, `1h`, `24 h`, `2 días`, `una semana`), in seconds; «siempre» (or nothing) is no limit, anything else unreadable is an error said plainly.
pub fn parse_keep(words: &str) -> Result<Option<u64>, String> {
    let w = words.trim().trim_end_matches('.').to_lowercase();
    let err = || format!("no entiendo «{words}»: escribe un tiempo como 30m, 1h, 24 h, 2 días o una semana, o «siempre» para quitar el límite");
    if matches!(w.as_str(), "" | "siempre" | "para siempre" | "always" | "forever") {
        return Ok(None);
    }
    let (num, unit) = match w.strip_prefix("una ").or_else(|| w.strip_prefix("un ")) {
        Some(u) => (1.0, u.trim()),
        None => {
            let i = w.find(|c: char| !c.is_ascii_digit() && c != '.').unwrap_or(w.len());
            (w[..i].parse::<f64>().map_err(|_| err())?, w[i..].trim())
        }
    };
    let unit_s: u64 = match unit {
        "s" | "seg" | "segs" | "segundo" | "segundos" | "sec" | "secs" | "second" | "seconds" => 1,
        "m" | "min" | "mins" | "minuto" | "minutos" | "minute" | "minutes" => 60,
        "h" | "hr" | "hrs" | "hora" | "horas" | "hour" | "hours" => 3600,
        "d" | "día" | "días" | "dia" | "dias" | "day" | "days" => 86400,
        "sem" | "semana" | "semanas" | "week" | "weeks" => 604800,
        _ => return Err(err()),
    };
    let total = (num * unit_s as f64) as u64;
    (total > 0).then_some(Some(total)).ok_or_else(err)
}
/// The words a conversation's life is kept in, as written ("" is none): a project's own `chat_keep`, else its bot role's.
fn keep_words(app: &Shared, conv: &str) -> String {
    let cfg = app.cfg();
    cfg.project(conv).map(|p| p.chat_keep.clone()).filter(|w| !w.is_empty()).or_else(|| cfg.factory.role::<String>(conv, "chat_keep")).unwrap_or_default()
}
/// A conversation's longest life in seconds, when it has one that reads; words nothing can read (written by hand) limit nothing.
fn keep_s(app: &Shared, conv: &str) -> Option<u64> {
    parse_keep(&keep_words(app, conv)).ok().flatten()
}
/// Forgets a conversation's saved agent sessions, one per agent alias, so its next session starts fresh instead of resuming what is gone.
fn forget_session(app: &Shared, conv: &str) {
    let end = format!(":{conv}");
    app.query("SELECT key FROM kv WHERE key LIKE 'manager_session:%'", [], |r| r.get::<_, String>(0)).iter().filter(|k| k.ends_with(&end)).for_each(|k| app.set_kv(k, None));
}
/// One sealed line to the paired rooms of a conversation, for the phone page to hide what is gone.
fn to_phones(app: &Shared, conv: &str, body: Value) {
    crate::rooms::to_phones(app, conv, &format!("tirith-chat {body}"));
}
/// Clears one conversation: its rows go, its saved session is forgotten and its live one ends without cutting a running turn, so the bot starts from zero; its memory, skills and routines stay.
pub fn clear(app: &Shared, project: &str) -> Result<Value, String> {
    (project.is_empty() || app.conv(project) == project).then_some(()).ok_or(format!("unknown project '{project}'"))?;
    let conv = app.conv(project);
    let _ = app.db.lock().unwrap().execute("DELETE FROM chat WHERE project = ?1", [conv]);
    forget_session(app, conv);
    crate::manager::end_session(conv);
    app.emit("chat-clear", json!({"project": conv}));
    to_phones(app, conv, json!({"v": 1, "clear": now_iso()}));
    Ok(json!({"ok": true, "project": conv}))
}
/// `text` of config.toml with a conversation's `chat_keep` set to `words` (or gone with None), by toml_edit so its other sections and comments stay: on its `[[projects]]` table, or on its bot's `[roles.<key>]`.
fn written(text: &str, is_project: bool, conv: &str, words: Option<&str>) -> Result<String, String> {
    let mut doc: toml_edit::DocumentMut = text.parse().map_err(|e| format!("config.toml: {e}"))?;
    let put = |t: &mut dyn toml_edit::TableLike| match words {
        Some(w) => drop(t.insert("chat_keep", toml_edit::value(w))),
        None => drop(t.remove("chat_keep")),
    };
    if is_project {
        let list = doc.get_mut("projects").and_then(|i| i.as_array_of_tables_mut()).ok_or("config.toml has no [[projects]] to write chat_keep on")?;
        put(list.iter_mut().find(|t| t.get("id").and_then(|i| i.as_str()) == Some(conv)).ok_or(format!("no [[projects]] with id '{conv}' in config.toml"))?);
    } else {
        let roles = doc.as_table_mut().entry("roles").or_insert_with(|| toml_edit::Item::Table(toml_edit::Table::new()));
        let role = roles.as_table_mut().ok_or("config.toml's [roles] is not a table")?.entry(conv).or_insert_with(|| toml_edit::Item::Table(toml_edit::Table::new()));
        put(role.as_table_mut().ok_or(format!("config.toml's [roles.{conv}] is not a table"))?);
    }
    Ok(doc.to_string())
}
/// Gives one conversation its longest life, in the owner's words («siempre» takes the limit away): written into config.toml, the page and the phones hear it, and what is already older goes at once.
pub fn set_keep(app: &Shared, project: &str, words: &str) -> Result<Value, String> {
    (project.is_empty() || app.conv(project) == project).then_some(()).ok_or(format!("unknown project '{project}'"))?;
    let conv = app.conv(project).to_string();
    let secs = parse_keep(words)?;
    let _writing = crate::projects::WRITING.lock().unwrap();
    let path = app.home.join("config.toml");
    let fail = |e: std::io::Error| format!("{}: {e}", path.display());
    let text = std::fs::read_to_string(&path).or_else(|e| if e.kind() == std::io::ErrorKind::NotFound { Ok(String::new()) } else { Err(e) }).map_err(fail)?;
    std::fs::write(&path, written(&text, app.cfg().project(&conv).is_some(), &conv, secs.map(|_| words.trim()))?).map_err(fail)?;
    app.reload_config(true);
    let now = keep_words(app, &conv);
    app.emit("chat-keep", json!({"project": conv, "keep": now, "keep_s": secs.unwrap_or(0)}));
    to_phones(app, &conv, json!({"v": 1, "keep_s": secs.unwrap_or(0)}));
    sweep(app, now_s());
    Ok(json!({"project": conv, "keep": now, "keep_s": secs.unwrap_or(0)}))
}
/// What a conversation's longest life is now, in the owner's words and in seconds (0: none).
fn keep_of(app: &Shared, project: &str) -> Value {
    let conv = app.conv(project);
    json!({"project": conv, "keep": keep_words(app, conv), "keep_s": keep_s(app, conv).unwrap_or(0)})
}
/// Every conversation with a longest life loses its rows older than it (their `at` is ISO UTC, so text order is time order); a conversation that lost any forgets its saved session too, so its bot does not remember what is gone. `now` is injected for the test.
pub fn sweep(app: &Shared, now: u64) {
    for conv in app.query("SELECT DISTINCT project FROM chat", [], |r| r.get::<_, String>(0)) {
        let Some(secs) = keep_s(app, &conv) else { continue };
        let Some(cut) = chrono::DateTime::from_timestamp(now.saturating_sub(secs) as i64, 0) else { continue };
        let before = cut.format("%Y-%m-%dT%H:%M:%SZ").to_string();
        let gone = app.db.lock().unwrap().execute("DELETE FROM chat WHERE project = ?1 AND at < ?2", rusqlite::params![conv, before]).unwrap_or(0);
        let _ = (gone > 0).then(|| (forget_session(app, &conv), app.emit("chat-trim", json!({"project": conv, "before": before}))));
    }
}
/// The minute sweep of every conversation's longest life.
pub fn start(app: Shared) {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(60)).await;
            sweep(&app, now_s());
        }
    });
}
pub fn routes() -> Router<Shared> {
    Router::new().route("/api/chat/clear", post(|State(app): State<Shared>, Json(b): Json<Value>| async move { clear(&app, &s(&b, "project")).map(Json).map_err(bad) })).route(
        "/api/chat/keep",
        get(|State(app): State<Shared>, Query(q): Query<std::collections::HashMap<String, String>>| async move { Json(keep_of(&app, q.get("project").map_or("", String::as_str))) })
            .post(|State(app): State<Shared>, Json(b): Json<Value>| async move { set_keep(&app, &s(&b, "project"), &s(&b, "keep")).map(Json).map_err(bad) }),
    )
}
/// `tirith chat clear|keep [<the owner's words>]`: on the conversation of the session that runs it (`TIRITH_PROJECT`), or `--project`'s. A lane never clears a chat: only a bot the owner talks with, on the owner's own words (its prompt says so).
pub fn cli(pos: &[&str], flag: Flag) -> Option<Result<(), String>> {
    let conv = || {
        let lane = std::env::var("TIRITH_LANE").is_ok_and(|l| !l.is_empty());
        let Some(()) = (!lane).then_some(()) else { return Err("a lane does not clear chats: only the bot the owner asked, in its own conversation".to_string()) };
        Ok(flag("--project").or_else(|| std::env::var("TIRITH_PROJECT").ok()).unwrap_or_default())
    };
    let say = |r: Result<Value, String>, what: &str| r.map(|v| println!("{what} ({}): {}", s(&v, "project"), Some(s(&v, "keep")).filter(|k| !k.is_empty()).unwrap_or_else(|| "siempre".into())));
    match pos {
        ["chat", "clear"] => Some(conv().and_then(|p| crate::client::call_json("POST", "/api/chat/clear", Some(json!({"project": p})))).map(|v| println!("chat cleared ({})", s(&v, "project")))),
        ["chat", "keep"] => Some(conv().and_then(|p| say(crate::client::call_json("GET", &format!("/api/chat/keep?project={p}"), None), "the chat lives"))),
        ["chat", "keep", words @ ..] => Some(conv().and_then(|p| say(crate::client::call_json("POST", "/api/chat/keep", Some(json!({"project": p, "keep": words.join(" ")}))), "the chat now lives"))),
        ["chat", ..] => Some(Err("use: tirith chat clear | tirith chat keep [<time, as 1h, 24 h, 2 días, una semana, siempre>]".into())),
        _ => None,
    }
}
#[cfg(test)]
#[path = "../checks/chat.rs"]
mod tests;
