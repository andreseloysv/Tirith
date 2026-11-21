//! Moves kept from older names.
use rusqlite::{params, Connection};

/// Moves kept from older names, run at every start and a no-op once done: the page's model choice of a renamed role is copied to its new name when that one has none (so a rename never loses the chosen model again), and the conversation of no project — its chat rows, its cards and its managers' sessions — becomes the `manager-general` system bot's, history intact.
pub(super) fn migrate(db: &Connection) {
    copy_renamed_models(db);
    let moves = [
        "UPDATE chat SET project = 'manager-general' WHERE project = ''",
        "UPDATE owner_items SET project = 'manager-general' WHERE project = ''",
        "INSERT OR IGNORE INTO kv (key, value) SELECT key || 'manager-general', value FROM kv WHERE key LIKE 'manager_session:%:'",
        "DELETE FROM kv WHERE key LIKE 'manager_session:%:'",
    ];
    moves.iter().for_each(|sql| drop(db.execute(sql, [])));
}

/// The page's model choice of each renamed role, copied to its new name when that one has none.
fn copy_renamed_models(db: &Connection) {
    const RENAMES: &str = concat!(
        "workers>worker;reviewer>ceo;impr",
        "over>ceo;lead>manager-general"
    );
    RENAMES
        .split(';')
        .filter_map(|m| m.split_once('>'))
        .for_each(|(old, new)| {
            drop(db.execute(
                "INSERT OR IGNORE INTO kv (key, value) SELECT ?1, value FROM kv WHERE key = ?2",
                params![format!("models:{new}"), format!("models:{old}")],
            ))
        });
}
