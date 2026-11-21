//! What was handed to each conversation's agent (the event or task, the notes and the owner's words; never the memory), waiting for the turn it starts.
use std::collections::BTreeMap;
use std::sync::Mutex;

/// What was handed to each conversation's agent and no turn has taken yet.
static TOLD: Mutex<BTreeMap<String, Vec<String>>> = Mutex::new(BTreeMap::new());

/// `text` was handed to the agent of `conv` (the event or task, the notes and the owner's words; not the memory): the turn it starts, or the next one, began with it.
pub fn told(conv: &str, text: &str) {
    let mut told = TOLD.lock().unwrap();
    let list = told.entry(conv.to_string()).or_default();
    list.push(text.to_string());
    list.drain(..list.len().saturating_sub(10));
}

/// What was handed to `conv` and no turn took yet, taken by the turn that starts now.
pub(super) fn taken(conv: &str) -> Vec<String> {
    TOLD.lock().unwrap().remove(conv).unwrap_or_default()
}
