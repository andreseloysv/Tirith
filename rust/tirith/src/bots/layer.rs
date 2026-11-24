//! A changed system bot's layer over its factory role, in config.toml.
use super::written::key_of;
use crate::server::s;
use serde_json::Value;
use toml_edit::{DocumentMut, Item, Table};
/// What the owner changed of a system bot (its name, show, session, access and face) is laid into its role's table (a real table, created with `entry`), its other keys untouched.
pub(super) fn lay_system(doc: &mut DocumentMut, b: &Value) {
    let key = key_of(b);
    let roles = doc
        .as_table_mut()
        .entry("roles")
        .or_insert_with(|| Item::Table(Table::new()));
    let role = roles
        .as_table_mut()
        .map(|r| r.entry(&key).or_insert_with(|| Item::Table(Table::new())));
    let _ = role.map(|role| {
        (
            role["name"],
            role["show"],
            role["session"],
            role["access"],
            role["face"]["shape"],
            role["face"]["color"],
        ) = (
            toml_edit::value(s(b, "name")),
            toml_edit::value(b["show"] != false),
            toml_edit::value(s(b, "session")),
            toml_edit::value(s(b, "access")),
            toml_edit::value(s(b, "shape")),
            toml_edit::value(s(b, "color")),
        );
        for k in super::look::KEYS {
            if !s(b, k).is_empty() {
                role["face"][k] = toml_edit::value(s(b, k));
            }
        }
        if let Some(a) = b["gangle"].as_f64() {
            role["face"]["gangle"] = toml_edit::value(a);
        }
    });
}
