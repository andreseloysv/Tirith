//! The page's Library: a bot's memory, its own skills and the shared ones, drafts included, read and changed.
use super::drafts::{drafts_dir, is_draft, sweep};
use super::folder::{folder, skills, SHARED, SKILLS};
use super::list::key_for;
use crate::app::App;
use crate::server::s;
use serde_json::{json, Value};
use std::path::PathBuf;
/// The page's Library: the shared memory of every bot, the bot's own memory, its own skills and the shared ones (each draft marked `draft`, kept in a drafts folder no agent loads), and the factory's skill template (a new skill starts from it, and its section titles are the ones a skill is marked for lacking).
pub(crate) fn library(app: &App, id: &str) -> Result<Value, String> {
    let key = key_for(app, id).ok_or(format!("no bot '{id}'"))?;
    sweep(app, &key);
    let list = |v: Vec<(String, String)>, shared: bool, draft: bool| {
        v.into_iter().map(move |(name, text)| {
            json!({"name": name, "text": text, "shared": shared, "draft": draft})
        })
    };
    let (own, common) = (skills_dir(app, &key, false), skills_dir(app, &key, true));
    let all: Vec<Value> = list(skills(&own, false), false, false)
        .chain(list(
            skills(&drafts_dir(app, &key, false), false),
            false,
            true,
        ))
        .chain(list(skills(&common, true), true, false))
        .chain(list(
            skills(&drafts_dir(app, &key, true), false),
            true,
            true,
        ))
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
/// The folder of skills of the bot (`shared`: every bot's).
fn skills_dir(app: &App, key: &str, shared: bool) -> PathBuf {
    if shared {
        app.home.join("bots/skills")
    } else {
        folder(app, key).join(SKILLS)
    }
}
/// A change from the Library: the `memory` (`shared`: every bot's), or the skill `name` (`shared`: every bot's), gets `text`, or goes with `delete`. A text with `draft: true` is kept as a draft, where no agent loads it; the text of a draft without it activates it, which an agent's call may not do.
pub(crate) fn learn(app: &App, id: &str, b: &Value, agent: bool) -> Result<Value, String> {
    let (key, name) = (
        key_for(app, id).ok_or(format!("no bot '{id}'"))?,
        s(b, "name"),
    );
    let (memory, shared) = (b["memory"] == true, b["shared"] == true);
    (memory || named(&name))
        .then_some(())
        .ok_or("a skill's name is lowercase letters, digits and dashes")?;
    let draft = !memory && is_draft(&s(b, "text"));
    let (skill, held) = (
        skills_dir(app, &key, shared).join(&name),
        drafts_dir(app, &key, shared).join(&name),
    );
    if agent && !memory && !draft && b["delete"] != true && held.exists() {
        return Err("only the owner activates a draft".into());
    }
    let dir = match (memory, shared, draft) {
        (true, true, _) => app.home.join("bots"),
        (true, false, _) => folder(app, &key),
        (_, _, true) => held.clone(),
        _ => skill.clone(),
    };
    let file = dir.join(if memory { "memory.md" } else { "SKILL.md" });
    let done = match (b["delete"] == true, memory) {
        (true, true) => std::fs::remove_file(&file),
        (true, false) => [&skill, &held]
            .iter()
            .filter(|d| std::fs::remove_dir_all(d).is_ok())
            .count()
            .gt(&0)
            .then_some(())
            .ok_or_else(|| std::io::ErrorKind::NotFound.into()),
        _ => std::fs::create_dir_all(&dir).and_then(|_| std::fs::write(&file, s(b, "text"))),
    };
    done.map_err(|e| format!("{}: {e}", file.display()))?;
    let _ = (!memory && !draft && b["delete"] != true).then(|| std::fs::remove_dir_all(&held));
    super::versioned(app, &key, "library");
    app.emit("library", json!({"bot": id}));
    Ok(json!({"ok": true}))
}
