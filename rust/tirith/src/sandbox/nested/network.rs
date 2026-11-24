//! What a limit leaves of this machine's ports: none of them the commands', unless the network was asked for, and the daemon's only a role's.
use super::{doors::Doors, sb::Sb, Request};

/// `own`: Codex's own process, in its part.
pub(super) fn network(sb: &mut Sb, r: &Request, doors: Doors, own: Option<&str>) {
    // each by name: a rule with no filter takes back no earlier one that has a filter (NeoBot's loopback and sockets)
    sb.line("(deny network-outbound (remote ip \"localhost:*\"))");
    sb.line("(deny network-outbound (remote unix-socket))");
    if let Some(own) = own {
        sb.line(&format!(
            "(allow network-outbound (require-all {own} (remote ip \"localhost:*\")))"
        ));
    }
    sb.line("(allow network-outbound (literal \"/private/var/run/syslog\"))");
    if r.net {
        sb.line("(allow network-outbound (remote ip \"localhost:*\"))");
    }
    let port = |port: u16, rule: &str| {
        format!("({rule} network-outbound (remote ip \"localhost:{port}\"))")
    };
    if !r.agent {
        // none of the API, reads and the events' stream included, not for Codex's own process either
        sb.line(&port(doors.main, "deny"));
    }
    if r.agent || own.is_some() {
        sb.line(&port(doors.open_to(r), "allow")); // the rules' hook is started by Codex's own process
    } else {
        sb.line(&port(doors.rules, "deny")); // a command has no hook to call
    }
    if r.net {
        return;
    }
    for op in ["network-bind", "network-inbound"] {
        sb.line(&format!("(deny {op})"));
        if let Some(own) = own {
            sb.line(&format!("(allow {op} {own})"));
        }
    }
}
