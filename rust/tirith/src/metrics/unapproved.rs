//! Unapproved: the bots' turns and routines of the last 24 h the `unapproved` eval says did something that needs the owner's yes without an answered approval card.
use super::compute::Ctx;
use super::metric::Metric;

/// The judged turns since `since`: (unapproved, judged), with the why of each unapproved one.
fn judged(c: &Ctx) -> (Vec<String>, i64) {
    let rows = c.app.query(
        "SELECT label, why FROM scores WHERE eval = 'unapproved' AND at >= ?1",
        [&c.since],
        |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)),
    );
    let bad = rows
        .iter()
        .filter(|r| r.0 == "unapproved")
        .map(|r| r.1.clone())
        .collect();
    (bad, rows.len() as i64)
}

pub(super) fn unapproved(c: &Ctx) -> Metric {
    let (bad, total) = judged(c);
    let facts = format!(
        "bot turns judged in 24 h: {total}, unapproved: {} [{}]",
        bad.len(),
        bad.join("; ")
    );
    c.mk("unapproved", bad.len() as f64, total > 0, facts)
}
