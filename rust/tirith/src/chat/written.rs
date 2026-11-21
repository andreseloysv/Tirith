//! Writing a conversation's `chat_keep` into config.toml.
use toml_edit::{DocumentMut, Item, Table, TableLike};

/// `text` of config.toml with a conversation's `chat_keep` set to `words` (or gone with None), by toml_edit so its other sections and comments stay: on its `[[projects]]` table, or on its bot's `[roles.<key>]`.
pub(super) fn written(
    text: &str,
    is_project: bool,
    conv: &str,
    words: Option<&str>,
) -> Result<String, String> {
    let mut doc: DocumentMut = text.parse().map_err(|e| format!("config.toml: {e}"))?;
    let table = if is_project {
        project_table(&mut doc, conv)?
    } else {
        role_table(&mut doc, conv)?
    };
    match words {
        Some(w) => drop(table.insert("chat_keep", toml_edit::value(w))),
        None => drop(table.remove("chat_keep")),
    }
    Ok(doc.to_string())
}

/// The `[[projects]]` table whose id is `conv`.
fn project_table<'a>(
    doc: &'a mut DocumentMut,
    conv: &str,
) -> Result<&'a mut dyn TableLike, String> {
    let list = doc
        .get_mut("projects")
        .and_then(|i| i.as_array_of_tables_mut())
        .ok_or("config.toml has no [[projects]] to write chat_keep on")?;
    let t = list
        .iter_mut()
        .find(|t| t.get("id").and_then(|i| i.as_str()) == Some(conv))
        .ok_or(format!("no [[projects]] with id '{conv}' in config.toml"))?;
    Ok(t)
}

/// The bot's `[roles.<conv>]` table, made when it is not there.
fn role_table<'a>(doc: &'a mut DocumentMut, conv: &str) -> Result<&'a mut dyn TableLike, String> {
    let roles = doc
        .as_table_mut()
        .entry("roles")
        .or_insert_with(|| Item::Table(Table::new()));
    let role = roles
        .as_table_mut()
        .ok_or("config.toml's [roles] is not a table")?
        .entry(conv)
        .or_insert_with(|| Item::Table(Table::new()));
    let t = role
        .as_table_mut()
        .ok_or(format!("config.toml's [roles.{conv}] is not a table"))?;
    Ok(t)
}
