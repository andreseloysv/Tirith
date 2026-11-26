//! A bot's `net` list (`[roles.bot-<id>] net = [...]`, the destinations it may reach) through a rewrite of config.toml: the page's card does not carry it, so it is taken out of the owner's bot roles before those go and laid back under each bot still there.
use super::written::key_of;
use toml_edit::{DocumentMut, Item};

/// Each owner's bot role's `net`, by role key, as written.
pub(super) fn taken(doc: &DocumentMut) -> Vec<(String, Item)> {
    let roles = doc.get("roles").and_then(|r| r.as_table());
    let bots = roles.into_iter().flat_map(|r| r.iter());
    bots.filter(|(k, _)| k.starts_with("bot-"))
        .filter_map(|(k, v)| Some((k.to_string(), v.get("net")?.clone())))
        .collect()
}

/// `text` with the lists laid back under the roles of the owner's bots in `bots`.
pub(super) fn laid(
    text: String,
    kept: &[(String, Item)],
    bots: &[serde_json::Value],
) -> Result<String, String> {
    let alive: Vec<String> = bots
        .iter()
        .filter(|b| b["system"] != true)
        .map(key_of)
        .collect();
    let kept: Vec<_> = kept.iter().filter(|(k, _)| alive.contains(k)).collect();
    if kept.is_empty() {
        return Ok(text);
    }
    let mut doc: DocumentMut = text.parse().map_err(|e| format!("config.toml: {e}"))?;
    for (key, net) in kept {
        if let Some(role) = doc["roles"][key.as_str()].as_table_mut() {
            role.insert("net", net.clone());
        }
    }
    Ok(doc.to_string())
}
