//! What a replay's processes may reach: the model of its bot's agent (`model_hosts` in the factory, and what the agents' catalog adds), and nothing else of the world.
use super::store::running_one;
use crate::app::Shared;

/// The hosts agent `alias` needs to think; none when nobody says.
fn hosts(app: &Shared, alias: &str) -> Option<Vec<String>> {
    let named: Option<Vec<String>> = app.cfg().factory.agent(alias, "model_hosts");
    let more = crate::setup::model_hosts(alias);
    (named.is_some() || !more.is_empty()).then(|| [named.unwrap_or_default(), more].concat())
}

/// Ok when bot `bot` can think inside a replay, else why not.
pub(super) fn ready(app: &Shared, bot: &str) -> Result<(), String> {
    let agent = crate::models::role(app, bot).agent;
    hosts(app, &agent).map(|_| ()).ok_or(format!(
        "a replay reaches nothing but its agent's model, and agent '{agent}' has no model_hosts: use live or shadow"
    ))
}

/// True when the proxy's caller `who` is a running replay and `host:port` is its bot's model, over TLS.
pub fn reaches(app: &Shared, who: &str, host: &str, port: u16) -> bool {
    let id = who.strip_prefix("replay:").and_then(|id| id.parse().ok());
    let Some(e) = id
        .filter(|_| port == 443)
        .and_then(|id| running_one(app, id))
    else {
        return false;
    };
    let agent = crate::models::role(app, &e.bot).agent;
    let hosts = hosts(app, &agent).unwrap_or_default();
    hosts.iter().any(|h| h.eq_ignore_ascii_case(host))
}

/// What agent `alias` is also started with inside a replay: its own tools that read outside and that no hook hears are off (`replay_args`).
pub fn replay_args(app: &Shared, alias: &str) -> Vec<String> {
    let args =
        super::replay::replaying().and_then(|_| app.cfg().factory.agent(alias, "replay_args"));
    args.unwrap_or_default()
}
