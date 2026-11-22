//! The edits of a block in config.toml's text, by toml_edit so every other line and comment stays: one bot's grant taken out (with what it was and where, to lay it back exactly) or laid back, and `blocked` set on or taken off a `[[connectors]]` row.
use serde_json::{json, Value};
use toml_edit::{DocumentMut, Item, Table};

fn parsed(text: &str) -> Result<DocumentMut, String> {
    text.parse()
        .map_err(|e| format!("config.toml does not parse, so it stays as it is: {e}"))
}

/// Role `key`'s table.
fn role<'a>(doc: &'a mut DocumentMut, key: &str) -> Option<&'a mut Table> {
    doc.get_mut("roles")?
        .as_table_mut()?
        .get_mut(key)?
        .as_table_mut()
}

/// `text` without role `key`'s grant of `id`, and that grant as kept: its text and its place in the file.
pub fn taken(text: &str, key: &str, id: &str) -> Result<(String, Value), String> {
    let mut doc = parsed(text)?;
    let grants = role(&mut doc, key)
        .and_then(|r| r.get_mut("connectors"))
        .and_then(Item::as_table_like_mut);
    let grant = grants
        .and_then(|g| g.remove(id))
        .ok_or_else(|| format!("{key} is not granted connector '{id}'"))?;
    let position = grant.as_table().and_then(Table::position);
    let mut alone = DocumentMut::new();
    alone.insert("grant", grant);
    let kept = json!({"key": key, "position": position, "toml": alone.to_string()});
    Ok((doc.to_string(), kept))
}

/// `text` with the grant `kept` laid back under role `key` as connector `id`, where it was.
pub fn laid(text: &str, key: &str, id: &str, kept: &Value) -> Result<String, String> {
    let mut doc = parsed(text)?;
    let alone = parsed(kept["toml"].as_str().unwrap_or_default())?;
    let mut grant = alone
        .get("grant")
        .cloned()
        .ok_or("the kept grant cannot be read")?;
    if let (Some(t), Some(p)) = (grant.as_table_mut(), kept["position"].as_u64()) {
        super::place::make_room(doc.as_table_mut(), p as usize);
        t.set_position(p as usize);
    }
    let role = role(&mut doc, key).ok_or_else(|| format!("{key} is gone from config.toml"))?;
    let grants = role.entry("connectors").or_insert_with(|| {
        let mut t = Table::new();
        t.set_implicit(true);
        Item::Table(t)
    });
    let grants = grants
        .as_table_mut()
        .ok_or_else(|| format!("[roles.{key}.connectors] is not a table"))?;
    if grants.contains_key(id) {
        return Err(format!("{key} is granted connector '{id}' again already"));
    }
    grants.insert(id, grant);
    Ok(doc.to_string())
}

/// `text` with connector `id`'s row blocked for every bot (`on`), or not.
pub fn blocked(text: &str, id: &str, on: bool) -> Result<String, String> {
    let mut doc = parsed(text)?;
    let row = doc
        .get_mut("connectors")
        .and_then(Item::as_array_of_tables_mut)
        .and_then(|a| {
            a.iter_mut()
                .find(|t| t.get("id").and_then(|v| v.as_str()) == Some(id))
        })
        .ok_or_else(|| format!("no [[connectors]] row has id '{id}'"))?;
    match on {
        true => drop(row.insert("blocked", toml_edit::value(true))),
        false => drop(row.remove("blocked")),
    }
    Ok(doc.to_string())
}
