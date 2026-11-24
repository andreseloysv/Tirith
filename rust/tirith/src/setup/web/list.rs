//! Each agent as the page shows it: known to this build, installed, and connected, which means it has really
//! worked here: a turn of it ended well, and no connection check of it failed after that turn.
use super::job;
use crate::app::Shared;
use crate::setup::{catalog, find, prove};
use serde_json::{json, Value};

/// Every agent this build knows how to set up (the catalog's, or one config.toml names), in the guide's order.
pub fn rows(app: &Shared) -> Vec<Value> {
    let cfg = app.cfg();
    let rows = catalog::all().into_iter().filter_map(|k| {
        let set = cfg.factory.command(&k.alias);
        (!k.bin.is_empty() || set.is_some()).then_some(())?;
        let installed = find::agent(&k, set.as_deref()).is_some();
        let connected = set.is_some() && installed && worked(app, &k.alias);
        let job = job::get(&k.alias);
        Some(json!({"alias": k.alias, "name": k.name, "installed": installed, "connected": connected, "job": job}))
    });
    rows.collect()
}

/// Its last good turn came after its last failed check: in config.toml alone, or with only failures, it is not.
pub(super) fn worked(app: &Shared, alias: &str) -> bool {
    let last = |sql: &str, args: &[&dyn rusqlite::ToSql]| {
        let got = app.query(sql, args, |r| r.get::<_, i64>(0));
        got.first().copied().unwrap_or(0)
    };
    let good = "SELECT COALESCE(MAX(id), 0) FROM turns WHERE agent = ?1 AND status = 'ok'";
    let failed = "SELECT COALESCE(MAX(id), 0) FROM turns WHERE agent = ?1 AND role = ?2 AND status = 'failed'";
    last(good, &[&alias]) > last(failed, &[&alias, &prove::ROLE])
}
