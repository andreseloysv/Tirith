//! Reading connectors and grants out of TOML that has already passed the checks.
use super::{Connector, Grant};

/// A list of strings, empty when missing.
pub(super) fn strings(t: &toml::Table, k: &str) -> Vec<String> {
    t.get(k)
        .and_then(|v| v.as_array())
        .into_iter()
        .flatten()
        .filter_map(|v| v.as_str().map(String::from))
        .collect()
}

/// One `[[connectors]]` row.
pub(super) fn connector(t: &toml::Table) -> Connector {
    let s = |k: &str| t.get(k).and_then(|v| v.as_str()).unwrap_or("").to_string();
    Connector {
        id: s("id"),
        command: strings(t, "command"),
        url: s("url"),
        blocked: t.get("blocked").and_then(|v| v.as_bool()).unwrap_or(false),
        deny: strings(t, "deny"),
        approval: strings(t, "approval"),
        secrets: strings(t, "secrets"),
    }
}

/// One `[roles.bot-<id>.connectors.<connector>]` table.
pub fn grant(t: &toml::Table) -> Grant {
    Grant {
        tools: t.contains_key("tools").then(|| strings(t, "tools")),
        deny: strings(t, "deny"),
        approval: strings(t, "approval"),
    }
}
