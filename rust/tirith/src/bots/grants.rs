//! A bot's connector grants (`[roles.bot-<id>.connectors.<connector>]`) through a rewrite of config.toml: the page's card does not carry them, so they are taken out of the owner's bot roles before those go and laid back under each bot still there; a deleted bot's go with its role.
use super::written::key_of;
use crate::server::s;
use serde_json::Value;
use toml_edit::{Array, DocumentMut, Item, Table};

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

/// Key `k` of a grant set to the list `v`, or taken off (`tools` when null, the others when empty too).
fn put(g: &mut dyn toml_edit::TableLike, k: &str, v: &Value) {
    let list: Vec<&str> = v
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect();
    if v.is_null() || (k != "tools" && list.is_empty()) {
        g.remove(k);
    } else {
        g.insert(k, toml_edit::value(Array::from_iter(list)));
    }
}

/// The grant a card's bot carries (`grant`: its connector and its table, null to remove it, `connect.rs`) laid into `kept`.
pub(super) fn changed(kept: &mut Vec<(String, Table)>, bots: &[Value]) {
    for b in bots.iter().filter(|b| b["grant"].is_object()) {
        let (key, id, t) = (key_of(b), s(&b["grant"], "connector"), &b["grant"]["table"]);
        if !kept.iter().any(|(k, _)| *k == key) {
            kept.push((key.clone(), Table::new()));
        }
        let Some((_, grants)) = kept.iter_mut().find(|(k, _)| *k == key) else {
            continue;
        };
        if t.is_null() {
            grants.remove(&id);
            continue;
        }
        let item = grants.entry(&id).or_insert(Item::Table(Table::new()));
        if let Some(g) = item.as_table_like_mut() {
            ["tools", "deny", "approval"]
                .iter()
                .for_each(|k| put(g, k, &t[*k]));
        }
    }
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
