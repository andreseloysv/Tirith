//! The facts a CEO pass reads: the red metrics (or all of them, on the scheduled daily pass), the measures and eval scores by factory version (the stamps), the last lanes with their stamp, failing checks, plan progress, owner questions.
use super::facts::owner_questions;
use super::metric::iso;
use super::plans::plans;
use crate::app::{now_s, tail, Lane, Shared};
use serde_json::Value;
/// The last 30 lanes, newest first.
fn last_lanes(app: &Shared) -> String {
    let mut lanes: Vec<Lane> = app.lanes.lock().unwrap().values().cloned().collect();
    lanes.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    lanes
        .iter()
        .take(30)
        .map(|l| {
            format!(
                "- {} [{}] {}:{} {} tries {} {} {}: {}\n",
                l.id,
                l.status,
                l.project,
                l.step,
                l.item,
                l.attempts + 1,
                l.created_at,
                l.factory,
                l.note.lines().next().unwrap_or("")
            )
        })
        .collect()
}
fn run_line(r: &Value) -> String {
    format!(
        "- {} check '{}' {}:\n```\n{}\n```\n",
        r["lane"].as_str().unwrap_or(""),
        r["stage"].as_str().unwrap_or(""),
        r["status"].as_str().unwrap_or(""),
        tail(std::path::Path::new(r["log"].as_str().unwrap_or("")), 15)
    )
}
/// The last 5 checks failed or timed out in 24 h, with the end of their logs.
fn failing_runs(app: &Shared) -> String {
    let day = iso(now_s().saturating_sub(86400) as i64);
    app.objects("SELECT lane, stage, status, COALESCE(log, '') FROM runs WHERE status IN ('failed', 'timeout') AND started_at >= ?1 ORDER BY id DESC LIMIT 5", [&day], &["lane", "stage", "status", "log"]).iter().map(run_line).collect()
}
/// A table's rows, one per line: grouped by factory version (the stamps), so two versions compare here.
fn grouped(app: &Shared, sql: &str, cols: &[&str]) -> String {
    app.objects(sql, [], cols)
        .iter()
        .map(|r| format!("- {r}\n"))
        .collect::<String>()
}
pub(super) fn brief(app: &Shared, why: &str) -> String {
    let lanes = last_lanes(app);
    let runs = failing_runs(app);
    let versions = grouped(app, "SELECT factory, COUNT(*) AS merged, CAST(100 * AVG(json_extract(data, '$.attempts') = 0) AS INTEGER) AS first_try_pct, MIN(json_extract(data, '$.created_at')) AS first_lane FROM lanes WHERE factory != '' AND json_extract(data, '$.status') = 'merged' GROUP BY factory ORDER BY first_lane", &["factory", "merged", "first_try_pct", "first_lane"]);
    let scores = grouped(app, "SELECT factory, eval, label, COUNT(*) AS n FROM scores WHERE factory != '' GROUP BY factory, eval, label ORDER BY factory, eval, label", &["factory", "eval", "label", "n"]);
    let stamped = grouped(app, "SELECT factory, 'card.opened' AS kind, COUNT(*) AS n FROM owner_items WHERE factory != '' GROUP BY factory UNION ALL SELECT factory, kind, COUNT(*) AS n FROM events WHERE factory != '' GROUP BY factory, kind ORDER BY factory, kind", &["factory", "kind", "n"]);
    let plans = plans(app);
    let qs: String = owner_questions(app)
        .iter()
        .map(|q| format!("- {q}\n"))
        .collect();
    format!("# Red metrics\n{why}\n\n# By factory version (oldest first)\n{versions}# Eval scores by factory version\n{scores}# Cards and events by factory version and kind\n{stamped}\n# Last lanes (newest first: id, status, project:step, item, tries, created, factory stamp, note)\n{lanes}\n# Failing checks (last 24 h)\n{runs}\n# Plans\n{plans}\n# Open owner questions\n{qs}")
}
