//! What the guide writes in config.toml: an agent's command and hooks file, the port, and which agent each role runs on.
use super::{catalog, edit::edit, edit::table};
use crate::config::Config;
use std::path::Path;
use toml_edit::value;

pub fn agent(home: &Path, alias: &str, command: &Path, settings: &str) -> Result<(), String> {
    let command = command.display().to_string();
    edit(home, |doc| {
        let t = table(doc, &["agents", alias])?;
        t.insert("command", value(command));
        if !settings.is_empty() && !t.contains_key("settings") {
            t.insert("settings", value(settings));
        }
        Ok(())
    })
}

/// The daemon's port, with a proxy port beside it when it is not the usual one (two Tiriths on one machine).
pub fn port(home: &Path, port: u16) -> Result<(), String> {
    let proxy = match port {
        7717 => 47812,
        p if p <= 25_535 => p + 40_000,
        p => p - 10_000,
    };
    edit(home, |doc| {
        table(doc, &["server"])?.insert("port", value(i64::from(port)));
        let net = table(doc, &["network"])?;
        if !net.contains_key("proxy_port") || port != 7717 {
            net.insert("proxy_port", value(i64::from(proxy)));
        }
        Ok(())
    })
}

/// The roles that run on an agent this machine has not connected move to `primary`: (role, the agent it had).
pub fn roles(
    home: &Path,
    connected: &[String],
    primary: &str,
) -> Result<Vec<(String, String)>, String> {
    let cfg = Config::load_at(home);
    let moved: Vec<(String, String)> = cfg
        .factory
        .roles()
        .into_iter()
        .filter(|r| !r.starts_with("bot-") && r != super::prove::ROLE)
        .filter_map(|r| Some((r.clone(), cfg.factory.role::<String>(&r, "agent")?)))
        .filter(|(_, agent)| !agent.is_empty() && !connected.contains(agent))
        .collect();
    if moved.is_empty() {
        return Ok(moved);
    }
    let cheap = catalog::known(primary)
        .map(|k| k.check_model)
        .unwrap_or_default();
    edit(home, |doc| {
        for (role, _) in &moved {
            let t = table(doc, &["roles", role])?;
            t.insert("agent", value(primary));
            t.remove("model"); // the other agent's
            if role == "judge" && !cheap.is_empty() {
                t.insert("model", value(&cheap)); // the judge is the cheap one
            }
        }
        Ok(())
    })?;
    Ok(moved)
}
