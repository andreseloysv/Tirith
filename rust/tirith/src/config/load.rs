//! Loading config.toml: parse it, move yesterday's sections, lay it over the factory and name what is wrong.
use super::file::Config;
use super::known::unknown;
use super::migrate::migrate;
use super::table::get;
use std::path::Path;

/// The file's table, and why it does not parse when it does not (an empty file is fine).
fn read(home: &Path) -> (toml::Table, Option<String>) {
    let text = std::fs::read_to_string(home.join("config.toml")).unwrap_or_default();
    match text.parse::<toml::Table>() {
        Ok(t) => (t, None),
        Err(_) if text.trim().is_empty() => (Default::default(), None),
        Err(e) => (
            Default::default(),
            Some(format!(
                "config.toml does not parse ({e}); the defaults run instead"
            )),
        ),
    }
}

/// Everything wrong with the file and the factory, `; `-joined; none when all is as written.
fn errors(parsed: Option<String>, t: &toml::Table) -> Option<String> {
    let unknown = unknown(t);
    let error = [
        parsed,
        crate::factory::error()
            .map(|e| format!("factory.invalid: {e} (the last good factory keeps running)")),
        (!unknown.is_empty())
            .then(|| format!("config.toml has keys nothing reads: {}", unknown.join(", "))),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>();
    (!error.is_empty()).then(|| error.join("; "))
}

/// `TIRITH_PORT`, else `[server] port`, else 7717.
fn port(t: &toml::Table) -> u16 {
    std::env::var("TIRITH_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .or_else(|| get(t, &["server", "port"]))
        .unwrap_or(7717)
}

impl Config {
    /// The config of one NeoBot home, named instead of read from the process-wide `TIRITH_HOME` (a test opens its App at its own folder; moving the variable would move every parallel test's home too).
    pub fn load_at(home: &Path) -> Config {
        let (mut t, parsed) = read(home);
        migrate(&mut t);
        let factory = crate::factory::load(&t);
        Config {
            error: errors(parsed, &t),
            projects: get(&t, &["projects"]).unwrap_or_default(),
            launch: get(&t, &["launch", "command"]).unwrap_or_default(),
            port: port(&t),
            push: get(&t, &["merge", "push"]).unwrap_or(true),
            autopilot: get(&t, &["autopilot", "enabled"]).unwrap_or(true),
            factory,
            raw: t,
        }
    }
}
