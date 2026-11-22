//! Counting each connection and refusal, and an event for each new one.
use super::dests::DESTS;
use crate::{
    app::{now_iso, now_s, Shared},
    sandbox::owner::owner,
};
use std::collections::{HashMap, HashSet};
use std::sync::{LazyLock, Mutex};

/// The (who, host:port) pairs already recorded, and refusals recorded in the last minute.
static SEEN: LazyLock<Mutex<HashSet<String>>> = LazyLock::new(Default::default);
static REFUSED: LazyLock<Mutex<HashMap<String, u64>>> = LazyLock::new(Default::default);

pub fn record(app: &Shared, who: &str, host: &str, port: u16, rule: Option<&str>) {
    let (lane, project) = owner(app, who);
    let host = &crate::setup::Hidden::in_home(&app.home).hide(host); // blocks read the real one, people this
    count(host, &lane, rule.is_some());
    let _ = is_new(&format!("{who}|{host}:{port}"), rule.is_some()).then(|| {
        app.event(
            &lane,
            &project,
            if rule.is_some() { "net-blocked" } else { "net" },
            "proxy",
            &format!("{host}:{port}"),
            "",
        )
    });
}

/// One more connection (or refusal) to `host`, by `lane`, now.
fn count(host: &str, lane: &str, refused: bool) {
    let mut dests = DESTS.lock().unwrap();
    let d = dests.entry(host.to_string()).or_default();
    let counter = if refused {
        &mut d.refused
    } else {
        &mut d.count
    };
    *counter += 1;
    d.last = now_iso();
    d.lanes.insert(lane.to_string());
}

/// Whether this pair gets an event: the first connection, or a refusal not recorded in the last minute.
fn is_new(key: &str, refused: bool) -> bool {
    match refused {
        true => REFUSED
            .lock()
            .unwrap()
            .insert(key.to_string(), now_s())
            .is_none_or(|t| now_s() - t >= 60),
        false => SEEN.lock().unwrap().insert(key.to_string()),
    }
}
