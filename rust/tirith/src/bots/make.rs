//! A bot made whole: its skills, its routines' stamps, its card and its first words, from a template or a package.
use super::folder::{folder, skill_file};
use super::store::with;
use crate::app::{now_iso, Shared};
use crate::server::s;
use serde_json::{json, Value};

/// What a new bot is made of.
pub(super) struct Parts<'a> {
    pub name: &'a str,
    pub shape: &'a str,
    pub color: &'a str,
    pub routines: Vec<Value>,
    pub skills: Vec<(String, String)>,
    pub start: &'a str,
    pub origin: &'a str,
}

fn fresh() -> String {
    format!("{:06x}", rand::random::<u32>() >> 8)
}

fn write_skill(app: &Shared, key: &str, name: &str, text: &str) -> Result<(), String> {
    let file = skill_file(app, key, name);
    file.parent()
        .map_or(Ok(()), std::fs::create_dir_all)
        .and_then(|_| std::fs::write(&file, text))
        .map_err(|e| format!("{}: {e}", file.display()))
}

/// A new bot. Its skills and its routines' stamps are there before the bot is, so the scheduler never finds it without them: a stamp keeps a routine from running for a time already past today.
pub(super) fn make(app: &Shared, p: Parts) -> Result<Value, String> {
    let bot_id = fresh();
    let key = format!("bot-{bot_id}");
    let undo = |stamps: &[String]| {
        stamps.iter().for_each(|k| app.set_kv(k, None));
        let _ = std::fs::remove_dir_all(folder(app, &key));
    };
    let mut stamps = vec![];
    let mut routines = vec![];
    for r in &p.routines {
        let rid = format!("r{}", fresh());
        stamps.push(format!("routine:{bot_id}:{rid}"));
        app.set_kv(&stamps[stamps.len() - 1], Some(&now_iso()));
        let mut r = r.clone();
        r["id"] = json!(rid);
        routines.push(r);
    }
    for (name, text) in &p.skills {
        write_skill(app, &key, name, text).inspect_err(|_| undo(&stamps))?;
    }
    let bot = json!({"id": bot_id, "key": key, "name": p.name, "shape": p.shape, "color": p.color, "session": "chat", "access": "", "routes": [], "routines": routines});
    with(app, |bots| {
        bots.push(bot.clone());
        Ok(())
    })
    .inspect_err(|_| undo(&stamps))?;
    super::versioned(app, &key, p.origin);
    for r in &routines {
        app.chat(
            "routine",
            &format!("Created routine · {}", s(r, "title")),
            &key,
        );
    }
    if !p.start.is_empty() {
        app.chat("manager", p.start, &key);
    }
    Ok(bot)
}
