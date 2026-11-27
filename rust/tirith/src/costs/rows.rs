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
            kind: "lane".into(),
            bot: String::new(),
        })
    })
}

/// The bots' turns and their judges' turns that ended since `since`, as runs: a turn of a bot's own role is `bot`, a judge's turn is `judge` and has the issue key of the turn it judged. Whatever they spent is in `cost_usd` (a price from config.toml where its agent gave none). No person is in them.
pub fn turn_runs(app: &Shared, since: &str) -> Vec<Run> {
    let cols = "conv, role, COALESCE(issue, ''), agent, model, started_at, ended_at, tokens_in, tokens_out, tokens_cache_read, tokens_cache_write, cost_usd";
    let sql = format!("SELECT {cols} FROM turns WHERE ended_at >= ?1 AND (role = conv AND conv LIKE 'bot-%' OR role != conv) ORDER BY id");
    let cfg = app.cfg();
    let all = app.query(&sql, [since], |r| {
        let (conv, role) = (text(r, 0), text(r, 1));
        let kind = match (role == conv, cfg.factory.is_judge(&role)) {
            (true, _) => "bot",
            (_, true) => "judge",
            _ => "",
        };
        Ok(Run {
            lane: String::new(),
            project: String::new(),
            step: String::new(),
            item: String::new(),
            issue: text(r, 2),
            attempt: 1,
            agent: text(r, 3),
            model: text(r, 4),
            started_at: text(r, 5),
            ended_at: text(r, 6),
            input: number(r, 7),
            output: number(r, 8),
            cache_read: number(r, 9),
            cache_write: number(r, 10),
            cost: r.get::<_, Option<f64>>(11).ok().flatten(),
            source: "turn".into(),
            kind: kind.into(),
            bot: conv,
        })
    });
    all.into_iter().filter(|r| !r.kind.is_empty()).collect()
}
