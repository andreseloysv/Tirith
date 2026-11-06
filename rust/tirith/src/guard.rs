//! The commit guard: git hooks in every project that refuse a commit whose message, file names or added lines
//! contain one of the owner's forbidden words (whole word, any case; a word may be a regex). The words come only
//! from config.toml: the project's `guard_words`, else `[guard] words`; `guard = false` opts a project out.

use crate::app::Shared;
use crate::config::{home_dir, Project};
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::{json, Value};
use std::path::PathBuf;

const MARKER: &str = "FORBIDDEN_WORDS_GUARD";
const HOOKS: [&str; 2] = ["pre-commit", "commit-msg"];

const HEAD: &str = "#!/bin/bash\n# FORBIDDEN_WORDS_GUARD\n# {what}\n# Installed by Tirith; words: {words}\nset -euo pipefail\nexport TIRITH_GUARD_RE='{re}'\n";

const PRE_COMMIT: &str = r#"
git diff --cached --no-color --no-ext-diff -U0 --diff-filter=ACMR | perl -ne '
    BEGIN { $re = qr/(?<![A-Za-z0-9])($ENV{TIRITH_GUARD_RE})(?![A-Za-z0-9])/i; $bad = 0 }
    if (/^\+\+\+ b\/(.*)$/) {
        $file = $1;
        if ($file =~ $re) { print STDERR "  $file: \"$1\" in the file name\n"; $bad = 1 }
        next;
    }
    if (/^@@ -\S+ \+(\d+)/) { $line = $1; next }
    if (/^\+/) {
        if (/$re/) { chomp; my $t = substr($_, 1, 120); print STDERR "  $file:$line: \"$1\" in: $t\n"; $bad = 1 }
        $line++;
    }
    END {
        if ($bad) { print STDERR "\npre-commit: commit blocked, forbidden words in the staged changes (see above).\n"; exit 1 }
    }
'
"#;

const COMMIT_MSG: &str = r#"
# A lane commit's default attribution trailer is dropped, not blocked.
if [ -n "${TIRITH_LANE:-}" ] && [ -f "${1:-}" ]; then
    tirith_msg=$(mktemp "${TMPDIR:-/tmp}/tirith-msg.XXXXXX") || tirith_msg=
    if [ -n "$tirith_msg" ]; then
        { grep -v -i -E '^[[:space:]]*co-[a-z]+-by:' "$1" || true; } > "$tirith_msg"
        cat "$tirith_msg" > "$1"
        rm -f "$tirith_msg"
    fi
fi
perl -ne '
    BEGIN { $re = qr/(?<![A-Za-z0-9])($ENV{TIRITH_GUARD_RE})(?![A-Za-z0-9])/i; $bad = 0 }
    next if /^#/;
    if (/$re/) { chomp; print STDERR "  line $.: \"$1\" in: $_\n"; $bad = 1 }
    END {
        if ($bad) { print STDERR "\ncommit-msg: commit blocked, forbidden words in the commit message (see above).\n"; exit 1 }
    }
' "$1"
"#;

/// The repository's own hook, kept next to ours as `<hook>.local` when the guard was installed, still runs.
const LOCAL: &str = r#"
local_hook="$(cd "$(dirname "$0")" && pwd)/$(basename "$0").local"
if [ -x "$local_hook" ]; then
    "$local_hook" "$@" || exit 1
fi
"#;

/// The project's forbidden words; empty when it needs no guard.
pub fn words(app: &Shared, project: &Project) -> Vec<String> {
    let cfg = app.cfg();
    let entry = cfg.raw.get("projects").and_then(|p| p.as_array()).and_then(|ps| ps.iter().find(|p| p.get("id").and_then(|i| i.as_str()) == Some(project.id.as_str())).cloned());
    let entry = entry.unwrap_or(toml::Value::Table(Default::default()));
    if entry.get("guard").and_then(|g| g.as_bool()) == Some(false) {
        return vec![];
    }
    let list = |v: Option<&toml::Value>| -> Option<Vec<String>> { v?.clone().try_into().ok() };
    list(entry.get("guard_words")).or_else(|| list(cfg.raw.get("guard").and_then(|g| g.get("words")))).unwrap_or_default().into_iter().filter(|w| !w.trim().is_empty()).collect()
}

