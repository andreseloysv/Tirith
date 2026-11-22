//! What a bot is handed at the start of every turn and routine, and the skill links laid before it.
use super::folder::{folder, skills, SHARED, SKILLS};
use super::list::bot;
use super::notes::numbered_file;
use crate::app::Shared;
use std::path::Path as FsPath;
/// Links to removed shared skills go, and every shared skill is linked into the bot's (a skill of its own with the same name wins).
fn link_shared(dir: &FsPath, common: &FsPath) {
    let _ = std::fs::create_dir_all(dir);
    let gone = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| {
            e.file_type().is_ok_and(|t| t.is_symlink()) && !e.path().join("SKILL.md").exists()
        });
    gone.for_each(|e| drop(std::fs::remove_file(e.path())));
    skills(common, true).iter().for_each(|(name, _)| {
        drop(std::os::unix::fs::symlink(
            common.join(name),
            dir.join(name),
        ))
    });
}
/// The skills folder of the bots' agent (.<its command>/skills) is linked to the bot's skills.
fn link_agent(app: &Shared, key: &str) {
    let agent = app
        .cfg()
        .factory
        .command(&crate::models::role(app, key).agent)
        .and_then(|c| {
            FsPath::new(&c)
                .file_name()
                .map(|n| folder(app, key).join(format!(".{}", n.to_string_lossy())))
        });
    agent.into_iter().for_each(|d| {
        drop(
            std::fs::create_dir_all(&d)
                .and_then(|_| std::os::unix::fs::symlink(format!("../{SKILLS}"), d.join("skills"))),
        )
    });
}
/// The skills' names, or "none".
fn names(v: Vec<(String, String)>) -> String {
    Some(v.into_iter().map(|s| s.0).collect::<Vec<_>>().join(", "))
        .filter(|n| !n.is_empty())
        .unwrap_or("none".into())
}
/// A memory.md as it is now (the owner may have changed it), its notes numbered with `tag` first, up to 12000 characters.
fn memory(file: &FsPath, tag: char) -> String {
    let memory = numbered_file(file, tag)
        .chars()
        .take(12000)
        .collect::<String>();
    if memory.trim().is_empty() {
        "(empty)".into()
    } else {
        memory.trim().into()
    }
}
/// What a bot is handed at the start of every turn and routine: the shared memory of every bot, then its own (both as they are now, their notes numbered), and the names of its skills. The shared skills are linked into its skills first, links to removed ones go, and the skills folder of the bots' agent is linked to its skills.
pub fn context(app: &Shared, key: &str) -> Option<String> {
    bot(app, key)?;
    let (dir, common) = (folder(app, key).join(SKILLS), app.home.join("bots/skills"));
    super::drafts::sweep(app, key); // a draft written where an agent loads skills goes to the drafts folder
    link_shared(&dir, &common);
    link_agent(app, key);
    let shared = memory(&app.home.join(SHARED), 's');
    let memory = memory(&folder(app, key).join("memory.md"), 'm');
    Some(format!("[tirith] Shared memory.md as it is now:\n{shared}\n[tirith] Your memory.md as it is now:\n{memory}\n[tirith] Your skills: {}. Shared skills: {}.", names(skills(&dir, false)), names(skills(&common, true))))
}
