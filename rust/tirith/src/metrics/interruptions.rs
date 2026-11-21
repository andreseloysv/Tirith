//! What reached the owner unasked: phone messages of turns no owner message started, notices on the phones, cards opened.
use super::compute::Ctx;
use super::metric::Metric;
use crate::app::Shared;
type Row = (String, String, String);
/// Phone messages of turns no owner message started: a manager message (at, project, text) counts when the last mark (at, project, unasked) of its conversation before it is unasked — an owner or room message asks, a `turn` route with `phone = true` does not — and a ∅ routine silence never reaches the phones.
pub(super) fn unasked(marks: &[(String, String, bool)], msgs: &[Row]) -> usize {
    msgs.iter()
        .filter(|m| {
            !m.2.trim_start().starts_with('∅')
                && marks
                    .iter()
                    .rposition(|k| k.1 == m.1 && k.0 <= m.0)
                    .is_some_and(|i| marks[i].2)
        })
        .count()
}
/// Rows of three texts since `since`.
fn rows(app: &Shared, sql: &str, since: &str) -> Vec<Row> {
    app.query(sql, [since], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
        ))
    })
}
/// A card answer is the owner's word too: the turn it starts was asked for.
fn asked(k: &str) -> bool {
    matches!(k, "owner.message" | "room.message" | "card.answered")
}
/// What reached the owner unasked since `since`: (phone messages of unasked turns, notices whose route has `phone = true`, cards opened).
fn interruptions(app: &Shared, since: &str) -> (usize, usize, usize) {
    let f = &app.cfg().factory;
    let phone = |k: &str, chat: bool| {
        f.routes_on(k)
            .iter()
            .any(|r| r.3 && if chat { r.2 } else { r.1 == "turn" })
    };
    let evs = rows(app, "SELECT at, project, kind FROM events WHERE at >= ?1 AND (kind LIKE '%.%' OR kind = 'schedule') ORDER BY id", since);
    let marks: Vec<_> = evs
        .iter()
        .filter(|e| asked(&e.2) || phone(&e.2, false))
        .map(|e| (e.0.clone(), e.1.clone(), !asked(&e.2)))
        .collect();
    let cards = app
        .query(
            "SELECT COUNT(*) FROM owner_items WHERE at >= ?1",
            [since],
            |r| r.get::<_, i64>(0),
        )
        .first()
        .copied()
        .unwrap_or(0) as usize;
    let msgs = rows(
        app,
        "SELECT at, project, text FROM chat WHERE at >= ?1 AND role = 'manager' ORDER BY id",
        since,
    );
    let notices = evs.iter().filter(|e| phone(&e.2, true)).count();
    (unasked(&marks, &msgs), notices, cards)
}
pub(super) fn metric(c: &Ctx, merged: usize) -> Metric {
    let (turns, notices, cards) = interruptions(c.app, &c.since);
    let facts = format!("unasked in 24 h: {turns} phone messages of turns no owner message started, {notices} notices with phone = true, {cards} cards, over {merged} merged items");
    c.mk(
        "interruptions",
        (turns + notices + cards) as f64 / merged.max(1) as f64,
        turns + notices + cards > 0,
        facts,
    )
}
