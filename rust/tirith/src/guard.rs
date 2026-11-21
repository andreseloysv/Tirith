//! The commit guard: git hooks in every project that refuse a commit containing one of the owner's forbidden words (from config.toml only; `guard = false` opts a project out).
use crate::app::Shared;
use crate::config::{home_dir, Project};
use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{http::StatusCode, Json, Router};
use serde_json::{json, Value};
use std::path::PathBuf;
const MARKER: &str = "FORBIDDEN_WORDS_GUARD";
const HOOKS: [&str; 2] = ["pre-commit", "commit-msg"];
const HEAD: &str = "#!/bin/bash\n# FORBIDDEN_WORDS_GUARD\n# {what}\n# Installed by Tirith; words: {words}\nset -euo pipefail\nexport TIRITH_GUARD_RE='{re}'\n";
const PRE_COMMIT: &str = include_str!("../assets/pre-commit.sh");
const COMMIT_MSG: &str = include_str!("../assets/commit-msg.sh");
/// The repository's own hook, kept next to ours as `<hook>.local` when the guard was installed, still runs.
const LOCAL: &str = include_str!("../assets/local-hook.sh");
pub fn words(app: &Shared, project: &Project) -> Vec<String> {
    let cfg = app.cfg();
    let entry = cfg.raw.get("projects").and_then(|p| p.as_array()).and_then(|ps| ps.iter().find(|p| p.get("id").and_then(|i| i.as_str()) == Some(project.id.as_str())).cloned()).unwrap_or(toml::Value::Table(Default::default()));
    let Some(()) = (entry.get("guard").and_then(|g| g.as_bool()) != Some(false)).then_some(()) else { return vec![] };
    let list = |v: Option<&toml::Value>| -> Option<Vec<String>> { v?.clone().try_into().ok() };
    list(entry.get("guard_words")).or_else(|| list(cfg.raw.get("guard").and_then(|g| g.get("words")))).unwrap_or_default().into_iter().filter(|w| !w.trim().is_empty()).collect()
}
/// The repo's common git dir, by reading `.git` (a folder, or a file pointing into a worktree's git dir).
pub fn git_dir(repo: &std::path::Path) -> Option<PathBuf> {
    let dot = repo.join(".git");
    let Some(()) = (!dot.is_dir()).then_some(()) else { return Some(dot) };
    let text = std::fs::read_to_string(&dot).ok()?;
    let dir = repo.join(text.strip_prefix("gitdir:")?.trim());
    Some(std::fs::read_to_string(dir.join("commondir")).map_or_else(|_| dir.clone(), |c| dir.join(c.trim())))
}
fn render(hook: &str, words: &[String]) -> String {
    let (what, body) = match hook {
        "pre-commit" => ("Blocks the commit when a staged file name or an added line contains a forbidden word (whole word, any case).", PRE_COMMIT),
        _ => ("Blocks the commit when the message contains a forbidden word (whole word, any case); # lines are ignored.", COMMIT_MSG),
    };
    let re = words.join("|").replace('\'', "'\\''");
    HEAD.replace("{what}", what).replace("{words}", &words.join(",")).replace("{re}", &re) + body + LOCAL
}
/// Ok when the project's guard is installed and intact (or the project opts out); the reason otherwise. The scheduler calls it before it starts a lane of the project: a few small file reads, no git process.
pub fn intact(app: &Shared, project: &Project) -> Result<(), String> {
    let words = words(app, project);
    let Some(words) = (!words.is_empty()).then_some(words) else { return Ok(()) };
    let git = git_dir(&project.repo()).ok_or(format!("commit guard: {} is not a git repo", project.repo().display()))?;
    let fix = format!("(tirith guard install {})", project.id);
    for hook in HOOKS {
        let text = std::fs::read_to_string(git.join("hooks").join(hook)).map_err(|_| format!("commit guard: the {hook} hook is missing {fix}"))?;
        (text.contains(MARKER)).then_some(()).ok_or_else(|| format!("commit guard: the {hook} hook is not Tirith's; merge the guard into it by hand"))?;
        (text == render(hook, &words)).then_some(()).ok_or_else(|| format!("commit guard: the {hook} hook is outdated or has other words than config.toml {fix}"))?;
    }
    // A `core.hooksPath` in the repo's or the user's git config makes git skip the guard.
    let skips = |f: &PathBuf| std::fs::read_to_string(f).is_ok_and(|t| t.lines().any(|l| l.trim().to_lowercase().replace(' ', "").starts_with("hookspath=")));
    [git.join("config"), home_dir().join(".gitconfig")].into_iter().find(skips).map_or(Ok(()), |f| Err(format!("commit guard: core.hooksPath in {} makes git skip the guard", f.display())))
}
/// Sets or clears the immutable flag (macOS `uchg`): installed hooks cannot be changed or deleted, by agents or anyone, until the installer clears it. Elsewhere a no-op.
#[cfg(target_os = "macos")]
fn immutable(path: &std::path::Path, on: bool) {
    use std::os::macos::fs::MetadataExt;
    std::fs::metadata(path).ok().zip(std::ffi::CString::new(path.as_os_str().as_encoded_bytes()).ok()).into_iter().for_each(|(m, c)| {
        unsafe { libc::chflags(c.as_ptr(), if on { m.st_flags() | libc::UF_IMMUTABLE } else { m.st_flags() & !libc::UF_IMMUTABLE }) };
    });
}
#[cfg(not(target_os = "macos"))]
fn immutable(_path: &std::path::Path, _on: bool) {}
/// Writes both hooks (mode 755) where they are missing or ours (an older guard is replaced); never over a hook that is not ours.
pub fn install(app: &Shared, project: &Project) -> Result<(), String> {
    let words = words(app, project);
    let Some(words) = (!words.is_empty()).then_some(words) else { return Ok(()) };
    let dir = git_dir(&project.repo()).ok_or(format!("{} is not a git repo", project.repo().display()))?.join("hooks");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let ours = |p: &std::path::Path| std::fs::read_to_string(p).is_ok_and(|t| t.contains(MARKER));
    for hook in HOOKS {
        let path = dir.join(hook);
        if path.exists() && !ours(&path) {
            // The repository's own hook keeps running after ours, as <hook>.local.
            let local = dir.join(format!("{hook}.local"));
            (!(local.exists())).then_some(()).ok_or_else(|| format!("{} is not Tirith's hook and {} exists: merge them by hand", path.display(), local.display()))?;
            immutable(&path, false);
            std::fs::rename(&path, &local).map_err(|e| format!("{}: {e}", path.display()))?;
        }
    }
    for hook in HOOKS {
        let path = dir.join(hook);
        immutable(&path, false);
        std::fs::write(&path, render(hook, &words)).map_err(|e| format!("{}: {e}", path.display()))?;
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).map_err(|e| e.to_string())?;
        immutable(&path, true);
    }
    intact(app, project)
}
/// Installs the guard where it is missing (never over a foreign hook); an event for each project that needs attention.
pub fn start(app: Shared) {
    tokio::task::spawn_blocking(move || {
        for p in app.cfg().projects.iter() {
            let installed = if intact(&app, p).is_err_and(|e| e.contains("is missing")) { install(&app, p) } else { Ok(()) };
            let _ = installed.and_then(|_| intact(&app, p)).map_err(|why| app.event("", &p.id, "guard", "", &why, ""));
        }
    });
}
async fn list(State(app): State<Shared>) -> Json<Value> {
    let rows: Vec<Value> = app
        .cfg()
        .projects
        .iter()
        .map(|p| {
            let n = words(&app, p).len();
            let r = intact(&app, p);
            json!({"project": p.id, "name": p.name, "words": n, "ok": r.is_ok(), "reason": r.err().unwrap_or_else(|| if n == 0 { "sin guarda".into() } else { "ok".into() })})
        })
        .collect();
    Json(json!(rows))
}
pub fn routes() -> Router<Shared> {
    let install_one = |State(app): State<Shared>, Path(project): Path<String>| async move {
        let p = app.cfg().project(&project).cloned().ok_or((StatusCode::NOT_FOUND, format!("unknown project {project}")))?;
        install(&app, &p).map_err(|e| (StatusCode::CONFLICT, e))?;
        Ok::<_, (StatusCode, String)>(Json(json!({"project": project, "ok": true})))
    };
    Router::new().route("/api/guard", get(list)).route("/api/guard/{project}/install", post(install_one))
}
/// `tirith guard …` commands.
pub fn cli(pos: &[&str], _flag: crate::client::Flag) -> Option<Result<(), String>> {
    use crate::client::call;
    let all = || -> Result<Vec<Value>, String> { serde_json::from_value(crate::client::call_json("GET", "/api/guard", None)?).map_err(|e| e.to_string()) };
    Some(match pos {
        ["guard", "check"] => all().and_then(|ps| {
            ps.iter().for_each(|p| println!("{:<20} {}", p["project"].as_str().unwrap_or(""), p["reason"].as_str().unwrap_or("")));
            ps.iter().all(|p| p["ok"] == true).then_some(()).ok_or_else(|| "some project's guard needs attention".into())
        }),
        ["guard", "install", rest @ ..] if rest.len() <= 1 => {
            let ids: Vec<String> = rest.first().map_or_else(|| all().map(|ps| ps.iter().filter(|p| p["ok"] != true).filter_map(|p| p["project"].as_str().map(String::from)).collect()).unwrap_or_default(), |id| vec![id.to_string()]);
            let mut failed = false;
            for id in &ids {
                match call("POST", &format!("/api/guard/{id}/install"), None) {
                    Ok(_) => println!("{id}: installed"),
                    Err(e) => {
                        eprintln!("{id}: {e}");
                        failed = true;
                    }
                }
            }
            (!failed).then_some(()).ok_or_else(|| "some guards were not installed".into())
        }
        _ => return None,
    })
}
#[cfg(test)]
#[path = "../checks/guard.rs"]
mod tests;
