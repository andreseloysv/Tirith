//! The one profile: Tirith's, then the limit that was asked. macOS lets a rule name the program it is for (`process-path`), so what Codex's own process needs (its state, the proxy, the system's services) is that binary's alone, and not its commands'. It only takes away from Tirith's.
use super::sb::{real, Sb};
use super::{commands, doors::Doors, services, writes, Request};

/// The name a tighter profile denies too, by which the daemon knows one.
pub(super) const TIGHTER: &str = "app.tirith.tighter";
/// What the daemon knows and no caller says.
pub(super) struct Around {
    /// The daemon's ports: a limited one's commands have the rules' door alone.
    pub doors: Doors,
    /// What nobody in there writes: the broker's socket, Tirith's program, its wrappers and its settings.
    pub kept: Vec<String>,
}

/// The profile's text and its parameters.
pub(super) fn profile(r: &Request, around: &Around) -> (String, Vec<(String, String)>) {
    let mut sb = Sb::after(super::super::profile::PROFILE);
    let program = sb.path(&real(&r.program));
    let own = format!("(process-path {program})");
    commands::limit(&mut sb, &program, &own);
    network(&mut sb, r, around.doors, &own);
    services::lookups(&mut sb, r.net, &own);
    writes::writes(&mut sb, r, around, &own);
    // last, so nothing above gives them back
    for name in ["app.tirith.outside", TIGHTER] {
        sb.line(&format!("(deny mach-lookup (global-name \"{name}\"))"));
    }
    sb.done()
}

fn network(sb: &mut Sb, r: &Request, doors: Doors, own: &str) {
    // each by name: a rule with no filter takes back no earlier one that has a filter (Tirith's loopback and sockets)
    sb.line("(deny network-outbound (remote ip \"localhost:*\"))");
    sb.line("(deny network-outbound (remote unix-socket))");
    sb.line(&format!(
        "(allow network-outbound (require-all {own} (remote ip \"localhost:*\")))"
    ));
    sb.line("(allow network-outbound (literal \"/private/var/run/syslog\"))");
    if r.net {
        sb.line("(allow network-outbound (remote ip \"localhost:*\"))");
    }
    if !r.agent {
        // none of the API, reads and the events' stream included, not for Codex's own process either
        let main = doors.main;
        sb.line(&format!(
            "(deny network-outbound (remote ip \"localhost:{main}\"))"
        ));
    }
    let open = doors.open_to(r);
    sb.line(&format!(
        "(allow network-outbound (remote ip \"localhost:{open}\"))"
    ));
    if r.net {
        return;
    }
    for op in ["network-bind", "network-inbound"] {
        sb.line(&format!("(deny {op})"));
        sb.line(&format!("(allow {op} {own})"));
    }
}
