//! config.toml changed by the guide: one change at a time, its comments kept, readable by the owner alone.
use std::path::Path;
use toml_edit::{DocumentMut, Item, Table};

pub fn edit(
    home: &Path,
    change: impl FnOnce(&mut Table) -> Result<(), String>,
) -> Result<(), String> {
    let file = home.join("config.toml");
    let failed = |e: std::io::Error| format!("{}: {e}", file.display());
    let text = match std::fs::read_to_string(&file) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(failed(e)),
    };
    let mut doc: DocumentMut = text.parse().map_err(|e| {
        format!(
            "{} does not parse, so it stays as it is: {e}",
            file.display()
        )
    })?;
    change(doc.as_table_mut())?;
    let out = doc.to_string();
    if out == text {
        return Ok(());
    }
    std::fs::create_dir_all(home).map_err(failed)?;
    let new = home.join(format!(".config.toml.{}", std::process::id()));
    std::fs::write(&new, out).map_err(failed)?;
    crate::os::set_mode(&new, 0o600).map_err(failed)?;
    std::fs::rename(&new, &file).map_err(failed)
}

/// The table at `path`, made as `[a.b]` where it is missing.
pub fn table<'a>(root: &'a mut Table, path: &[&str]) -> Result<&'a mut Table, String> {
    let mut here = root;
    for (i, key) in path.iter().enumerate() {
        if !here.contains_key(key) {
            let mut new = Table::new();
            new.set_implicit(i + 1 < path.len());
            here.insert(key, Item::Table(new));
        }
        here = here
            .get_mut(key)
            .and_then(Item::as_table_mut)
            .ok_or_else(|| format!("config.toml: [{}] is not a table", path[..=i].join(".")))?;
    }
    Ok(here)
}
