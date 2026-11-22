//! The versions a turn ran with: the git blob hash (`git hash-object`) of its memory.md, the bots' shared memory and each skill it read, as they were when it started.
use super::record::Turn;
use crate::app::Shared;
use serde_json::{Map, Value};
use std::path::PathBuf;

/// The folder a role's sessions run in, as `manager::spawn_chat` chooses it: a bot's own, else its conversation's below the role's.
pub(super) fn folder(app: &Shared, t: &Turn) -> PathBuf {
    let own = app
        .cfg()
        .factory
        .role::<String>(&t.role, "scope")
        .as_deref()
        == Some("own");
    crate::lab::seat(if own {
        app.home.join("manager").join(&t.role)
    } else {
        app.home.join(&t.role).join(&t.conv)
    })
}

/// The SKILL.md of skill `name` the turn read: a bot's own (its agent's skills folder links there), else the shared one.
pub(super) fn skill_file(app: &Shared, t: &Turn, name: &str) -> Option<PathBuf> {
    [
        folder(app, t).join(".agents/skills"),
        app.home.join("bots/skills"),
    ]
    .into_iter()
    .map(|d| d.join(name).join("SKILL.md"))
    .find(|p| p.is_file())
}

const MEMORY: [&str; 2] = ["memory.md", "bots/memory.md"];

/// Before a bot is handed the message of a new turn: its folder committed, so each version that turn starts with is in its history.
pub fn prepare(app: &Shared, conv: &str) {
    let due = super::live::idle(conv) && crate::bots::bot(app, conv).is_some();
    let _ = due.then(|| crate::bots::versioned(app, conv, "before a turn"));
}

/// As the turn starts: its memory, the shared one and every skill it could read (its own before a shared one of the same name).
pub(super) fn at_start(app: &Shared, t: &Turn) -> Map<String, Value> {
    let dir = folder(app, t);
    let mut files = vec![
        (MEMORY[0].to_string(), dir.join("memory.md")),
        (MEMORY[1].to_string(), app.home.join("bots/memory.md")),
    ];
    for skills in [app.home.join("bots/skills"), dir.join(".agents/skills")] {
        for e in std::fs::read_dir(&skills).into_iter().flatten().flatten() {
            let name = format!("skills/{}", e.file_name().to_string_lossy());
            files.push((name, e.path().join("SKILL.md")));
        }
    }
    super::hashes::hashes(files)
}

/// `{"memory.md": ..., "bots/memory.md": ..., "skills/<name>": ...}` as the turn found them, the skills only those it read (one it wrote meanwhile as it is now).
pub fn versions(app: &Shared, t: &Turn) -> Value {
    let mut out = Map::new();
    let mut later = vec![];
    for key in MEMORY {
        if let Some(hash) = t.began.get(key) {
            out.insert(key.into(), hash.clone());
        }
    }
    for name in &t.skills {
        let key = format!("skills/{name}");
        match t.began.get(&key) {
            Some(hash) => _ = out.insert(key, hash.clone()),
            None => later.extend(skill_file(app, t, name).map(|p| (key, p))),
        }
    }
    out.extend(super::hashes::hashes(later));
    Value::Object(out)
}
