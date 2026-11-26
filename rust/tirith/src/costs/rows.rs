//! The runs kept in `lane_runs`, newest last, of one project or of all, since a day.
use super::sum::Run;
use crate::app::Shared;

const COLS: &str = "lane, project, step, item, issue, attempt, agent, model, started_at, ended_at, tokens_in, tokens_out, tokens_cache_read, tokens_cache_write, cost_usd, source";

fn text(r: &rusqlite::Row, i: usize) -> String {
    r.get::<_, Option<String>>(i)
        .ok()
        .flatten()
        .unwrap_or_default()
}

fn number(r: &rusqlite::Row, i: usize) -> i64 {
    r.get::<_, Option<i64>>(i).ok().flatten().unwrap_or(0)
}

/// `since` is a day (`2026-11-01`) or a time: runs that ended before it are left out. An empty `project` is every project.
pub fn runs(app: &Shared, project: &str, since: &str) -> Vec<Run> {
    let sql = format!("SELECT {COLS} FROM lane_runs WHERE (?1 = '' OR project = ?1) AND ended_at >= ?2 ORDER BY id");
    app.query(&sql, [project, since], |r| {
        Ok(Run {
            lane: text(r, 0),
            project: text(r, 1),
            step: text(r, 2),
            item: text(r, 3),
            issue: text(r, 4),
            attempt: number(r, 5),
            agent: text(r, 6),
            model: text(r, 7),
            started_at: text(r, 8),
            ended_at: text(r, 9),
            input: number(r, 10),
            output: number(r, 11),
            cache_read: number(r, 12),
            cache_write: number(r, 13),
            cost: r.get::<_, Option<f64>>(14).ok().flatten(),
            source: text(r, 15),
        })
    })
}
