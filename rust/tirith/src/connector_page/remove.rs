//! Remove: a connector's `[[connectors]]` row and every bot's grant of it out of config.toml (the config writer, toml_edit, every other line and comment kept), and with them each one-bot block kept of it, each bot's sign-in to it and what the page saw of it.
use super::seen;
use crate::app::Shared;
use serde_json::json;
use toml_edit::DocumentMut;

/// `text` without connector `id`'s row and without any role's grant of it.
pub fn removed(text: &str, id: &str) -> Result<String, String> {
    let mut doc: DocumentMut = text.parse().map_err(|e| format!("config.toml: {e}"))?;
    let rows = doc
        .get_mut("connectors")
        .and_then(|c| c.as_array_of_tables_mut());
    let rows = rows.ok_or_else(|| format!("no connector '{id}' in config.toml"))?;
    let before = rows.len();
    rows.retain(|t| t.get("id").and_then(|v| v.as_str()) != Some(id));
    if rows.len() == before {
        return Err(format!("no connector '{id}' in config.toml"));
    }
    if rows.is_empty() {
        doc.remove("connectors");
    }
    let roles = doc.get_mut("roles").and_then(|r| r.as_table_like_mut());
    for (_, role) in roles.into_iter().flat_map(|r| r.iter_mut()) {
        let Some(role) = role.as_table_like_mut() else {
            continue;
        };
        let grants = role
            .get_mut("connectors")
            .and_then(|c| c.as_table_like_mut());
        if grants.map(|g| (g.remove(id), g.is_empty()).1) == Some(true) {
            role.remove("connectors");
        }
    }
    Ok(doc.to_string())
}

/// Removes connector `id` and every grant of it; what was done.
pub fn remove(app: &Shared, id: &str) -> Result<String, String> {
    crate::projects::rewrite(app, |t| removed(t, id))?;
    let sql = "SELECT key FROM kv WHERE key LIKE 'connector-block:%'";
    let blocks: Vec<String> = app.query(sql, [], |r| r.get(0));
    let ours = |k: &&String| k.ends_with(&format!(":{id}"));
    blocks.iter().filter(ours).for_each(|k| app.set_kv(k, None));
    seen::forget(app, id);
    let bots = crate::bots::all(app);
    let owners = bots.iter().filter(|b| b["system"] != true);
    for bot in owners.filter_map(|b| b["id"].as_str()) {
        for name in crate::oauth::sign_in_names(id) {
            crate::secrets::drop_name(app, bot, &name)?;
        }
    }
    let said = format!("connector {id} removed with every grant of it");
    app.event("", "", "connector-remove", "owner", &said, "");
    app.emit("connectors", json!({}));
    app.emit("bots", json!({}));
    Ok(said)
}
