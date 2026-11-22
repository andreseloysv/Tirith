//! TOML as Codex's settings are read from it.
use std::path::Path;
use toml::{Table, Value};

pub(super) fn file(path: &Path) -> Table {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|t| t.parse().ok())
        .unwrap_or_default()
}

/// A `-c` value: TOML, else the words as they are.
pub(super) fn value(raw: &str) -> Value {
    format!("v = {raw}")
        .parse::<Table>()
        .ok()
        .and_then(|t| t.get("v").cloned())
        .unwrap_or_else(|| Value::String(raw.trim_matches('"').into()))
}

pub(super) fn deeper<'a>(v: &'a Value, path: &str) -> Option<&'a Value> {
    path.split('.').try_fold(v, |at, part| at.get(part))
}
