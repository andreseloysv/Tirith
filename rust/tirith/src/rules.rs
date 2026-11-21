//! Rules as code, checked in memory on every `PreToolUse`. A refusal says what to do instead.

use crate::app::{Lane, Shared};
use crate::config::{expand, Project};
use regex::Regex;
use serde_json::{json, Value};
use std::sync::LazyLock;

static GUARD: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"--no-verify|\bgit\b[^;&|]*\bcommit\b[^;&|]*\s-[a-zA-Z]*n\b|core\.hooksPath").unwrap());
/// A shell command that changes files: with `.git/hooks` in it, it is refused (reading the hooks stays allowed).
static WRITES: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\b(rm|mv|cp|ln|chmod|chflags|chown|tee|truncate|touch|install|unlink|rsync)\b|>|\b(sed|perl)\s+-[a-zA-Z]*i").unwrap());
static MAIN: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\bgit\b[^;&|]*\b(push|update-ref)\b|\bgit\b[^;&|]*\b(checkout|switch)\s+(main|master)\b|\bgit\b[^;&|]*\bbranch\s+-[fDM]\b[^;&|]*\b(main|master)\b").unwrap());
static WAIT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?s)\bsleep\s+(?:[5-9]|[1-9]\d+)(?:\.\d+)?s?\b|\bsleep\s+[1-9]\d*(?:\.\d+)?[mhd]\b|\b(?:for|while|until)\b.*\bsleep\s").unwrap()); // sleep >= 5s, or a loop's sleep
/// Following or watching something until it happens, only in command position (at the start or after `;`, `&&`, `||`, `|`, `(`, `$(`, `do` or `then`), with quoted text taken out first (`QUOTED`) so a message that says «watch» or «wait» passes: `tail` with `-f`/`-F`/`--follow` (in a joined cluster too, as `-fn0` or `-n0F`), `watch`, `fswatch`, `inotifywait`, the shell's `wait`, `gh run watch`, and any `--watch`.
static QUOTED: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"(?s)'(?:[^'\\]|\\.)*'|"(?:[^"\\]|\\.)*""#).unwrap());
static FOLLOW: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?:^|[;&|(\n]|\b(?:do|then)\b)\s*(?:\S*/)?(?:(?:watch|fswatch|inotifywait|wait)(?:$|[\s;&|()])|tail\b[^;&|()\n]*?(?:\s--follow(?:[=\s]|$)|\s-[A-Za-z0-9]*[fF][A-Za-z0-9]*)|gh\s+run\s+watch(?:$|[\s;&|()]))|(?:^|\s)--watch(?:[=\s]|$)").unwrap()
});

pub const EDIT_TOOLS: [&str; 7] = ["Edit", "Write", "MultiEdit", "NotebookEdit", "edit_file", "write_file", "apply_patch"];

/// Every agent's tool call in the names the rules know, translated in this one place before `check` and the sandbox audit (`tags`, `check_command`): a shell is `Bash` (also `bash` and `monitor`, the text in `command`); an edit is one of the four edit tools above (also `edit_file`/`write_file`, the file in `path`, and `apply_patch`, whose files are its patch's `*** … File:` and `*** Move to:` lines). The call keeps its input, with its files — every one of them, relative ones taken against `cwd` — in `file_path` (the first) and `file_paths` (all). Any other call keeps its name.
pub fn normalize(tool: &str, input: &Value, cwd: &str) -> (String, Value) {
    let at = |p: &str| if std::path::Path::new(p.trim()).is_absolute() || cwd.is_empty() { p.trim().to_string() } else { std::path::Path::new(cwd).join(p.trim()).display().to_string() };
    let heads = ["*** Add File:", "*** Update File:", "*** Delete File:", "*** Move to:"];
    let files: Vec<String> = if tool == "apply_patch" {
        input.as_str().into_iter().chain(input.as_object().into_iter().flat_map(|o| o.values().filter_map(|v| v.as_str()))).flat_map(|t| t.lines()).filter_map(|l| heads.iter().find_map(|h| l.trim_start().strip_prefix(h))).map(&at).collect()
    } else {
        ["file_path", "notebook_path", "path"].iter().find_map(|k| input[*k].as_str()).map(&at).into_iter().collect()
    };
    let name = match tool {
        "bash" | "monitor" => "Bash",
        "write_file" => "Write",
        "edit_file" | "apply_patch" => "Edit",
        t => t,
    };
    let mut v = input.clone();
    if let Some(first) = files.first().filter(|_| v.is_object()) {
        (v["file_path"], v["file_paths"]) = (json!(first), json!(files));
    }
    (name.into(), v)
}

/// The role whose folder under Tirith's home holds `cwd` — `<home>/<role>/…`, a bot's `<home>/manager/<role>`, and a project's chat in `<home>/manager/<project>` (the manager) — never a lane under `<home>/lanes` (no role of that name runs a chat or a turn there). It names a hook's chat role when the agent passed none on to its hooks.
pub fn role_at(app: &Shared, cwd: &str) -> Option<String> {
    let rel = std::path::Path::new(cwd).strip_prefix(&app.home).ok()?;
    let mut parts = rel.iter().map(|c| c.to_string_lossy().into_owned());
    let known = |r: &str| app.cfg().factory.roles().iter().any(|x| x == r) && app.cfg().factory.role::<String>(r, "session").as_deref() != Some("lane");
    match parts.next().as_deref() {
        Some("manager") => Some(parts.next().filter(|r| known(r)).unwrap_or_else(|| "manager".into())),
        Some(r) if known(r) => Some(r.to_string()),
        _ => None,
    }
}

