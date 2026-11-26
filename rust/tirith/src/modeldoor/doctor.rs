//! What `neobot doctor` says of the door: each agent that holds a provider key either has it kept by the daemon or has it in its environment.
use super::spec::find;
use crate::config::{get, Config};
use std::path::Path;

/// One line for each auth file that holds a provider key: (good, what, fix).
pub fn lines(cfg: &Config, home: &Path) -> Vec<(bool, String, String)> {
    let server = get::<String>(&cfg.raw, &["server", "mode"]).as_deref() == Some("server");
    let on = get::<bool>(&cfg.raw, &["door", "enabled"]).unwrap_or(server);
    let files = std::fs::read_dir(home.join("auth"))
        .into_iter()
        .flatten()
        .flatten();
    let mut names: Vec<String> = files
        .filter_map(|f| {
            f.path()
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .filter(|_| f.path().extension().is_some_and(|e| e == "env"))
        })
        .collect();
    names.sort();
    names
        .into_iter()
        .filter(|alias| !find(&crate::setup::auth_env(home, alias)).is_empty())
        .map(|alias| match on {
            true => (true, format!("the model door keeps {alias}'s provider key: its agents get a token, never the key"), String::new()),
            false => (false, format!("{alias}'s provider key is in its agents' environment"), "turn the model door on: [door] enabled = true in config.toml (on by default in server mode)".into()),
        })
        .collect()
}
