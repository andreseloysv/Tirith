//! Passes asked by an event while a pass of the same role runs: kept, and run as one pass when it ends, so an event (a routine gone worse) heard in the middle of a pass is never lost.
use std::collections::BTreeMap;
use std::sync::Mutex;

/// By role: the (metric, why) of each event heard during its pass.
static WAITING: Mutex<BTreeMap<String, Vec<(String, String)>>> = Mutex::new(BTreeMap::new());

/// A pass of `role` was asked (`metric`, `why`) while one runs: kept when an event asked it (the scheduled, red and owner's passes are asked again by their own conditions).
pub(super) fn keep(role: &str, metric: &str, why: &str) {
    let Some(()) = metric.starts_with("event:").then_some(()) else {
        return;
    };
    let mut waiting = WAITING.lock().unwrap();
    let list = waiting.entry(role.to_string()).or_default();
    list.push((metric.to_string(), why.to_string()));
}

/// The pass the events heard during `role`'s last pass ask for, as one: the first one's metric and every why.
pub(super) fn take(role: &str) -> Option<(String, String)> {
    let list = WAITING.lock().unwrap().remove(role)?;
    let metric = list.first()?.0.clone();
    let why: Vec<String> = list.into_iter().map(|(_, w)| w).collect();
    Some((metric, why.join("\n")))
}
