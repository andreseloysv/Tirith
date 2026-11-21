//! The argv of a lane run.
use super::flags::{flags, set_model_and_effort, AgentCmd};
use super::{browser::add_browser, owners::mismatch, roles::lane_role, roles::role};
use crate::app::Shared;
use crate::config::{get, Config};
use std::path::Path;
/// The argv of a lane run. `agent`/`model` override the lane role (a fallback relaunch passes them). `resume = Some((session_id, message))` continues that agent session with `message` instead of the brief; an alias without `resume_args` starts fresh with the brief.
pub fn lane_argv(
    app: &Shared,
    agent: Option<&str>,
    model: Option<&str>,
    brief: &Path,
    workspace: &Path,
    resume: Option<(&str, &str)>,
) -> Result<AgentCmd, String> {
    let cfg = app.cfg();
    let lane = lane_role(&cfg);
    let (alias, model, effort) = choice(app, &lane, agent, model);
    let command = cfg
        .factory
        .command(&alias)
        .ok_or(format!("no agent alias '{alias}' in config.toml"))?;
    mismatch(&cfg, &alias, model.as_deref()).map_or(Ok(()), Err)?;
    let (mflag, eflag, resume_args, args) = flags(&cfg, &alias);
    let bin = command.rsplit('/').next().unwrap_or(&command).to_string();
    let filled = |s: &String| fill(s, brief, workspace, resume.unwrap_or_default());
    let resumed = resume.is_some() && !resume_args.is_empty();
    let words = if resumed { &resume_args } else { &args };
    let mut argv: Vec<String> = if uses_launch_line(&cfg, &lane, &alias, resumed) {
        cfg.launch.iter().map(filled).collect()
    } else {
        std::iter::once(command)
            .chain(words.iter().map(filled))
            .collect()
    };
    set_model_and_effort(&mut argv, (&mflag, &eflag), &model, &effort);
    let _ = (alias == "codex").then(|| crate::codex::options(&cfg, &mut argv, effort.as_deref()));
    let _ = argv
        .first()
        .is_some_and(|a| a.rsplit('/').next() == Some(bin.as_str()))
        .then(|| add_browser(app, &mut argv, &alias, true)); // a custom [launch] line naming another binary than the alias's command never gets its agent's browser flags
    Ok(AgentCmd { alias, argv })
}
/// (alias, model, effort) of the run: the lane role's, an `agent` other than its own bringing neither its model nor its effort.
fn choice(
    app: &Shared,
    lane: &str,
    agent: Option<&str>,
    model: Option<&str>,
) -> (String, Option<String>, Option<String>) {
    let w = role(app, lane);
    let alias = agent.unwrap_or(&w.agent).to_string();
    let same = alias == w.agent;
    let model = model
        .map(String::from)
        .or(if same { w.model } else { None });
    let effort = if same { w.effort } else { None };
    (alias, model, effort)
}
/// A lane word with `{brief}`, `{workspace}`, `{session}` and `{message}` filled in.
fn fill(s: &str, brief: &Path, workspace: &Path, (session, message): (&str, &str)) -> String {
    s.replace("{brief}", &brief.display().to_string())
        .replace("{workspace}", &workspace.display().to_string())
        .replace("{session}", session)
        .replace("{message}", message)
}
/// True when a fresh run of the lane role's own agent uses config.toml's `[launch]` line.
fn uses_launch_line(cfg: &Config, lane: &str, alias: &str, resumed: bool) -> bool {
    !resumed
        && Some(alias)
            == get::<String>(&cfg.raw, &["roles", lane, "agent"])
                .or_else(|| cfg.factory.role(lane, "agent"))
                .as_deref()
        && !cfg.launch.is_empty()
}
