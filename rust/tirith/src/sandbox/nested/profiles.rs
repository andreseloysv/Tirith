//! Which profile of Codex's settings is chosen, and which MCP servers they start.
use super::asked::Asked;
use super::layers::Layers;
use super::values::value;
use std::collections::BTreeSet;
use toml::Table;

/// `--profile`, else the `profile` setting.
pub(super) fn chosen(a: &Asked, base: &Table) -> Option<String> {
    let set = a.sets.iter().rev().find(|(key, _)| key == "profile");
    let named = || match set {
        Some((_, raw)) => value(raw).as_str().map(String::from),
        None => base.get("profile")?.as_str().map(String::from),
    };
    a.profile.clone().or_else(named)
}

impl Layers {
    /// The names of the MCP servers that Codex starts as programs: they run where it runs.
    pub(super) fn stdio(&self) -> Vec<String> {
        let mut names = BTreeSet::new();
        for (key, set) in &self.sets {
            let Some(rest) = key.strip_prefix("mcp_servers.") else {
                continue;
            };
            let (name, field) = rest.split_once('.').unwrap_or((rest, ""));
            if field == "command" || (field.is_empty() && set.get("command").is_some()) {
                names.insert(name.to_string());
            }
        }
        for servers in self
            .files
            .iter()
            .filter_map(|f| f.get("mcp_servers")?.as_table())
        {
            let started = servers.iter().filter(|(_, v)| v.get("command").is_some());
            names.extend(started.map(|(name, _)| name.clone()));
        }
        names.into_iter().collect()
    }
}
