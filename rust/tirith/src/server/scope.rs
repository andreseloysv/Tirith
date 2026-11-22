//! What an agent's token does not open: what is the owner's alone, and what a lane or a role would do in another's name.
use super::who::Agent;
use axum::http::Method;

const OWNERS: &str =
    "that is the owner's, from the page or their own terminal, never an agent's: say in your chat what you need and end your turn";
const ASKS: &str = "tirith ask is a lane's own question: no one asks in a lane's name";
const MANAGERS: &str = "a lane does not do that: ask the manager with tirith ask \"<question>\"";

/// Why `agent` may not make this call.
pub(super) fn refused(agent: &Agent, method: &Method, path: &str) -> Option<&'static str> {
    let parts: Vec<&str> = path.split('/').skip(1).collect();
    let post = method == Method::POST;
    let owners = match parts.as_slice() {
        // pairing a phone, which is then the owner
        ["api", "rooms", .., "qr" | "qr.svg"] => true,
        ["api", "net", "blocks"] => method == Method::DELETE,
        ["api", "turns", .., "reaction"] => true,
        ["api", "page"] => true,
        // an agent is never the teach extension
        ["api", "teach", _, "events" | "next" | "done"] => true,
        _ => false,
    };
    if owners || crate::secrets::route(path) {
        return Some(OWNERS);
    }
    if !post {
        return None;
    }
    if let ["api", "lanes", lane, "ask"] = parts.as_slice() {
        return (agent.lane != *lane).then_some(ASKS);
    }
    let managers = matches!(
        parts.as_slice(),
        ["api", "owner"]
            | ["api", "lanes", _, "answer"]
            | ["api", "chat", "clear" | "keep"]
            | ["api", "turns", "checks"]
            | ["api", "safari"]
    );
    (managers && !agent.lane.is_empty()).then_some(MANAGERS)
}