/// The repo's common git dir, by reading `.git` (a folder, or a file pointing into a worktree's git dir).
pub fn git_dir(repo: &std::path::Path) -> Option<PathBuf> {
    let dot = repo.join(".git");
    if dot.is_dir() {
        return Some(dot);
    }
    let text = std::fs::read_to_string(&dot).ok()?;
    let dir = repo.join(text.strip_prefix("gitdir:")?.trim());
    match std::fs::read_to_string(dir.join("commondir")) {
        Ok(c) => Some(dir.join(c.trim())),
        Err(_) => Some(dir),
    }
}

fn render(hook: &str, words: &[String]) -> String {
    let (what, body) = match hook {
        "pre-commit" => ("Blocks the commit when a staged file name or an added line contains a forbidden word (whole word, any case).", PRE_COMMIT),
        _ => ("Blocks the commit when the message contains a forbidden word (whole word, any case); # lines are ignored.", COMMIT_MSG),
    };
    let re = words.join("|").replace('\'', "'\\''");
    HEAD.replace("{what}", what).replace("{words}", &words.join(",")).replace("{re}", &re) + body + LOCAL
}

/// A `core.hooksPath` in the repo's or the user's git config makes git skip the guard.
fn hooks_path_set(git: &std::path::Path) -> Option<PathBuf> {
    [git.join("config"), home_dir().join(".gitconfig")].into_iter().find(|f| std::fs::read_to_string(f).is_ok_and(|t| t.lines().any(|l| l.trim().to_lowercase().replace(' ', "").starts_with("hookspath="))))
}

/// Ok when the project's guard is installed and intact (or the project opts out); the reason otherwise.
/// The scheduler calls it before it starts a lane of the project: a few small file reads, no git process.
pub fn intact(app: &Shared, project: &Project) -> Result<(), String> {
    let words = words(app, project);
    if words.is_empty() {
        return Ok(());
    }
    let git = git_dir(&project.repo()).ok_or(format!("commit guard: {} is not a git repo", project.repo().display()))?;
    let fix = format!("(tirith guard install {})", project.id);
    for hook in HOOKS {
        let text = std::fs::read_to_string(git.join("hooks").join(hook)).map_err(|_| format!("commit guard: the {hook} hook is missing {fix}"))?;
        if !text.contains(MARKER) {
            return Err(format!("commit guard: the {hook} hook is not Tirith's; merge the guard into it by hand"));
        }
        if text != render(hook, &words) {
            return Err(format!("commit guard: the {hook} hook is outdated or has other words than config.toml {fix}"));
        }
    }
    if std::fs::read_to_string(git.join("hooks/pre-push")).is_ok_and(|t| t.contains(MARKER)) {
        return Err(format!("commit guard: an old pre-push hook of the Python era is still installed {fix}"));
    }
    if let Some(f) = hooks_path_set(&git) {
        return Err(format!("commit guard: core.hooksPath in {} makes git skip the guard", f.display()));
    }
    Ok(())
}

/// Sets or clears the immutable flag (macOS `uchg`): installed hooks cannot be changed or deleted, by agents or
/// anyone, until the installer clears it. Elsewhere a no-op.
#[cfg(target_os = "macos")]
fn immutable(path: &std::path::Path, on: bool) {
    use std::os::macos::fs::MetadataExt;
    if let (Ok(m), Ok(c)) = (std::fs::metadata(path), std::ffi::CString::new(path.as_os_str().as_encoded_bytes())) {
        let flags = if on { m.st_flags() | libc::UF_IMMUTABLE } else { m.st_flags() & !libc::UF_IMMUTABLE };
        unsafe { libc::chflags(c.as_ptr(), flags) };
    }
}

#[cfg(not(target_os = "macos"))]
fn immutable(_path: &std::path::Path, _on: bool) {}

