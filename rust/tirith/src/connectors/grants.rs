//! The strict check of the grants under the roles: only a bot's role holds one, only of a defined connector, with only the keys a grant has.
use super::check::string_list;
use super::Connector;

const KEYS: [&str; 3] = ["tools", "deny", "approval"];

/// What is wrong with one grant: a table of string lists, `tools`, `deny` and `approval` only.
fn wrong_grant(g: &toml::Value) -> Option<String> {
    let Some(g) = g.as_table() else {
        return Some("a table".into());
    };
    g.keys()
        .find(|k| !KEYS.contains(&k.as_str()))
        .map(|k| format!("unknown key {k}"))
        .or_else(|| {
            KEYS.into_iter()
                .find(|k| g.get(*k).is_some_and(|v| !string_list(v)))
                .map(|k| format!("{k} as a list of strings"))
        })
}

/// What is wrong with one role's `connectors` table.
fn wrong_in(role: &str, c: &toml::Value, known: &[&str]) -> Option<String> {
    let Some(c) = c.as_table().filter(|_| role.starts_with("bot-")) else {
        return Some(format!(
            "role '{role}': only a bot's [roles.bot-<id>.connectors.<connector>] tables grant a connector"
        ));
    };
    c.iter().find_map(|(id, g)| {
        if !known.contains(&id.as_str()) {
            return Some(format!("role '{role}' grants unknown connector '{id}'"));
        }
        wrong_grant(g).map(|e| format!("role '{role}' grant '{id}': {e}"))
    })
}

/// The first role whose grants are wrong.
pub(super) fn wrong(roles: &toml::Table, connectors: &[Connector]) -> Option<String> {
    let known: Vec<&str> = connectors.iter().map(|c| c.id.as_str()).collect();
    roles
        .iter()
        .find_map(|(n, r)| r.get("connectors").and_then(|c| wrong_in(n, c, &known)))
}
