//! The one profile: Tirith's, then the limit that was asked. macOS lets a rule name the program it is for (`process-path`), so what Codex's own process needs (its state, the proxy, the system's services) is that binary's alone, and not its commands'. It only takes away from Tirith's.
use super::sb::{real, Sb};
use super::{commands, services, writes, Request};

/// The name a tighter profile denies too, by which the daemon knows one.
pub(super) const TIGHTER: &str = "app.tirith.tighter";
/// The name that commands under a caller's own limit are denied besides: the daemon changes nothing for them (server/limited.rs).
pub(super) const LIMITED: &str = "app.tirith.limited";

/// What the daemon knows and no caller says.
pub(super) struct Around {
    /// The daemon's port: the rules' hook must reach it.
    pub port: u16,
    /// What nobody in there writes: the broker's socket, Tirith's program, its wrappers and its settings.
    pub kept: Vec<String>,
}

/// The profile's text and its parameters.
pub(super) fn profile(r: &Request, around: &Around) -> (String, Vec<(String, String)>) {
    let mut sb = Sb::after(super::super::profile::PROFILE);
    let program = sb.path(&real(&r.program));
    let own = format!("(process-path {program})");
    commands::limit(&mut sb, &program, &own);
    network(&mut sb, r.net, around.port, &own);
    services::lookups(&mut sb, r.net, &own);
    writes::writes(&mut sb, r, around, &own);
    // last, so nothing above gives them back
    let mut names = vec!["app.tirith.outside", TIGHTER];
    if !r.agent {
        names.push(LIMITED);
    }
    for name in names {
        sb.line(&format!("(deny mach-lookup (global-name \"{name}\"))"));
    }
    sb.done()
}

fn network(sb: &mut Sb, net: bool, port: u16, own: &str) {
    // each by name: a rule with no filter takes back no earlier one that has a filter (Tirith's loopback and sockets)
    sb.line("(deny network-outbound (remote ip \"localhost:*\"))");
    sb.line("(deny network-outbound (remote unix-socket))");
    sb.line(&format!(
        "(allow network-outbound (require-all {own} (remote ip \"localhost:*\")))"
    ));
    sb.line(&format!(
        "(allow network-outbound (remote ip \"localhost:{port}\"))"
    ));
    sb.line("(allow network-outbound (literal \"/private/var/run/syslog\"))");
    if net {
        sb.line("(allow network-outbound (remote ip \"localhost:*\"))");
        return;
    }
    for op in ["network-bind", "network-inbound"] {
        sb.line(&format!("(deny {op})"));
        sb.line(&format!("(allow {op} {own})"));
    }
}
