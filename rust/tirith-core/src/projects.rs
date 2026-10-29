//! Projects are folders (`projects.py`, `docs/CONTRACT-sessions.md`).
//!
//! Registered projects live in `config.toml`; folders the owner never registered but that lanes, stages or
//! rooms point at are derived into *auto* projects and persisted in `projects-auto.json` so their ids stay
//! stable. Ported: discovery and matching. Not ported yet (projects lane): `config.toml` editing
//! (`add_project`, `remove_project`, `set_project_guard`) and sessions.

use crate::home::Config;
use crate::pytext::expanduser;
use crate::store::{read_json, write_json_atomic};
use serde_json::{json, Value};
use sha1::{Digest, Sha1};
use std::collections::HashSet;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

pub const PALETTE: [&str; 8] = ["#2f7d5b", "#3b6fb6", "#b5654a", "#8a5fb0", "#b08d2f", "#2f8a94", "#a84e72", "#6b7f3a"];
pub const AUTO_FILE: &str = "projects-auto.json";
const TEMP_ROOTS: [&str; 6] = ["/tmp", "/private/tmp", "/var/folders", "/private/var/folders", "/var/tmp", "/private/var/tmp"];

/// Lower-case, runs of anything but `a-z0-9` become `-`, cut to `limit`, no edge dashes.
pub fn slug(text: &str, fallback: &str, limit: usize) -> String {
    let mut out = String::new();
    let mut dash = false;
    for c in text.to_lowercase().chars() {
        if c.is_ascii_lowercase() || c.is_ascii_digit() {
            if dash && !out.is_empty() {
                out.push('-');
            }
            dash = false;
            out.push(c);
        } else {
            dash = true;
        }
    }
    let cut: String = out.chars().take(limit).collect();
    let cut = cut.trim_matches('-');
    if cut.is_empty() { fallback.to_string() } else { cut.to_string() }
}

/// The palette colour a key maps to (the same pick as the Python collector).
pub fn color_for(key: &str) -> &'static str {
    let digest = Sha1::digest(key.as_bytes());
    PALETTE[usize::from(digest[digest.len() - 1]) % PALETTE.len()] // 8 divides 256: the last byte decides
}

/// Absolute, symlink-free path (works for folders that do not exist yet), like `Path.resolve()`.
pub fn real(path: &Path) -> PathBuf {
    let path = expanduser(&path.to_string_lossy());
    let absolute = if path.is_absolute() { path } else { std::env::current_dir().unwrap_or_default().join(path) };
    let mut resolved = PathBuf::new();
    for part in absolute.components() {
        match part {
            Component::ParentDir => {
                resolved.pop();
            }
            Component::CurDir => {}
            Component::Normal(name) => {
                resolved.push(name);
                if let Ok(target) = std::fs::canonicalize(&resolved) {
                    resolved = target;
                }
            }
            other => resolved.push(other.as_os_str()),
        }
    }
    resolved
}

fn git_output(folder: &Path, args: &[&str]) -> Option<String> {
    let out = Command::new("git").arg("-C").arg(folder).args(args).output().ok()?;
    let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
    (out.status.success() && !text.is_empty()).then_some(text)
}

pub fn git_toplevel(folder: &Path) -> Option<PathBuf> {
    if !folder.is_dir() {
        return None;
    }
    git_output(folder, &["rev-parse", "--show-toplevel"]).map(|top| real(Path::new(&top)))
}

/// `path` moved into the repository's main worktree when it lies in a linked git worktree.
fn in_main_worktree(path: &Path) -> PathBuf {
    let folder = if path.is_dir() { path } else { path.parent().unwrap_or(path) };
    let Some(top) = git_toplevel(folder) else { return path.to_path_buf() };
    let Some(common) = git_output(&top, &["rev-parse", "--path-format=absolute", "--git-common-dir"]) else {
        return path.to_path_buf();
    };
    let common = real(Path::new(&common));
    if common.file_name().is_none_or(|n| n != ".git") || common.parent() == Some(top.as_path()) {
        return path.to_path_buf();
    }
    match (common.parent(), path.strip_prefix(&top)) {
        (Some(main), Ok(rest)) => main.join(rest),
        _ => path.to_path_buf(),
    }
}

/// Absolute path, with linked git worktrees (agent lanes) mapped onto their main repository.
pub fn canonical(folder: &Path) -> PathBuf {
    in_main_worktree(&real(folder))
}

/// The folder a project derived from `folder` would use: git top-level, else itself.
pub fn project_root(folder: &Path) -> PathBuf {
    let folder = real(folder);
    git_toplevel(&folder).unwrap_or(folder)
}

