//! The one profile: Tirith's, and then Codex's limit on top. It only takes away; paths travel as parameters, never as profile text.
use super::Request;
use std::path::Path;

/// The name a tighter profile denies too, by which the daemon knows one.
pub(super) const TIGHTER: &str = "app.tirith.tighter";
/// What Codex's state folder holds that steers Codex or is run: never the commands' to write.
const STEERS: [&str; 9] = [
    "config.toml",
    "hooks.json",
    "AGENTS.md",
    "AGENTS.override.md",
    "rules",
    "packages",
    "plugins",
    "prompts",
    "bin",
];
/// What workspace-write itself protects in each of its folders.
const PROTECTED: [&str; 3] = [".git", ".codex", ".agents"];

/// The profile's text and its parameters. `ports`: the loopback ports that stay open when the commands have no network.
pub(super) fn profile(r: &Request, ports: &[u16]) -> (String, Vec<(String, String)>) {
    let mut params: Vec<(String, String)> = vec![];
    let mut param = |path: &str| {
        let name = format!("P{}", params.len());
        params.push((name.clone(), real(path)));
        format!("(param \"{name}\")")
    };
    let mut text = super::super::profile::PROFILE.to_string();
    text += &format!("(deny mach-lookup (global-name \"{TIGHTER}\"))\n(deny lsopen)\n(deny network-outbound (remote unix-socket))\n");
    text += "(deny file-write*)\n(allow file-write* (subpath \"/dev\"))\n";
    let roots: &[String] = if r.write { &r.roots } else { &[] };
    for path in roots.iter().chain(Some(&r.state)).filter(|p| !p.is_empty()) {
        text += &format!("(allow file-write* (subpath {}))\n", param(path));
    }
    if !r.out.is_empty() {
        text += &format!("(allow file-write* (literal {}))\n", param(&r.out));
    }
    let steers = STEERS
        .iter()
        .filter(|_| !r.state.is_empty())
        .map(|n| format!("{}/{n}", r.state));
    let guarded = roots
        .iter()
        .flat_map(|root| PROTECTED.map(|n| format!("{root}/{n}")));
    for path in steers.chain(guarded) {
        text += &format!("(deny file-write* (subpath {}))\n", param(&path));
    }
    if !r.net {
        text += "(deny network-outbound (remote ip \"localhost:*\"))\n";
        for port in ports {
            text += &format!("(allow network-outbound (remote ip \"localhost:{port}\"))\n");
        }
    }
    (text, params)
}

/// `path` as the kernel names it (/tmp is /private/tmp), also when its end is not there yet.
fn real(path: &str) -> String {
    let path = Path::new(path);
    let (mut head, mut tail) = (path, vec![]);
    loop {
        if let Ok(found) = head.canonicalize() {
            let whole = tail.iter().rev().fold(found, |p, part| p.join(part));
            return whole.display().to_string();
        }
        let (Some(parent), Some(last)) = (head.parent(), head.file_name()) else {
            return path.display().to_string();
        };
        tail.push(last.to_owned());
        head = parent;
    }
}
