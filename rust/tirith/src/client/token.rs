//! The token this process calls the daemon with: its agent's, else the owner's.

pub const HEADER: &str = "X-Tirith-Token";
/// Where `sandbox::wrap` puts an agent's.
pub const VAR: &str = "TIRITH_TOKEN";

/// The agent's token: this process's, else the nearest ancestor's, as an agent may hand its hooks and shells none of its variables (Muse).
fn agent() -> Option<String> {
    std::env::var(VAR)
        .ok()
        .filter(|t| !t.is_empty())
        .or_else(|| super::ancestry::var(VAR))
}

/// What a hook sends: an agent's token or none, never the owner's.
pub fn hook() -> Vec<(&'static str, String)> {
    agent().map(|t| (HEADER, t)).into_iter().collect()
}

/// What a command sends. A lane's or a role's session that lost its token never reaches for the owner's.
pub fn command() -> Vec<(&'static str, String)> {
    let sent = hook();
    let in_session = ["TIRITH_LANE", "TIRITH_ROLE"]
        .iter()
        .any(|k| std::env::var(k).is_ok_and(|v| !v.is_empty()));
    if !sent.is_empty() || in_session {
        return sent;
    }
    let file = crate::config::tirith_home().join(crate::app::OWNER);
    std::fs::read_to_string(file)
        .ok()
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
        .map(|t| (HEADER, t))
        .into_iter()
        .collect()
}
