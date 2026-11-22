//! One computation: episodes opened, fixed, noted and reviewed, the other turn roles' scheduled passes and the routines gone worse; the page gets the result.
use super::api::payload;
use super::book::BOOK;
use super::compute::compute;
use super::episode::episode;
use super::metric::{describe, iso, Metric};
use super::pass::{daily_due, under_budget};
use super::review::{delayed_review, start_review};
use crate::app::{now_s, Shared};
use crate::manager;
use serde_json::json;
/// The reviews due now: (the measure's id, what the CEO pass reads of it).
type Due = Vec<(&'static str, String)>;
/// Every measure's episode: the page's copy kept in the book, the reviews due and the measures red since now.
fn record<'a>(app: &Shared, metrics: &'a [Metric], now: u64, after: u64) -> (Due, Vec<&'a Metric>) {
    let (mut due, mut notes, mut last) = (vec![], vec![], vec![]);
    for m in metrics {
        let (fresh, due_now, from) = episode(app, m.id, m.red, now, after);
        let _ = fresh.then(|| notes.push(m));
        let _ = due_now.then(|| due.push((m.id, describe(m))));
        last.push(json!({"id": m.id, "label": m.label, "value": m.value, "target": m.target, "at_least": m.at_least, "red": m.red, "since": from.map(|s| iso(s as i64)), "facts": m.facts}));
    }
    BOOK.lock().unwrap().last = json!(last);
    (due, notes)
}
/// A red main is not reported again here: its project's manager already got it from the merge queue, with the facts.
fn announce(app: &Shared, notes: Vec<&Metric>) {
    notes
        .into_iter()
        .filter(|m| m.id != "main_red")
        .for_each(|m| {
            let _ = (m.id == "idle").then(|| app.sched_wake.notify_one()); // the mechanical fix: wake the scheduler and the autopilot
            manager::emit_event(
                app,
                "",
                "metric.red",
                &format!("health: {}. Get it moving, and never stop a lane that works: it is how things move.", describe(m)),
                None,
            );
        });
}
pub(super) fn tick(app: &Shared) {
    let metrics = compute(app);
    let now = now_s();
    let (ceo, after) = delayed_review(app);
    let (mut due, notes) = record(app, &metrics, now, after);
    announce(app, notes);
    let budget = under_budget(app, &ceo, now);
    let _ = daily_due(app, &ceo).then(|| {
        let all = metrics.iter().map(describe).collect::<Vec<_>>().join("\n");
        due.push((
            "daily",
            format!("Scheduled CEO pass (no red measure needed).\n{all}"),
        ))
    });
    let _ = (!due.is_empty() && !ceo.is_empty() && budget).then(|| {
        let ids = due.iter().map(|d| d.0).collect::<Vec<_>>().join(",");
        let why = due.iter().map(|d| d.1.as_str()).collect::<Vec<_>>();
        start_review(app, ceo.clone(), ids, why.join("\n"))
    });
    super::roles::scheduled(app, &ceo, now);
    super::worse::check(app);
    app.emit("metrics", payload(app));
}
