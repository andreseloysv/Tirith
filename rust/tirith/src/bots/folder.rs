//! A bot's folder and skills: where its sessions run, where agents may write, and the skills a folder holds.
use crate::app::App;
use std::path::{Path as FsPath, PathBuf};
/// A bot's skills, in its folder: the agents read skills there (or, through a link, in their own .<command>/skills).
pub(super) const SKILLS: &str = ".agents/skills";
/// The memory of every bot, in Tirith's home: what holds for all of them (the owner's name, time zone, language).
pub(super) const SHARED: &str = "bots/memory.md";
/// A bot's folder, where its sessions run: memory.md, its working files and its skills.
pub fn folder(app: &App, key: &str) -> PathBuf {
    app.home.join("manager").join(key)
}
/// A bot's own skill `name`: its SKILL.md.
pub fn skill_file(app: &App, key: &str, name: &str) -> PathBuf {
    folder(app, key).join(SKILLS).join(name).join("SKILL.md")
}
/// The one part of Tirith's home besides the lanes where agents write: the bots' folders and the shared skills. In a bot's folder a hidden folder is written only as a skills folder: an agent's settings there could turn Tirith's hooks off.
pub fn writable(home: &FsPath, path: &FsPath) -> bool {
    let parts: Vec<String> = path
        .strip_prefix(home.join("manager"))
        .map(|r| r.iter().map(|c| c.to_string_lossy().into_owned()).collect())
        .unwrap_or_default();
    let own = parts.first().is_some_and(|b| {
        b.starts_with("bot-")
            || crate::factory::current()
                .role::<String>(b, "scope")
                .as_deref()
                == Some("own")
    }) && parts
        .windows(2)
        .all(|w| !w[0].starts_with('.') || w[1] == "skills");
    own || path.starts_with(home.join("bots/skills"))
}
/// The skills of a folder of skills, by name with the text of their SKILL.md; `links` false leaves out the shared ones linked into a bot's.
pub(super) fn skills(dir: &FsPath, links: bool) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| links || !e.file_type().is_ok_and(|t| t.is_symlink()))
        .filter_map(|e| {
            Some((
                e.file_name().into_string().ok()?,
                std::fs::read_to_string(e.path().join("SKILL.md")).ok()?,
            ))
        })
        .collect();
    out.sort();
    out
}
