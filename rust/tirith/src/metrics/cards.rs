//! Needless cards: the share of the cards resolved in 24 h that were closed without an answer or judged decidable alone.
use super::compute::Ctx;
use super::metric::Metric;
use crate::manager;
/// The cards resolved since `since`, (id, status). An autoclose («its lane no longer waits») whose question a later card of the same project and kind reopened (its question names the closed card's id) counts nowhere: the same question still waits, under the new card.
fn resolved(c: &Ctx) -> Vec<(String, String)> {
    c.app.query("SELECT c.id, c.status FROM owner_items c WHERE c.at >= ?1 AND c.status != 'open' AND NOT (c.status = 'closed' AND COALESCE(c.answer, '') LIKE '%its lane no longer waits%' AND EXISTS (SELECT 1 FROM owner_items s WHERE s.rowid > c.rowid AND s.kind = c.kind AND s.project = c.project AND instr(COALESCE(s.question, ''), c.id) > 0 AND substr(COALESCE(s.question, ''), instr(COALESCE(s.question, ''), c.id) + length(c.id), 1) NOT GLOB '[0-9]' AND substr(COALESCE(s.question, ''), instr(COALESCE(s.question, ''), c.id) - 1, 1) NOT GLOB '[A-Za-z0-9]'))", [&c.since], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
}
/// The resolved cards the card-needed eval judged decidable alone (an answered one too: it bothered the owner needlessly), named by their event's line; the closed ones are counted already.
fn alone(c: &Ctx, resolved: &[(String, String)], closed: &[&str]) -> Vec<String> {
    let judged = c.app.query("SELECT events.summary FROM scores JOIN events ON events.id = scores.event WHERE scores.eval = 'card-needed' AND scores.label = 'alone' AND scores.at >= ?1", [&c.since], |r| r.get::<_, String>(0));
    judged
        .iter()
        .filter_map(|s| manager::card_id_of(s))
        .filter(|id| resolved.iter().any(|c| &c.0 == id) && !closed.contains(&id.as_str()))
        .collect()
}
pub(super) fn needless(c: &Ctx) -> Metric {
    let resolved = resolved(c);
    let closed: Vec<&str> = resolved
        .iter()
        .filter(|c| c.1 == "closed")
        .map(|c| c.0.as_str())
        .collect();
    let alone = alone(c, &resolved, &closed);
    let share = if resolved.is_empty() {
        0.0
    } else {
        100.0 * (closed.len() + alone.len()) as f64 / resolved.len() as f64
    };
    c.mk(
        "needless_cards",
        share,
        !resolved.is_empty(),
        format!(
            "closed without an answer in 24 h: [{}]; answered {}; judged decidable alone: [{}]",
            closed.join(", "),
            resolved.len() - closed.len(),
            alone.join(", ")
        ),
    )
}