/// Writes both hooks (mode 755) where they are missing or ours (an older guard is replaced, its pre-push
/// removed); never over a hook that is not ours.
pub fn install(app: &Shared, project: &Project) -> Result<(), String> {
    let words = words(app, project);
    if words.is_empty() {
        return Ok(());
    }
    let dir = git_dir(&project.repo()).ok_or(format!("{} is not a git repo", project.repo().display()))?.join("hooks");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let ours = |p: &std::path::Path| std::fs::read_to_string(p).is_ok_and(|t| t.contains(MARKER));
    for hook in HOOKS {
        let path = dir.join(hook);
        if path.exists() && !ours(&path) {
            // The repository's own hook keeps running after ours, as <hook>.local.
            let local = dir.join(format!("{hook}.local"));
            if local.exists() {
                return Err(format!("{} is not Tirith's hook and {} exists: merge them by hand", path.display(), local.display()));
            }
            immutable(&path, false);
            std::fs::rename(&path, &local).map_err(|e| format!("{}: {e}", path.display()))?;
        }
    }
    let old_push = dir.join("pre-push");
    if ours(&old_push) {
        immutable(&old_push, false);
        std::fs::remove_file(&old_push).map_err(|e| format!("{}: {e}", old_push.display()))?;
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
            let missing = intact(&app, p).is_err_and(|e| e.contains("is missing"));
            let installed = if missing { install(&app, p) } else { Ok(()) };
            if let Err(why) = installed.and_then(|_| intact(&app, p)) {
                app.event("", &p.id, "guard", "", &why, "");
            }
        }
    });
}

fn status(app: &Shared) -> Vec<Value> {
    let cfg = app.cfg();
    cfg.projects
        .iter()
        .map(|p| {
            let n = words(app, p).len();
            let r = intact(app, p);
            json!({"project": p.id, "name": p.name, "words": n, "ok": r.is_ok(), "reason": r.err().unwrap_or_else(|| if n == 0 { "sin guarda".into() } else { "ok".into() })})
        })
        .collect()
}

async fn list(State(app): State<Shared>) -> Json<Value> {
    Json(json!(status(&app)))
}

async fn install_one(State(app): State<Shared>, Path(project): Path<String>) -> Result<Json<Value>, (StatusCode, String)> {
    let p = app.cfg().project(&project).cloned().ok_or((StatusCode::NOT_FOUND, format!("unknown project {project}")))?;
    install(&app, &p).map_err(|e| (StatusCode::CONFLICT, e))?;
    Ok(Json(json!({"project": project, "ok": true})))
}

pub fn routes() -> Router<Shared> {
    Router::new().route("/api/guard", get(list)).route("/api/guard/{project}/install", post(install_one))
}

/// `tirith guard …` commands.
pub fn cli(pos: &[&str], _flag: crate::client::Flag) -> Option<Result<(), String>> {
    use crate::client::call;
    let all = || -> Result<Vec<Value>, String> { serde_json::from_str(&call("GET", "/api/guard", None)?).map_err(|e| e.to_string()) };
    Some(match pos {
        ["guard", "check"] => all().and_then(|ps| {
            for p in &ps {
                println!("{:<20} {}", p["project"].as_str().unwrap_or(""), p["reason"].as_str().unwrap_or(""));
            }
            if ps.iter().all(|p| p["ok"] == true) {
                Ok(())
            } else {
                Err("some project's guard needs attention".into())
            }
        }),
        ["guard", "install", rest @ ..] if rest.len() <= 1 => {
            let ids: Vec<String> = match rest.first() {
                Some(id) => vec![id.to_string()],
                None => all().map(|ps| ps.iter().filter(|p| p["ok"] != true).filter_map(|p| p["project"].as_str().map(String::from)).collect()).unwrap_or_default(),
            };
            let mut failed = false;
            for id in ids {
                match call("POST", &format!("/api/guard/{id}/install"), None) {
                    Ok(_) => println!("{id}: installed"),
                    Err(e) => {
                        eprintln!("{id}: {e}");
                        failed = true;
                    }
                }
            }
            if failed {
                Err("some guards were not installed".into())
            } else {
                Ok(())
            }
        }
        _ => return None,
    })
}
