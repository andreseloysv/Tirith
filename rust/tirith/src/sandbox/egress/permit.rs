//! Putting a destination on a list in config.toml (`neobot net permit <host> [--role R]`, a yes on a card).
use crate::app::Shared;
use toml_edit::{Array, DocumentMut, Item, Table};

/// `host` on `[network] allow` (no `role`) or on the `net` of `[roles.<role>]`, once.
pub fn allow(app: &Shared, role: Option<&str>, host: &str) -> Result<(), String> {
    let host = crate::sandbox::rules::clean(host);
    (!host.is_empty())
        .then_some(())
        .ok_or("no destination to allow")?;
    crate::projects::rewrite(app, |text| {
        let mut doc: DocumentMut = text.parse().map_err(|e| format!("config.toml: {e}"))?;
        let (table, name) = match role {
            Some(role) => (child(table(&mut doc, "roles")?, role)?, "net"),
            None => (table(&mut doc, "network")?, "allow"),
        };
        let mut list = Array::new();
        let old = table.get(name).and_then(|i| i.as_value());
        match old.map(|v| (v.as_array(), v.as_str())) {
            Some((Some(a), _)) => a.iter().for_each(|v| list.push(v.clone())),
            Some((_, Some(word))) => list.push(word),
            _ => {}
        }
        if !list.iter().any(|v| v.as_str() == Some(host.as_str())) {
            list.push(host.as_str());
        }
        table.insert(name, toml_edit::value(list));
        Ok(doc.to_string())
    })
}

fn table<'a>(doc: &'a mut DocumentMut, name: &str) -> Result<&'a mut Table, String> {
    let item = doc.entry(name).or_insert(Item::Table(Table::new()));
    item.as_table_mut()
        .ok_or(format!("[{name}] of config.toml is no table"))
}

fn child<'a>(parent: &'a mut Table, name: &str) -> Result<&'a mut Table, String> {
    parent.set_implicit(true);
    let item = parent.entry(name).or_insert(Item::Table(Table::new()));
    item.as_table_mut()
        .ok_or(format!("[roles.{name}] of config.toml is no table"))
}
