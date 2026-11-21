//! What went wrong or waits: projects whose main is red, lanes failed in 24 h, questions open for the owner.
use super::compute::Ctx;
use super::facts::owner_questions;
use super::metric::{secs, Metric};
pub(super) fn main_red(c: &Ctx) -> Metric {
    let red: Vec<String> = c
        .cfg
        .projects
        .iter()
        .filter_map(|p| {
            c.app
                .kv(&format!("main_red:{}", p.id))
                .map(|v| format!("{}: {v}", p.id))
        })
        .collect();
    c.mk("main_red", red.len() as f64, true, red.join("; "))
}
pub(super) fn failed(c: &Ctx) -> Metric {
    let failed: Vec<String> = c
        .lanes
        .iter()
        .filter(|l| {
            l.status == "failed"
                && l.ended_at
                    .as_deref()
                    .and_then(secs)
                    .is_some_and(|e| e >= c.day)
        })
        .map(|l| format!("{}: {}", l.id, l.note.lines().next().unwrap_or("")))
        .collect();
    c.mk("failed", failed.len() as f64, false, failed.join("; "))
}
pub(super) fn owner(c: &Ctx) -> Metric {
    let qs = owner_questions(c.app);
    c.mk("owner", qs.len() as f64, false, qs.join("; "))
}
