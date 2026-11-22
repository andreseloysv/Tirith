//! The command line Codex is started with: its own sandbox off, and its model told the true limit.
use super::{asked::Asked, here::here, layers::Layers, mode::Mode, told::instructions};
use std::path::Path;

/// The proxy's variables: a limited Codex's commands get them empty, as they got no network.
const NO_NET: [&str; 7] = [
    "HTTPS_PROXY",
    "HTTP_PROXY",
    "ALL_PROXY",
    "https_proxy",
    "http_proxy",
    "all_proxy",
    "GRADLE_OPTS",
];
const BYPASS: &str = "--dangerously-bypass-approvals-and-sandbox";
/// With its sandbox off there Codex would tell its model that it has full access, in its permissions' words and in its environment note.
const UNTOLD: [&str; 2] = [
    "include_permissions_instructions=false",
    "include_environment_context=false",
];

/// The caller's line with those words where `exec`'s options go.
pub(super) fn line(a: &Asked, set: &Layers, mode: Mode, net: bool, dir: &Path) -> Vec<String> {
    let mut words = vec![BYPASS.to_string()];
    for untold in UNTOLD {
        words.extend(["-c".to_string(), untold.to_string()]);
    }
    let theirs = set.get("developer_instructions");
    let theirs = theirs.as_ref().and_then(|v| v.as_str()).unwrap_or("");
    let truth = toml::Value::String(instructions(theirs, mode, net, &here(dir)));
    words.extend(["-c".into(), format!("developer_instructions={truth}")]);
    for name in NO_NET.iter().filter(|_| !net) {
        words.extend([
            "-c".into(),
            format!("shell_environment_policy.set.{name}=\"\""),
        ]);
    }
    let mut line = a.args.clone();
    line.splice(a.at..a.at, words);
    line
}
