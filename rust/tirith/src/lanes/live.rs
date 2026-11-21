//! Live messages for a lane whose agent still runs: told while it works, taken once.
use std::collections::BTreeMap;
use std::sync::Mutex;

static LIVE: Mutex<BTreeMap<String, Vec<String>>> = Mutex::new(BTreeMap::new());
pub fn tell(id: &str, text: &str) {
    LIVE.lock()
        .unwrap()
        .entry(id.into())
        .or_default()
        .push(text.into());
}
pub fn drain(id: &str) -> Vec<String> {
    LIVE.lock().unwrap().remove(id).unwrap_or_default()
}
