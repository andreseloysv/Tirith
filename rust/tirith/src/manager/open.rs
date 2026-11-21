//! The open questions: the cards still open and the lanes that wait for an answer.
use crate::app::Shared;
use serde_json::{json, Value};
use std::collections::BTreeSet;
/// A lane that asked with `tirith ask` is blocked until answered: it is an open question too.
pub(super) fn waiting_cards(lanes: &[crate::app::Lane]) -> Vec<Value> {
    lanes
        .iter()
        .filter(|l| l.status == "waiting")
        .map(|l| json!({"id": l.id, "at": l.created_at, "kind": "lane", "project": l.project, "status": "open", "factory": l.factory, "role_hash": l.role_hash, "question": format!("lane bloqueada esperando al dueño ({}): {}", l.item, l.question)}))
        .collect()
}
pub fn owner_open(app: &Shared) -> Vec<Value> {
    let cols = [
        "id",
        "at",
        "kind",
        "question",
        "project",
        "status",
        "lane",
        "factory",
        "role_hash",
    ];
    let cards = app.objects("SELECT id, at, kind, question, project, status, lane, factory, role_hash FROM owner_items WHERE status = 'open' ORDER BY rowid", [], &cols);
    // A lane whose question a card carries is that card.
    let carried: BTreeSet<&str> = cards.iter().filter_map(|c| c["lane"].as_str()).collect();
    let lanes: Vec<crate::app::Lane> = app
        .lanes
        .lock()
        .unwrap()
        .values()
        .filter(|l| !carried.contains(l.id.as_str()))
        .cloned()
        .collect();
    [cards, waiting_cards(&lanes)].concat()
}