pub fn check(app: &Shared, tool: &str, input: &Value, lane: Option<&Lane>, project: Option<&Project>, role: Option<&str>) -> Option<String> {
    let cfg = app.cfg();
    let cmd = input["command"].as_str().unwrap_or("");
    let files: Vec<&str> = input["file_paths"].as_array().into_iter().flatten().filter_map(|v| v.as_str()).collect();
    let edit = EDIT_TOOLS.contains(&tool);
    let bash = tool == "Bash";

    let hooks_write = bash && cmd.contains(".git/hooks") && WRITES.is_match(&cmd.replace("2>/dev/null", "").replace(">/dev/null", ""));
    let skips = (bash && GUARD.is_match(cmd)) || hooks_write || (edit && files.iter().any(|f| f.contains("/.git/hooks/")));
    let Some(()) = (!skips).then_some(()) else { return Some("Commit checks are never skipped (--no-verify, commit -n, core.hooksPath, .git/hooks). Fix what the check reports and commit again.".into()) };
    let waiting = lane.is_none() && role.is_some() && ((bash && (WAIT.is_match(cmd) || FOLLOW.is_match(&QUOTED.replace_all(cmd, " ")))) || tool == "clocksleep");
    let Some(()) = (!waiting).then_some(()) else { return Some("no esperes dentro del turno: contesta y termina; Tirith te avisa cuando una lane entre, falle o pregunte".into()) };
    let mut protected: Vec<String> = cfg.factory.commands().iter().filter_map(|(alias, _)| cfg.factory.agent::<String>(alias, "settings")).map(|s| expand(&s).display().to_string()).collect();
    protected.push(format!("{}/", app.home.display()));
    // The lanes' worktrees live inside Tirith's home: with the bots' folders they are the parts of it agents write in. A path with `..` in it can end anywhere, so it is never taken for one of them.
    let in_lanes = |path: &std::path::Path| !path.components().any(|c| c == std::path::Component::ParentDir) && (cfg.projects.iter().any(|p| path.starts_with(p.lanes_dir())) || crate::bots::writable(&app.home, path));
    let guarded = files.iter().find(|f| !in_lanes(std::path::Path::new(f)) && protected.iter().any(|p| f.starts_with(p.as_str())));
    let Some(()) = (!(edit && guarded.is_some())).then_some(()) else { return Some(format!("{} belongs to Tirith or to an agent's settings: agents never change it. Ask the owner if it must change.", guarded.unwrap())) };
    let manifest = project.map(|p| app.manifest(&p.repo()));
    if let (Some(p), Some(m)) = (project, &manifest) {
        let plan = format!("/{}", m.roadmap.trim_start_matches("./"));
        // A shell command that writes to the plan: redirect, tee, in-place edit, move, copy, delete or restore.
        let escaped = regex::escape(m.roadmap.trim_start_matches("./"));
        let writes = bash && Regex::new(&format!(r"(>>?\s*|\btee\s+(-a\s+)?|\b(sed|perl)\s+-[a-zA-Z]*i\S*\s[^;&|]*|\b(mv|cp|rm|truncate)\s[^;&|]*|\bgit\s+(checkout|restore)\s[^;&|]*)\S*{escaped}")).map(|r| r.is_match(cmd)).unwrap_or(false);
        let Some(()) = (!((edit && files.iter().any(|f| f.ends_with(&plan))) || writes)).then_some(()) else { return Some(format!("The plan is written only by Tirith. Add an item with: tirith plan add {} <step> \"<text>\"", p.id)) };
    }
    if let Some(l) = lane {
        let outside = files.iter().copied().map(std::path::Path::new).any(|path| (in_lanes(path) || cfg.projects.iter().any(|p| path.starts_with(p.repo()))) && !path.starts_with(&l.workspace));
        let Some(()) = (!(edit && outside)).then_some(()) else { return Some(format!("A lane writes only inside its own copy: {}. Change the file there.", l.workspace)) };
        let Some(()) = (!(bash && MAIN.is_match(cmd))).then_some(()) else { return Some("A lane never pushes or touches main. Commit on your branch and end your turn: Tirith runs the checks and merges.".into()) };
        let names: Vec<String> = cfg.factory.commands().iter().filter_map(|(_, c)| std::path::Path::new(c).file_name().map(|n| regex::escape(&n.to_string_lossy()))).collect();
        let spawns = !names.is_empty() && Regex::new(&format!(r"(^|[;&|(]\s*|\bexec\s+|\bnohup\s+)({})\b", names.join("|"))).map(|r| r.is_match(cmd)).unwrap_or(false);
        if tool == "Task" || tool == "Agent" || (bash && spawns) {
            return Some("Agents start only through Tirith, so the owner sees them. If the item needs more hands, ask the manager: tirith ask \"<question>\".".into());
        }
    }
    manifest.map(|m| m.rules).unwrap_or_default().into_iter().find_map(|r| {
        let applies = !(r.lanes_only && lane.is_none()) && (r.tools.is_empty() || r.tools.iter().any(|t| t == tool));
        (applies && Regex::new(&r.pattern).is_ok_and(|re| if bash { re.is_match(cmd) } else { files.iter().any(|f| re.is_match(f)) })).then(|| format!("{} ({})", r.message, r.id))
    })
}

pub fn summary(tool: &str, input: &Value) -> String {
    let detail = ["command", "file_path", "notebook_path", "path", "pattern", "url", "description", "prompt", "skill"].iter().find_map(|k| input[*k].as_str()).unwrap_or("");
    format!("{}{}", if tool.is_empty() { String::new() } else { format!("{tool}: ") }, detail.lines().next().unwrap_or("").chars().take(160).collect::<String>())
}
