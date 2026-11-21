//! A lane's rules: it writes only in its own copy, never touches main and never starts agents.
use super::{call::Call, owned::in_lanes, patterns::MAIN};
use crate::app::{Lane, Shared};
use crate::config::Config;
use regex::Regex;

pub(super) fn refused(app: &Shared, cfg: &Config, call: &Call, l: &Lane) -> Option<String> {
    let outside = call
        .files
        .iter()
        .copied()
        .map(std::path::Path::new)
        .any(|path| {
            (in_lanes(app, cfg, path) || cfg.projects.iter().any(|p| path.starts_with(p.repo())))
                && !path.starts_with(&l.workspace)
        });
    if call.edit && outside {
        return Some(format!(
            "A lane writes only inside its own copy: {}. Change the file there.",
            l.workspace
        ));
    }
    if call.bash && MAIN.is_match(call.cmd) {
        return Some("A lane never pushes or touches main. Commit on your branch and end your turn: Tirith runs the checks and merges.".into());
    }
    let starts =
        call.tool == "Task" || call.tool == "Agent" || (call.bash && spawns(cfg, call.cmd));
    starts.then(|| "Agents start only through Tirith, so the owner sees them. If the item needs more hands, ask the manager: tirith ask \"<question>\".".into())
}

/// A shell command that runs one of the configured agent commands.
fn spawns(cfg: &Config, cmd: &str) -> bool {
    let names: Vec<String> = cfg
        .factory
        .commands()
        .iter()
        .filter_map(|(_, c)| {
            std::path::Path::new(c)
                .file_name()
                .map(|n| regex::escape(&n.to_string_lossy()))
        })
        .collect();
    !names.is_empty()
        && Regex::new(&format!(
            r"(^|[;&|(]\s*|\bexec\s+|\bnohup\s+)({})\b",
            names.join("|")
        ))
        .map(|r| r.is_match(cmd))
        .unwrap_or(false)
}
