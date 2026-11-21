//! One computation of every measure, in the order the page shows them, from what one moment of the daemon holds.
use super::metric::{iso, Metric};
use super::{cards, flow, idle, interruptions, mood, trouble, unapproved, work};
use crate::app::{now_s, Lane, Shared};
use std::sync::Arc;
/// The moment the measures are computed at: the configuration, now, a day ago (also as a database time) and the lanes.
pub(super) struct Ctx<'a> {
    pub(super) app: &'a Shared,
    pub(super) cfg: Arc<crate::config::Config>,
    pub(super) now: i64,
    pub(super) day: i64,
    pub(super) since: String,
    pub(super) lanes: Vec<Lane>,
}
impl<'a> Ctx<'a> {
    fn new(app: &'a Shared) -> Self {
        let cfg = app.cfg();
        let now = now_s() as i64;
        let day = now - 86400;
        let lanes: Vec<Lane> = app.lanes.lock().unwrap().values().cloned().collect();
        Ctx {
            app,
            cfg,
            now,
            day,
            since: iso(day),
            lanes,
        }
    }
    /// The measure `id` as the factory describes it.
    pub(super) fn mk(&self, id: &'static str, value: f64, live: bool, facts: String) -> Metric {
        Metric::new(self.cfg.factory.metric(id), id, value, live, facts)
    }
}
pub(super) fn compute(app: &Shared) -> Vec<Metric> {
    let c = Ctx::new(app);
    let w = work::work(&c);
    let merged = work::merged(&c);
    vec![
        work::merged_metric(&c, &w, &merged),
        flow::median(&c, &merged),
        flow::first_try(&c, &merged),
        idle::idle(&c, &w.open),
        idle::stuck(&c),
        trouble::main_red(&c),
        trouble::failed(&c),
        trouble::owner(&c),
        interruptions::metric(&c, merged.len()),
        cards::needless(&c),
        mood::satisfaction(&c),
        unapproved::unapproved(&c),
    ]
}
