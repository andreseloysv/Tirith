//! The destinations seen, by host, for the page and `neobot net list`.
use crate::{app::Shared, sandbox::blocks::blocked};
use serde_json::{json, Value};
use std::collections::{BTreeSet, HashMap};
use std::sync::{LazyLock, Mutex};

#[derive(Default)]
pub struct Dest {
    pub count: u64,
    pub refused: u64,
    pub last: String,
    pub lanes: BTreeSet<String>,
}

pub static DESTS: LazyLock<Mutex<HashMap<String, Dest>>> = LazyLock::new(Default::default);

/// One `net` / `net-blocked` event group: destination, lane, kind, how many and the last time.
type Row = (String, String, String, u64, String);

/// The destinations seen so far from the `net` / `net-blocked` events, so the page keeps its history.
pub fn seed(app: &Shared) {
    let sql = "SELECT summary, lane, kind, COUNT(*), MAX(at) FROM events WHERE kind IN ('net', 'net-blocked') GROUP BY summary, lane, kind";
    let rows: Vec<Row> = app.query(sql, [], |r| {
        Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
    });
    let mut dests = DESTS.lock().unwrap();
    rows.into_iter().for_each(|row| add(&mut dests, row));
}

/// Adds one event group to its host.
fn add(dests: &mut HashMap<String, Dest>, (dest, lane, kind, n, at): Row) {
    let d = dests
        .entry(
            dest.rsplit_once(':')
                .map(|(h, _)| h)
                .unwrap_or(&dest)
                .to_string(),
        )
        .or_default();
    *(if kind == "net" {
        &mut d.count
    } else {
        &mut d.refused
    }) += n;
    d.last = d.last.clone().max(at);
    d.lanes.insert(lane);
}

/// The destinations, newest first: host, connections, refusals, callers, last seen and the rule blocking it now.
pub fn destinations() -> Value {
    let dests = DESTS.lock().unwrap();
    let mut out: Vec<Value> = dests.iter().map(|(h, d)| json!({"host": h, "count": d.count, "refused": d.refused, "lanes": d.lanes, "last": d.last, "rule": blocked(h)})).collect();
    out.sort_by(|a, b| b["last"].as_str().cmp(&a["last"].as_str()));
    out.truncate(300);
    json!(out)
}
