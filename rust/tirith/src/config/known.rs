//! The sections of config.toml something reads, and the keys nothing does.

const KNOWN: [&str; 26] = [
    "projects",
    "agents",
    "launch",
    "models",
    "server",
    "merge",
    "autopilot",
    "app",
    "network",
    "messenger",
    "owner",
    "rooms",
    "guard",
    "factory",
    "roles",
    "limits",
    "plan",
    "cards",
    "metrics",
    "routes",
    "evals",
    "lab",
    "browser",
    "prices",
    "power",
    "connectors",
];

/// Keys nothing reads any more, named for the owner: an unknown section, or a leftover `[models.*]` role (`choices`/`efforts` lists stay). The factory-shaped sections are checked by the factory loader itself.
pub(super) fn unknown(t: &toml::Table) -> Vec<String> {
    let mut unknown: Vec<String> = t
        .keys()
        .filter(|k| !KNOWN.contains(&k.as_str()))
        .cloned()
        .collect();
    unknown.extend(
        t.get("models")
            .and_then(|v| v.as_table())
            .into_iter()
            .flat_map(|m| m.keys())
            .filter(|k| !matches!(k.as_str(), "choices" | "efforts"))
            .map(|k| format!("models.{k}")),
    );
    unknown
}
