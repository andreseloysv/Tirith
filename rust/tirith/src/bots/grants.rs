//! A bot's connector grants (`[roles.bot-<id>.connectors.<connector>]`) through a rewrite of config.toml: the page's card does not carry them, so they are taken out of the owner's bot roles before those go and laid back under each bot still there; a deleted bot's go with its role.
use super::written::key_of;
use serde_json::Value;
use toml_edit::{DocumentMut, Item, Table};

/// Each owner's bot role's `connectors` table, by role key, as written.
pub(super) fn taken(doc: &DocumentMut) -> Vec<(String, Table)> {
    doc.get("roles")
        .and_then(|r| r.as_table())
        .into_iter()
        .flat_map(|r| r.iter())
        .filter(|(k, _)| k.starts_with("bot-"))
        .filter_map(|(k, v)| Some((k.to_string(), v.get("connectors")?.as_table()?.clone())))
        .collect()
}

/// A grant copied into a fresh table, so it is written after the role it belongs to.
fn fresh(g: &Item) -> Item {
    let mut t = Table::new();
    g.as_table()
        .into_iter()
        .flat_map(|g| g.iter())
        .for_each(|(k, v)| drop(t.insert(k, v.clone())));
    Item::Table(t)
}

/// `text` with the grants laid back under the roles of the owner's bots in `bots`.
pub(super) fn laid(
    text: String,
    kept: &[(String, Table)],
    bots: &[Value],
) -> Result<String, String> {
    let alive: Vec<String> = bots
        .iter()
        .filter(|b| b["system"] != true)
        .map(key_of)
        .collect();
    let kept: Vec<&(String, Table)> = kept.iter().filter(|(k, _)| alive.contains(k)).collect();
    if kept.is_empty() {
        return Ok(text);
    }
    let mut doc: DocumentMut = text.parse().map_err(|e| format!("config.toml: {e}"))?;
    for (key, grants) in kept {
        let Some(role) = doc["roles"][key.as_str()].as_table_mut() else {
            continue;
        };
        let mut all = Table::new();
        all.set_implicit(true);
        grants
            .iter()
            .for_each(|(id, g)| drop(all.insert(id, fresh(g))));
        role.insert("connectors", Item::Table(all));
    }
    Ok(doc.to_string())
}
