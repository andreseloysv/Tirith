//! The profiles of a limited Codex's two parts (`pair.rs`): OpenBot's, then the limit that was asked. macOS lets a rule name the program it is for (`process-path`), so in Codex's own part what its process needs (its state, the proxy, the system's services) is that binary's alone, and not its children's. They only take away from OpenBot's.
use super::sb::{real, Sb};
use super::{commands, doors::Doors, network::network, services, writes, Request};

/// The name a tighter profile denies too, by which the daemon knows one.
pub(super) const TIGHTER: &str = "app.tirith.tighter";
/// Whose profile it is.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Part {
    /// Codex's own process, which holds its state.
    Codex,
    /// What runs the model's commands and patches: nothing of that.
    Commands,
}

/// What the daemon knows and no caller says.
pub(super) struct Around {
    /// The daemon's ports: a limited one's commands have the rules' door alone.
    pub doors: Doors,
    /// What nobody in there writes: the broker's socket, OpenBot's program, its wrappers and its settings.
    pub kept: Vec<String>,
    /// What nobody in there reads either (`app/keys.rs`).
    pub hidden: Vec<std::path::PathBuf>,
}

/// The profile's text and its parameters.
pub(super) fn profile(r: &Request, around: &Around, part: Part) -> (String, Vec<(String, String)>) {
    let mut sb = Sb::after(super::super::profile::PROFILE);
    let program = sb.path(&real(&r.program));
    let process = format!("(process-path {program})");
    let own = (part == Part::Codex).then_some(process.as_str());
    commands::limit(&mut sb, &program, own);
    network(&mut sb, r, around.doors, own);
    services::lookups(&mut sb, r.net, own);
    writes::writes(&mut sb, r, around, own);
    // last, so nothing above gives them back
    for name in ["app.tirith.outside", TIGHTER] {
        sb.line(&format!("(deny mach-lookup (global-name \"{name}\"))"));
    }
    for rule in super::super::hidden::rules(&around.hidden).lines() {
        sb.line(rule);
    }
    sb.done()
}
