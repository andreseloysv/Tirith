//! Roles that extend another role: each is laid over its parent, all the way up.
use super::values::merge;

/// A role's table with what it extends under it, all the way up; a missing parent or a cycle keeps `extends` and fails validation.
pub(super) fn resolved(roles: &toml::Table, name: &str, up: usize) -> toml::Value {
    let mut own = roles
        .get(name)
        .cloned()
        .unwrap_or(toml::Value::Table(Default::default()));
    let Some(parent) = own
        .get("extends")
        .and_then(|e| e.as_str())
        .filter(|_| up < 8)
        .map(String::from)
    else {
        return own;
    };
    own.as_table_mut().map(|t| t.remove("extends"));
    let mut base = resolved(roles, &parent, up + 1);
    merge(&mut base, &own);
    let _ = (!(roles.contains_key(&parent) && base.get("extends").is_none())).then(|| {
        base.as_table_mut()
            .map(|t| t.insert("extends".into(), parent.into()))
    }); // stays, so validation names it
    base
}

/// Every role of the table replaced by its resolved self.
pub(super) fn resolve_roles(table: &mut toml::Value) {
    let roles = table["roles"].as_table().cloned().unwrap_or_default();
    roles
        .keys()
        .cloned()
        .collect::<Vec<_>>()
        .iter()
        .for_each(|name| table["roles"][name] = resolved(&roles, name, 0));
}
