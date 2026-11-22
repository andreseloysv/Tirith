//! The argv of a streaming chat session for a role.
use super::flags::{flags, set_model_and_effort, AgentCmd};
use super::{browser::add_browser, gateway_args::add_gateway, roles::role};
use crate::app::Shared;
/// The argv of a streaming chat session (stream-json in and out) for a role. `resume` continues that session id. The caller appends its own system prompt.
pub fn chat_argv(app: &Shared, role_name: &str, resume: Option<&str>) -> Result<AgentCmd, String> {
    let cfg = app.cfg();
    let r = role(app, role_name);
    let mut argv = vec![cfg
        .factory
        .command(&r.agent)
        .ok_or(format!("no agent alias '{}' in config.toml", r.agent))?];
    // Exec agents do not speak this chat protocol: their chat and turn roles run as exec turns. Never hand them the default flags of the other one.
    let Some(chat) = cfg.factory.agent::<Vec<String>>(&r.agent, "chat_args") else {
        return Err(format!("agent '{}' has no stream-json chat mode; set chat_args for it in config.toml or run it as exec turns", r.agent));
    };
    argv.extend(chat);
    let (mflag, eflag, _, _) = flags(&cfg, &r.agent);
    set_model_and_effort(&mut argv, (&mflag, &eflag), &r.model, &r.effort);
    argv.extend(
        resume
            .into_iter()
            .flat_map(|id| ["--resume".to_string(), id.to_string()]),
    );
    add_browser(app, &mut argv, &r.agent, false);
    add_gateway(app, &mut argv, &r.agent, role_name); // a bot's role alone
    Ok(AgentCmd {
        alias: r.agent,
        argv,
    })
}
