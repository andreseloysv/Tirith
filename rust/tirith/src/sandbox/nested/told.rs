//! What is said of a limit kept in one profile, to who called and to Codex's model: it never changes in silence.
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
        "macOS keeps one sandbox, so Codex's {limit} limit is applied by NeoBot's around all of Codex"
    );
    if role {
        line += "; its commands still reach NeoBot, as this role's";
    }
    if !servers.is_empty() {
        line += &format!(
            "; its MCP servers ({}) run under that limit too",
            servers.join(", ")
        );
    }
    line
}

/// The model's instructions: the caller's (`theirs`), where it runs (`here.rs`) and the true limit, for Codex describes the sandbox it was started with, which is none.
pub(super) fn instructions(theirs: &str, mode: Mode, net: bool, here: &str) -> String {
    let writes = match mode {
        Mode::Write => "write only in the working directory, the folders added to it and the temporary folders (never in their .git, .codex or .agents)",
        _ => "write nothing",
    };
    let network = if net {
        "have the network"
    } else {
        "have no network"
    };
    let limit = format!("{here}\nYour commands run in a sandbox, whatever else says: they {writes}, and they {network}. Do not try to get around it: say what you could not do.");
    if theirs.trim().is_empty() {
        limit
    } else {
        format!("{theirs}\n\n{limit}")
    }
}