/// A folder that lives in a temporary directory (`/tmp`, `mktemp`, test folders): never an auto project.
pub fn is_temp_folder(folder: &Path) -> bool {
    let temp = std::env::temp_dir();
    let mut roots: Vec<String> = TEMP_ROOTS.iter().map(|r| r.to_string()).collect();
    roots.push(temp.to_string_lossy().into_owned());
    roots.push(real(&temp).to_string_lossy().into_owned());
    [folder.to_string_lossy().into_owned(), real(folder).to_string_lossy().into_owned()].iter().any(|path| {
        roots.iter().any(|root| {
            let root = root.trim_end_matches('/');
            !root.is_empty() && (path == root || path.starts_with(&format!("{root}/")))
        })
    })
}

/// One project: registered (`auto == false`) or derived from a folder.
#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    pub id: String,
    pub name: String,
    pub folder: Option<PathBuf>,
    pub color: String,
    pub auto: bool,
}

/// `projects-auto.json`: the derived projects and the folders the owner removed.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Auto {
    pub projects: Vec<Value>,
    pub ignored: Vec<String>,
}

fn field(row: &Value, key: &str) -> String {
    match row.get(key) {
        Some(Value::String(s)) => s.clone(),
        Some(other) => other.to_string(),
        None => String::new(),
    }
}

fn truthy_field(row: &Value, key: &str) -> bool {
    row.get(key).is_some_and(|v| match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::String(s) => !s.is_empty(),
        Value::Number(n) => n.as_f64().is_some_and(|f| f != 0.0),
        Value::Array(a) => !a.is_empty(),
        Value::Object(o) => !o.is_empty(),
    })
}

pub fn load_auto(home: &Path) -> Auto {
    let data = read_json(&home.join(AUTO_FILE)).unwrap_or(Value::Null);
    let rows = data.get("projects").and_then(Value::as_array).cloned().unwrap_or_default();
    let projects = rows
        .into_iter()
        .filter(|p| p.is_object() && truthy_field(p, "id") && truthy_field(p, "folder"))
        .filter(|p| !is_temp_folder(Path::new(&field(p, "folder"))))
        .collect();
    let ignored = data
        .get("ignored")
        .and_then(Value::as_array)
        .map(|a| a.iter().map(|v| v.as_str().map_or_else(|| v.to_string(), String::from)).collect())
        .unwrap_or_default();
    Auto { projects, ignored }
}

pub fn save_auto(home: &Path, auto: &Auto) -> std::io::Result<()> {
    let payload = json!({"projects": auto.projects, "ignored": auto.ignored});
    write_json_atomic(&home.join(AUTO_FILE), &payload, false)
}

/// Registered projects first (config order), then the auto rows whose id is not taken.
pub fn entries(config: &Config, auto: &Auto) -> Vec<Entry> {
    let mut out: Vec<Entry> = config
        .projects
        .iter()
        .map(|p| Entry {
            id: p.id.clone(),
            name: p.name.clone(),
            folder: p.repo.as_deref().map(real),
            color: p.color.clone(),
            auto: false,
        })
        .collect();
    let seen: HashSet<String> = out.iter().map(|e| e.id.clone()).collect();
    for row in &auto.projects {
        let id = field(row, "id");
        if seen.contains(&id) {
            continue;
        }
        let name = if truthy_field(row, "name") { field(row, "name") } else { id.clone() };
        let color = if truthy_field(row, "color") { field(row, "color") } else { color_for(&id).to_string() };
        out.push(Entry { name, folder: Some(real(Path::new(&field(row, "folder")))), color, auto: true, id });
    }
    out
}

/// The project whose folder is the longest prefix of `folder`.
pub fn match_entry<'a>(folder: &Path, items: &'a [Entry]) -> Option<&'a Entry> {
    if folder.as_os_str().is_empty() {
        return None;
    }
    let target = canonical(folder);
    let mut best: Option<&Entry> = None;
    for item in items {
        let Some(root) = item.folder.as_deref() else { continue };
        if target.starts_with(root) && best.is_none_or(|b| root.components().count() > b.folder.as_deref().map_or(0, |f| f.components().count())) {
            best = Some(item);
        }
    }
    best
}

pub fn unique_id(wanted: &str, taken: &HashSet<String>) -> String {
    let mut candidate = wanted.to_string();
    let mut number = 2;
    while taken.contains(&candidate) {
        candidate = format!("{wanted}-{number}");
        number += 1;
    }
    candidate
}

