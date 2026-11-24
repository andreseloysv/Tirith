//! Each agent as the page shows it: known to this build, installed, and connected, which means NeoBot's rules have
//! really heard it here: its last connection check passed or, never checked, its hooks reached the rules while a
//! turn of it ended well (an agent in config.toml alone, a stand-in, or one whose hooks never arrived is not).
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
        let connected = set.is_some() && installed && works(app, &k.alias);
        let job = job::get(&k.alias);
        Some(json!({"alias": k.alias, "name": k.name, "installed": installed, "connected": connected, "job": job}))
    });
    rows.collect()
}

/// Its last check passed; never checked, a turn of it ended well while a hook call reached the rules.
pub(super) fn works(app: &Shared, alias: &str) -> bool {
    if let Some(verdict) = app.kv(&prove::verdict_key(alias)) {
        return verdict == "ok";
    }
    let heard = "SELECT COUNT(*) FROM turns t WHERE t.agent = ?1 AND t.status = 'ok' AND EXISTS (SELECT 1 FROM events e WHERE e.kind = 'PreToolUse' AND e.at BETWEEN t.started_at AND t.ended_at)";
    let got = app.query(heard, [alias], |r| r.get::<_, i64>(0));
    got.first().is_some_and(|n| *n > 0)
}
