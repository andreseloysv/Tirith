//! The page's Library: a bot's memory, its own skills and the shared ones, read and changed.
use super::folder::{folder, skills, SHARED, SKILLS};
use super::list::key_for;
use crate::app::App;
use crate::server::s;
use serde_json::{json, Value};
use std::path::PathBuf;
/// The page's Library: the shared memory of every bot, the bot's own memory, its own skills and the shared ones, and the factory's skill template (a new skill starts from it, and its section titles are the ones a skill is marked for lacking).
pub(super) fn library(app: &App, id: &str) -> Result<Value, String> {
    let key = key_for(app, id).ok_or(format!("no bot '{id}'"))?;
    let list = |v: Vec<(String, String)>, shared: bool| {
        v.into_iter()
            .map(move |(name, text)| json!({"name": name, "text": text, "shared": shared}))
    };
    let all: Vec<Value> = list(skills(&folder(app, &key).join(SKILLS), false), false)
        .chain(list(skills(&app.home.join("bots/skills"), true), true))
        .collect();
    Ok(
        json!({"memory": read(&folder(app, &key).join("memory.md")), "shared_memory": read(&app.home.join(SHARED)), "skills": all, "template": app.cfg().factory.prompt("skill")}),
    )
}
fn read(file: &std::path::Path) -> String {
    std::fs::read_to_string(file).unwrap_or_default()
}
/// A skill's name is lowercase letters, digits and dashes, up to 64.
fn named(name: &str) -> bool {
    name.len() <= 64
        && name.starts_with(|c: char| c.is_ascii_lowercase())
        && name
            .bytes()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-')
}
/// The folder the change lands in: the bot's own for its memory (`shared`: the bots' folder, for the shared memory), else the skill's (`shared`: every bot's).
fn target(app: &App, key: &str, name: &str, b: &Value) -> PathBuf {
    let skill = (if b["shared"] == true {
        app.home.join("bots/skills")
    } else {
        folder(app, key).join(SKILLS)
    })
    .join(name);
    if b["memory"] == true && b["shared"] == true {
        app.home.join("bots")
    } else if b["memory"] == true {
        folder(app, key)
    } else {
        skill
    }
}
/// A change from the Library: the `memory` (`shared`: every bot's), or the skill `name` (`shared`: every bot's), gets `text`, or goes with `delete`.
pub(super) fn learn(app: &App, id: &str, b: &Value) -> Result<Value, String> {
    let (key, name) = (
        key_for(app, id).ok_or(format!("no bot '{id}'"))?,
        s(b, "name"),
    );
    let dir = target(app, &key, &name, b);
    (b["memory"] == true || named(&name))
        .then_some(())
        .ok_or("a skill's name is lowercase letters, digits and dashes")?;
    let file = dir.join(if b["memory"] == true {
        "memory.md"
    } else {
        "SKILL.md"
    });
    let done = match (b["delete"] == true, b["memory"] == true) {
        (true, true) => std::fs::remove_file(&file),
        (true, false) => std::fs::remove_dir_all(&dir),
        _ => std::fs::create_dir_all(&dir).and_then(|_| std::fs::write(&file, s(b, "text"))),
    };
    done.map_err(|e| format!("{}: {e}", file.display()))?;
    super::versioned(app, &key, "library");
    app.emit("library", json!({"bot": id}));
    Ok(json!({"ok": true}))
}
