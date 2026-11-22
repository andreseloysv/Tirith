//! A bot made from a template: its card, its routine, its skill and its first words.
use super::folder::{folder, skill_file};
use super::store::with;
use super::template::{templates, Template};
use crate::app::{bad, now_iso, Shared, R};
use crate::server::s;
use axum::extract::State;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::{json, Value};

fn fresh() -> String {
    format!("{:06x}", rand::random::<u32>() >> 8)
}

fn write_skill(app: &Shared, key: &str, t: &Template) -> Result<(), String> {
    let file = skill_file(app, key, &t.id);
    file.parent()
        .map_or(Ok(()), std::fs::create_dir_all)
        .and_then(|_| std::fs::write(&file, &t.skill))
        .map_err(|e| format!("{}: {e}", file.display()))
}

/// A new bot from the template `id`. Its skill and its routine's stamp are there before the bot is, so the scheduler never finds it without them: the stamp keeps it from running for a time already past today.
pub(super) fn create(app: &Shared, id: &str) -> Result<Value, String> {
    let t = templates(app)?
        .into_iter()
        .find(|t| t.id == id)
        .ok_or(format!("no bot template '{id}'"))?;
    let (bot_id, rid) = (fresh(), format!("r{}", fresh()));
    let key = format!("bot-{bot_id}");
    write_skill(app, &key, &t)?;
    let stamp = format!("routine:{bot_id}:{rid}");
    app.set_kv(&stamp, Some(&now_iso()));
    let mut routine = t.routine.clone();
    routine["id"] = json!(rid);
    let bot = json!({"id": bot_id, "key": key, "name": t.name, "shape": t.shape, "color": t.color, "session": "chat", "access": "", "routes": [], "routines": [routine]});
    with(app, |bots| {
        bots.push(bot.clone());
        Ok(())
    })
    .inspect_err(|_| {
        app.set_kv(&stamp, None);
        let _ = std::fs::remove_dir_all(folder(app, &key));
    })?;
    super::versioned(app, &key, "template");
    let title = s(&t.routine, "title");
    app.chat("routine", &format!("Created routine · {title}"), &key);
    if !t.start.is_empty() {
        app.chat("manager", &t.start, &key);
    }
    Ok(bot)
}

async fn list(State(app): State<Shared>) -> R {
    let all = templates(&app).map_err(bad)?;
    Ok(Json(json!(all
        .iter()
        .map(Template::shown)
        .collect::<Vec<_>>())))
}

async fn make(State(app): State<Shared>, Json(b): Json<Value>) -> R {
    create(&app, &s(&b, "template")).map(Json).map_err(bad)
}

pub(super) fn routes() -> Router<Shared> {
    Router::new()
        .route("/api/bots/templates", get(list))
        .route("/api/bots/from-template", post(make))
}
