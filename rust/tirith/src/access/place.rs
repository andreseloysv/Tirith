//! Making room for a table laid back at the place it had: every table at or after that place moves one down, so the file reads in its old order again.
use toml_edit::{Item, Table};

/// Moves each table under `t` (itself included) whose place is `from` or later one place down.
pub fn make_room(t: &mut Table, from: usize) {
    if let Some(p) = t.position().filter(|p| *p >= from) {
        t.set_position(p + 1);
    }
    for (_, item) in t.iter_mut() {
        match item {
            Item::Table(sub) => make_room(sub, from),
            Item::ArrayOfTables(rows) => rows.iter_mut().for_each(|r| make_room(r, from)),
            _ => {}
        }
    }
}