/// Register auto projects for folders outside every known project; returns all entries.
pub fn discover(home: &Path, config: &Config, folders: &[PathBuf]) -> Vec<Entry> {
    let mut auto = load_auto(home);
    let mut items = entries(config, &auto);
    let mut changed = false;
    for folder in folders.iter().filter(|f| !f.as_os_str().is_empty()) {
        if !real(folder).exists() || match_entry(folder, &items).is_some() {
            continue;
        }
        let root = project_root(folder);
        if is_temp_folder(folder) || is_temp_folder(&root) {
            continue;
        }
        let root_text = root.to_string_lossy().into_owned();
        if auto.ignored.contains(&root_text) || match_entry(&root, &items).is_some() {
            continue;
        }
        let name = root.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        let taken: HashSet<String> = items.iter().map(|e| e.id.clone()).collect();
        let id = unique_id(&slug(&name, "project", 40), &taken);
        let color = color_for(&id).to_string();
        let name = if name.is_empty() { id.clone() } else { name };
        auto.projects.push(json!({"id": id, "name": name, "folder": root_text, "color": color}));
        items.push(Entry { id, name, folder: Some(root), color, auto: true });
        changed = true;
    }
    if changed {
        let _ = save_auto(home, &auto);
    }
    items
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::home::ProjectConfig;
    use std::fs;

    fn project(id: &str, repo: &Path) -> ProjectConfig {
        ProjectConfig {
            id: id.into(),
            name: id.to_uppercase(),
            repo: Some(repo.to_path_buf()),
            color: "#123456".into(),
            guard_words: None,
            architecture: None,
            guard: true,
            cargo_target_dir: None,
        }
    }

    #[test]
    fn slug_and_color_follow_python() {
        assert_eq!(slug("My  Cool_Project!!", "project", 40), "my-cool-project");
        assert_eq!(slug("///", "project", 40), "project");
        assert_eq!(slug(&"a".repeat(60), "project", 40).len(), 40);
        assert!(PALETTE.contains(&color_for("tirith")));
        assert_eq!(color_for("tirith"), color_for("tirith"));
    }

    #[test]
    fn real_resolves_missing_tails_lexically() {
        let dir = tempfile::tempdir().unwrap();
        let base = real(dir.path());
        assert_eq!(real(&dir.path().join("a/../b/./c")), base.join("b/c"));
    }

    #[test]
    fn entries_put_registered_first_and_skip_taken_auto_ids() {
        let dir = tempfile::tempdir().unwrap();
        let config = Config { projects: vec![project("one", dir.path())], ..Config::default() };
        let auto = Auto {
            projects: vec![json!({"id": "one", "folder": "/x"}), json!({"id": "two", "folder": "/y", "name": "Two"})],
            ignored: vec![],
        };
        let rows = entries(&config, &auto);
        assert_eq!(rows.iter().map(|e| (e.id.as_str(), e.auto)).collect::<Vec<_>>(), [("one", false), ("two", true)]);
        assert_eq!(rows[1].color, color_for("two"));
    }

    #[test]
    fn match_prefers_the_longest_folder() {
        let dir = tempfile::tempdir().unwrap();
        let outer = real(dir.path());
        let inner = outer.join("inner");
        fs::create_dir_all(&inner).unwrap();
        let items = vec![
            Entry { id: "outer".into(), name: "o".into(), folder: Some(outer.clone()), color: "#000000".into(), auto: false },
            Entry { id: "inner".into(), name: "i".into(), folder: Some(inner.clone()), color: "#000000".into(), auto: false },
            Entry { id: "nofolder".into(), name: "n".into(), folder: None, color: "#000000".into(), auto: false },
        ];
        assert_eq!(match_entry(&inner.join("deep"), &items).unwrap().id, "inner");
        assert_eq!(match_entry(&outer.join("elsewhere"), &items).unwrap().id, "outer");
        assert!(match_entry(Path::new("/definitely/not/here"), &items).is_none());
    }

    #[test]
    fn temp_folders_are_never_auto_projects() {
        let dir = tempfile::tempdir().unwrap();
        assert!(is_temp_folder(dir.path()));
        assert!(!is_temp_folder(Path::new("/Users/someone/code")));
        let home = tempfile::tempdir().unwrap();
        let items = discover(home.path(), &Config::default(), &[dir.path().to_path_buf()]);
        assert!(items.is_empty());
        assert!(!home.path().join(AUTO_FILE).exists());
    }

    #[test]
    fn unique_id_counts_up() {
        let taken: HashSet<String> = ["app".to_string(), "app-2".to_string()].into();
        assert_eq!(unique_id("app", &taken), "app-3");
        assert_eq!(unique_id("new", &taken), "new");
    }

    #[test]
    fn load_auto_drops_rows_without_id_or_folder() {
        let home = tempfile::tempdir().unwrap();
        let rows = json!({"projects": [{"id": "a"}, {"folder": "/f"}, {"id": "ok", "folder": "/Users/z/ok"}], "ignored": ["/q"]});
        fs::write(home.path().join(AUTO_FILE), rows.to_string()).unwrap();
        let auto = load_auto(home.path());
        assert_eq!(auto.projects.len(), 1);
        assert_eq!(auto.ignored, ["/q"]);
    }
}
