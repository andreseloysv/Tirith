//! Models per role and the fallback chain (STUB: the models agent replaces the bodies, keeping every signature).
//!
//! Roles: "manager", "workers", "ceo", "brain". Each has an agent alias, a model, an effort and a fallback chain.
//! The effective setting is the owner's override from the page (stored by this module) or `[models.<role>]` of
//! `config.toml`. Agents are aliases (`[agents.<alias>]`); the real command lives only in config.toml.

use crate::app::Shared;
use axum::Router;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct RoleSetting {
    pub agent: String,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub effort: Option<String>,
    /// Next choices when the agent runs out of credits: "alias" or "alias:model", in order.
    #[serde(default)]
    pub fallback: Vec<String>,
}

/// An agent command line: the alias it belongs to and the full argv (program first).
pub struct AgentCmd {
    pub alias: String,
    pub argv: Vec<String>,
}

/// The effective setting of a role.
pub fn role(app: &Shared, role: &str) -> RoleSetting {
    let cfg = app.cfg();
    let r = match role {
        "manager" => cfg.manager.clone(),
        "workers" => cfg.workers.clone(),
        _ => None,
    };
    r.map(|r| RoleSetting { agent: r.agent, model: r.model, ..Default::default() })
        .unwrap_or_else(|| RoleSetting { agent: "anthropic".into(), ..Default::default() })
}

/// The argv of a lane run. `agent`/`model` override the workers role (a fallback relaunch passes them).
/// `resume = Some((session_id, message))` continues that agent session with `message` instead of the brief.
pub fn lane_argv(
    app: &Shared,
    _project: &str,
    agent: Option<&str>,
    _model: Option<&str>,
    brief: &Path,
    workspace: &Path,
    resume: Option<(&str, &str)>,
) -> Result<AgentCmd, String> {
    let cfg = app.cfg();
    let alias = agent.map(String::from).unwrap_or_else(|| role(app, "workers").agent);
    let a = cfg.agents.get(&alias).ok_or(format!("no agent alias '{alias}' in config.toml"))?;
    let fill = |s: &String| s.replace("{brief}", &brief.display().to_string()).replace("{workspace}", &workspace.display().to_string());
    let mut argv: Vec<String> = if let Some((session, message)) = resume {
        vec![a.command.clone(), "--resume".into(), session.into(), "-p".into(), message.into()]
    } else if agent.is_none() && !cfg.launch.is_empty() {
        cfg.launch.iter().map(fill).collect()
    } else {
        let args = a.args.clone().unwrap_or_else(|| vec!["-p".into(), "Read the brief at {brief} and do it completely in {workspace}.".into()]);
        std::iter::once(a.command.clone()).chain(args.iter().map(fill)).collect()
    };
    if argv.is_empty() {
        argv.push(a.command.clone());
    }
    Ok(AgentCmd { alias, argv })
}

/// The argv of a streaming chat session (stream-json in and out) for "manager" or "ceo".
/// `resume` continues that session id. The caller appends its own system prompt.
pub fn chat_argv(app: &Shared, role_name: &str, resume: Option<&str>) -> Result<AgentCmd, String> {
    let cfg = app.cfg();
    let r = role(app, role_name);
    let a = cfg.agents.get(&r.agent).ok_or(format!("no agent alias '{}' in config.toml", r.agent))?;
    let mut argv = vec![a.command.clone()];
    argv.extend(a.chat_args.clone().unwrap_or_else(|| {
        ["-p", "--input-format", "stream-json", "--output-format", "stream-json", "--verbose"].map(String::from).to_vec()
    }));
    if let Some(m) = &r.model {
        argv.extend(["--model".into(), m.clone()]);
    }
    if let Some(id) = resume {
        argv.extend(["--resume".into(), id.into()]);
    }
    Ok(AgentCmd { alias: r.agent, argv })
}

/// True when an agent's output says it ran out of credits or hit its usage limit.
pub fn out_of_credits(_text: &str) -> bool {
    false
}

/// The next choice of a role's fallback chain that is not in `tried` (aliases already used): (alias, model).
pub fn fallback(_app: &Shared, _role: &str, _tried: &[String]) -> Option<(String, Option<String>)> {
    None
}

pub fn start(_app: Shared) {}

pub fn routes() -> Router<Shared> {
    Router::new()
}

/// `tirith models …` commands; `None` when `pos` is not one of this module's commands.
pub fn cli(_pos: &[&str], _flag: crate::client::Flag) -> Option<Result<(), String>> {
    None
}
