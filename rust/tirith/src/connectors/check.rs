//! The strict check of `[[connectors]]` and of the grants that name them: a typo, an unknown connector or a half-defined one is an error, never silence.
use super::{grants, read, row, Connector};

/// True when `v` is an array of non-empty strings.
pub(super) fn string_list(v: &toml::Value) -> bool {
    v.as_array().is_some_and(|a| {
        a.iter()
            .all(|x| x.as_str().is_some_and(|s| !s.trim().is_empty()))
    })
}

/// The rows of `[[connectors]]`, each a table.
fn tables(overlay: &toml::Table) -> Result<Vec<toml::Table>, String> {
    match overlay.get("connectors") {
        None => Ok(vec![]),
        Some(toml::Value::Array(a)) => a
            .iter()
            .map(|r| r.as_table().cloned().ok_or("each row is a table".into()))
            .collect(),
        Some(_) => Err("write each as a [[connectors]] row".into()),
    }
}

/// The first id defined twice.
fn twice(connectors: &[Connector]) -> Option<String> {
    connectors
        .iter()
        .enumerate()
        .find(|(i, c)| connectors[..*i].iter().any(|d| d.id == c.id))
        .map(|(_, c)| format!("connector '{}' is defined twice", c.id))
}

/// The connectors of config.toml (`overlay`), checked, and every role's grants checked against them (`roles`, the factory's resolved roles).
pub fn checked(overlay: &toml::Table, roles: &toml::Table) -> Result<Vec<Connector>, String> {
    let wrap = |e: String| format!("connectors: {e}");
    let tables = tables(overlay).map_err(wrap)?;
    let connectors: Vec<Connector> = tables.iter().map(read::connector).collect();
    tables
        .iter()
        .enumerate()
        .find_map(|(i, t)| row::wrong(i, t))
        .or_else(|| twice(&connectors))
        .or_else(|| grants::wrong(roles, &connectors))
        .map_or(Ok(connectors), |e| Err(wrap(e)))
}
