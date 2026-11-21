//! The versions a turn ran with: the git blob hash (`git hash-object`) of its memory.md, the bots' shared memory and each skill it read.
use super::record::Turn;
use crate::app::Shared;
use serde_json::{Map, Value};
use std::path::{Path, PathBuf};

/// The folder a role's sessions run in, as `manager::spawn_chat` chooses it: a bot's own, else its conversation's below the role's.
pub(super) fn folder(app: &Shared, t: &Turn) -> PathBuf {
    let own = app
        .cfg()
        .factory
        .role::<String>(&t.role, "scope")
        .as_deref()
        == Some("own");
    if own {
        app.home.join("manager").join(&t.role)
    } else {
        app.home.join(&t.role).join(&t.conv)
    }
}

/// The git blob hash of a file, if it exists.
fn hash(path: &Path) -> Option<String> {
    path.is_file().then_some(())?;
    let out = std::process::Command::new("git")
        .arg("hash-object")
        .arg(path)
        .output()
        .ok()?;
    let id = String::from_utf8_lossy(&out.stdout).trim().to_string();
    (out.status.success() && !id.is_empty()).then_some(id)
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

/// `{"memory.md": …, "bots/memory.md": …, "skills/<name>": …}`, each file that exists.
pub fn versions(app: &Shared, t: &Turn) -> Value {
    let dir = folder(app, t);
    let mut files = vec![
        ("memory.md".to_string(), dir.join("memory.md")),
        ("bots/memory.md".into(), app.home.join("bots/memory.md")),
    ];
    for name in &t.skills {
        files.extend(skill_file(app, t, name).map(|p| (format!("skills/{name}"), p)));
    }
    let hashed = files
        .into_iter()
        .filter_map(|(k, p)| hash(&p).map(|h| (k, Value::String(h))));
    Value::Object(hashed.collect::<Map<String, Value>>())
}
