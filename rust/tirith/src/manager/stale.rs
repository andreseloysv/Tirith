//! Open questions that no longer apply, closed by NeoBot.
use super::cards::card_done;
use super::open::owner_open;
use crate::app::Shared;
use serde_json::{json, Value};
/// Why an open question no longer applies: it proposes turning its project's autopilot off (on) and that is already so.
pub fn stale_reason(question: &str, autopilot_on: bool, free_gb: f64) -> Option<&'static str> {
    let q = question.to_lowercase();
    let Some(()) = (!(q.contains("gb free on this disk") && free_gb > 10.0)).then_some(()) else {
        return Some("the disk has more than 10 GB free again");
    };
    let has = |ws: &[&str]| ws.iter().any(|w| q.contains(w));
    let (off, on) = (
        has(&[
            "turn off",
            "switch off",
            "disable",
            "apagar",
            "desactivar",
            "pausar",
        ]),
        has(&[
            "turn on",
            "switch on",
            "enable",
            "encender",
            "activar",
            "reanudar",
        ]),
    );
    match (q.contains("autopilot"), off, on, autopilot_on) {
        (true, true, false, false) => Some("the autopilot is already off"),
        (true, false, true, true) => Some("the autopilot is already on"),
        _ => None,
    }
}
pub fn close_stale_questions(app: &Shared) {
    for o in owner_open(app) {
        let lane = o["lane"].as_str().unwrap_or("").to_string();
        if !lane.is_empty() && !app.lane(&lane).is_some_and(|l| l.status == "waiting") {
            release_card(app, o);
        } else if lane.is_empty() {
            close_if_stale(app, &o);
        }
    }
}
/// Its lane no longer waits (answered, merged, closed): the card stays open for the owner as a card of its project, same id, kind and question; only a card of no lane can go stale.
fn release_card(app: &Shared, mut o: Value) {
    let id = o["id"].as_str().unwrap_or("").to_string();
    let _ = app.db.lock().unwrap().execute(
        "UPDATE owner_items SET lane = '' WHERE id = ?1 AND status = 'open'",
        rusqlite::params![id],
    );
    o["lane"] = json!("");
    app.emit("owner", o);
}
/// A card of no lane whose question no longer applies is closed with the reason.
fn close_if_stale(app: &Shared, o: &Value) {
    let (id, project, q) = (
        o["id"].as_str().unwrap_or(""),
        o["project"].as_str().unwrap_or(""),
        o["question"].as_str().unwrap_or(""),
    );
    stale_reason(
        q,
        if project.is_empty() {
            app.cfg().autopilot
        } else {
            crate::lanes::autopilot_on(app, project)
        },
        crate::lanes::free_gb(&app.home.join("lanes")),
    )
    .into_iter()
    .for_each(|why| card_done(app, id, "closed", &format!("closed by NeoBot: {why}")));
}
