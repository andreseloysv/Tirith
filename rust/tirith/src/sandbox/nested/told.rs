//! What is said of a limit kept in one profile: it never changes in silence.
use super::mode::Mode;

/// One line for whoever called. It is news, not an error, and has no words of one.
pub(super) fn said(mode: Mode, net: bool, unread: bool, role: bool, servers: &[String]) -> String {
    let limit = match (unread, mode, net) {
        (true, ..) => "read-only (its permission profile or managed settings are not read here: give --sandbox to choose)",
        (_, Mode::Write, true) => "workspace-write with network",
        (_, Mode::Write, _) => "workspace-write",
        _ => "read-only",
    };
    let mut line = format!(
        "macOS keeps one sandbox, so Codex's {limit} limit is applied by Tirith's around all of Codex"
    );
    if role {
        line += "; its commands still reach Tirith, as this role's";
    }
    if !servers.is_empty() {
        line += &format!(
            "; its MCP servers ({}) run under that limit too",
            servers.join(", ")
        );
    }
    line
}
