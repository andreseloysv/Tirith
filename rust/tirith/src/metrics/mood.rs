//! Satisfaction from the owner-mood scores of the last 24 h, per owner message; judged from what the owner writes anyway, never asked.
use super::compute::Ctx;
use super::metric::Metric;
/// How many owner messages came since `since`.
fn owner_messages(c: &Ctx) -> i64 {
    c.app
        .query(
            "SELECT COUNT(*) FROM events WHERE kind = 'owner.message' AND at >= ?1",
            [&c.since],
            |r| r.get::<_, i64>(0),
        )
        .into_iter()
        .next()
        .unwrap_or(0)
}
pub(super) fn satisfaction(c: &Ctx) -> Metric {
    let moods = c.app.query(
        "SELECT label, COUNT(*) FROM scores WHERE eval = 'owner-mood' AND at >= ?1 GROUP BY label",
        [&c.since],
        |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)),
    );
    let mood = |label: &str| {
        moods
            .iter()
            .find(|m| m.0 == label)
            .map(|m| m.1)
            .unwrap_or(0)
    };
    let msgs = owner_messages(c);
    let sat = if msgs > 0 {
        (mood("approval") - mood("correction") - mood("repeat") - mood("noise")) as f64
            / msgs as f64
    } else {
        0.0
    };
    let facts =
        format!("owner mood judged in 24 h: {} approvals, {} corrections, {} repeats, {} noise complaints, {} neutral, over {msgs} owner messages", mood("approval"), mood("correction"), mood("repeat"), mood("noise"), mood("neutral"));
    c.mk("satisfaction", sat, msgs > 0 && !moods.is_empty(), facts)
}
