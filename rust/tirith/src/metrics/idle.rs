//! Work standing still: projects with open items and nothing running, and running lanes gone quiet.
use super::book::BOOK;
use super::compute::Ctx;
use super::facts::paused_of;
use super::metric::Metric;
use crate::app::Lane;
/// Idle: a project with open items, autopilot on and nothing queued, running or merging, and for how many minutes.
fn idle_minutes(c: &Ctx, open: &[(String, usize)]) -> Vec<(String, usize, u64)> {
    let now = c.now as u64;
    let mut book = BOOK.lock().unwrap();
    let active = |p: &String| c.lanes.iter().any(|l| &l.project == p && l.active());
    book.idle_since
        .retain(|p, _| open.iter().any(|(o, _)| o == p) && !active(p));
    open.iter()
        .filter(|(p, _)| !active(p))
        .map(|(p, n)| {
            (
                p.clone(),
                *n,
                (now - *book.idle_since.entry(p.clone()).or_insert(now)) / 60,
            )
        })
        .collect()
}
pub(super) fn idle(c: &Ctx, open: &[(String, usize)]) -> Metric {
    let idle = idle_minutes(c, open);
    let worst = idle.iter().map(|i| i.2).max().unwrap_or(0) as f64;
    let facts = idle
        .iter()
        .map(|(p, n, m)| {
            format!(
                "{p}: {n} open items, nothing running for {m} min{}",
                paused_of(c.app, p)
            )
        })
        .collect::<Vec<_>>()
        .join("; ");
    c.mk("idle", worst, true, facts)
}
/// A running lane quiet (no activity, no log line) for more than half the inactivity limit.
fn stuck_note(c: &Ctx, l: &Lane, half: u64) -> Option<String> {
    let log_age = std::fs::metadata(c.app.home.join(format!("logs/lanes/{}.log", l.id)))
        .and_then(|m| m.modified())
        .map(|t| t.elapsed().map(|d| d.as_secs()).unwrap_or(0))
        .unwrap_or(u64::MAX);
    let quiet = (c.now as u64).saturating_sub(l.last_activity).min(log_age);
    (quiet > half).then(|| format!("{} ({}) quiet {} min", l.id, l.project, quiet / 60))
}
pub(super) fn stuck(c: &Ctx) -> Metric {
    let half = c.cfg.factory.limits.inactivity_minutes * 30;
    let stuck: Vec<String> = c
        .lanes
        .iter()
        .filter(|l| l.status == "running")
        .filter_map(|l| stuck_note(c, l, half))
        .collect();
    c.mk("stuck", stuck.len() as f64, true, stuck.join("; "))
}
