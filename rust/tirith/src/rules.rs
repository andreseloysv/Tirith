//! Rules as code, checked in memory on every `PreToolUse`. A refusal says what to do instead.

use crate::app::{Lane, Shared};
use crate::config::{expand, Project};
use regex::Regex;
use serde_json::Value;
use std::sync::LazyLock;

static GUARD: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"--no-verify|\bgit\b[^;&|]*\bcommit\b[^;&|]*\s-[a-zA-Z]*n\b|core\.hooksPath").unwrap());
/// A shell command that changes files: with `.git/hooks` in it, it is refused (reading the hooks stays allowed).
static WRITES: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\b(rm|mv|cp|ln|chmod|chflags|chown|tee|truncate|touch|install|unlink|rsync)\b|>|\b(sed|perl)\s+-[a-zA-Z]*i").unwrap());
static MAIN: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\bgit\b[^;&|]*\b(push|update-ref)\b|\bgit\b[^;&|]*\b(checkout|switch)\s+(main|master)\b|\bgit\b[^;&|]*\bbranch\s+-[fDM]\b[^;&|]*\b(main|master)\b").unwrap());

pub const EDIT_TOOLS: [&str; 4] = ["Edit", "Write", "MultiEdit", "NotebookEdit"];

pub fn check(app: &Shared, tool: &str, input: &Value, lane: Option<&Lane>, project: Option<&Project>) -> Option<String> {
    let cfg = app.cfg();
    let cmd = input["command"].as_str().unwrap_or("");
    let file = ["file_path", "notebook_path"].iter().find_map(|k| input[*k].as_str()).unwrap_or("");
    let edit = EDIT_TOOLS.contains(&tool);
    let bash = tool == "Bash";

    let hooks_write = bash && cmd.contains(".git/hooks") && WRITES.is_match(&cmd.replace("2>/dev/null", "").replace(">/dev/null", ""));
    if (bash && GUARD.is_match(cmd)) || hooks_write || (edit && file.contains("/.git/hooks/")) {
        return Some("Commit checks are never skipped (--no-verify, commit -n, core.hooksPath, .git/hooks). Fix what the check reports and commit again.".into());
    }
    let mut protected: Vec<String> = cfg.factory.commands().iter().filter_map(|(alias, _)| cfg.factory.agent::<String>(alias, "settings")).map(|s| expand(&s).display().to_string()).collect();
    protected.push(format!("{}/", app.home.display()));
    // The lanes' worktrees live inside Tirith's home: with the bots' folders they are the parts of it agents write in. A path with `..` in it can end anywhere, so it is never taken for one of them.
    let path = std::path::Path::new(file);
    let in_lanes = !path.components().any(|c| c == std::path::Component::ParentDir) && (cfg.projects.iter().any(|p| path.starts_with(p.lanes_dir())) || crate::bots::writable(&app.home, path));
    if edit && !in_lanes && protected.iter().any(|p| file.starts_with(p.as_str())) {
        return Some(format!("{file} belongs to Tirith or to an agent's settings: agents never change it. Ask the owner if it must change."));
    }
    let manifest = project.map(|p| app.manifest(&p.repo()));
    if let (Some(p), Some(m)) = (project, &manifest) {
        let plan = format!("/{}", m.roadmap.trim_start_matches("./"));
        // A shell command that writes to the plan: redirect, tee, in-place edit, move, copy, delete or restore.
        let escaped = regex::escape(m.roadmap.trim_start_matches("./"));
        let writes = bash && Regex::new(&format!(r"(>>?\s*|\btee\s+(-a\s+)?|\b(sed|perl)\s+-[a-zA-Z]*i\S*\s[^;&|]*|\b(mv|cp|rm|truncate)\s[^;&|]*|\bgit\s+(checkout|restore)\s[^;&|]*)\S*{escaped}")).map(|r| r.is_match(cmd)).unwrap_or(false);
        if (edit && file.ends_with(&plan)) || writes {
            return Some(format!("The plan is written only by Tirith. Add an item with: tirith plan add {} <step> \"<text>\"", p.id));
        }
    }
    if let Some(l) = lane {
        let in_projects = in_lanes || cfg.projects.iter().any(|p| path.starts_with(p.repo()));
        if edit && in_projects && !path.starts_with(&l.workspace) {
            return Some(format!("A lane writes only inside its own copy: {}. Change the file there.", l.workspace));
        }
        if bash && MAIN.is_match(cmd) {
            return Some("A lane never pushes or touches main. Commit on your branch and end your turn: Tirith runs the checks and merges.".into());
        }
        let names: Vec<String> = cfg.factory.commands().iter().filter_map(|(_, c)| std::path::Path::new(c).file_name().map(|n| regex::escape(&n.to_string_lossy()))).collect();
        let spawns = !names.is_empty() && Regex::new(&format!(r"(^|[;&|(]\s*|\bexec\s+|\bnohup\s+)({})\b", names.join("|"))).map(|r| r.is_match(cmd)).unwrap_or(false);
        if tool == "Task" || tool == "Agent" || (bash && spawns) {
            return Some("Agents start only through Tirith, so the owner sees them. If the item needs more hands, ask the manager: tirith ask \"<question>\".".into());
        }
    }
    for r in manifest.map(|m| m.rules).unwrap_or_default() {
        if (r.lanes_only && lane.is_none()) || (!r.tools.is_empty() && !r.tools.iter().any(|t| t == tool)) {
            continue;
        }
        let text = if bash { cmd } else { file };
        if Regex::new(&r.pattern).map(|re| re.is_match(text)).unwrap_or(false) {
            return Some(format!("{} ({})", r.message, r.id));
        }
    }
    None
}

pub fn summary(tool: &str, input: &Value) -> String {
    let detail = ["command", "file_path", "notebook_path", "pattern", "url", "description", "prompt", "skill"].iter().find_map(|k| input[*k].as_str()).unwrap_or("");
    let short: String = detail.lines().next().unwrap_or("").chars().take(160).collect();
    let prefix = if tool.is_empty() { String::new() } else { format!("{tool}: ") };
    prefix + &short
}
