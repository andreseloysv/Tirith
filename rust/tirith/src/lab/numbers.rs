//! An experiment's numbers from the store: each side's runs (their mean tokens, dollars and time) and B's comparisons.
use super::rule::{Side, Tally};
use super::saved::less;
use crate::app::Shared;
use rusqlite::params;

const SIDE: &str = "SELECT COUNT(*), AVG(COALESCE(tokens_in, 0) + COALESCE(tokens_out, 0) + COALESCE(tokens_cache_read, 0) + COALESCE(tokens_cache_write, 0)), CASE WHEN COUNT(cost_usd) = COUNT(*) THEN AVG(cost_usd) END, AVG(COALESCE(ms, 0)) FROM turns WHERE experiment = ?1 AND variant = ?2 AND id NOT IN (SELECT a_turn FROM lab_comparisons WHERE experiment = ?1 AND verdict = 'void' UNION SELECT b_turn FROM lab_comparisons WHERE experiment = ?1 AND verdict = 'void')";
const TALLY: &str = "SELECT COALESCE(SUM(verdict = 'win'), 0), COALESCE(SUM(verdict = 'tie'), 0), COALESCE(SUM(verdict = 'lose'), 0), COALESCE(SUM(COALESCE(verdict, '') <> 'void' AND b.status = 'failed' AND a.status <> 'failed'), 0), (SELECT COUNT(*) FROM turns WHERE experiment = ?1 AND variant = 'B' AND reaction = -1), COALESCE(SUM(verdict = 'void'), 0) FROM lab_comparisons c JOIN turns a ON a.id = c.a_turn JOIN turns b ON b.id = c.b_turn WHERE c.experiment = ?1";
const PAIRS: &str = "SELECT COALESCE(a.tokens_in, 0) + COALESCE(a.tokens_out, 0) + COALESCE(a.tokens_cache_read, 0) + COALESCE(a.tokens_cache_write, 0), COALESCE(b.tokens_in, 0) + COALESCE(b.tokens_out, 0) + COALESCE(b.tokens_cache_read, 0) + COALESCE(b.tokens_cache_write, 0), a.cost_usd, b.cost_usd, COALESCE(a.ms, 0), COALESCE(b.ms, 0) FROM lab_comparisons c JOIN turns a ON a.id = c.a_turn JOIN turns b ON b.id = c.b_turn WHERE c.experiment = ?1 AND COALESCE(c.verdict, '') <> 'void'";

/// One side's runs in experiment `id`.
pub fn side(app: &Shared, id: i64, variant: &str) -> Side {
    let rows = app.query(SIDE, params![id, variant], |r| {
        Ok(Side {
            runs: r.get(0)?,
            tokens: r.get::<_, Option<f64>>(1)?.unwrap_or(0.0),
            cost: r.get(2)?,
            ms: r.get::<_, Option<f64>>(3)?.unwrap_or(0.0),
        })
    });
    rows.into_iter().next().unwrap_or_default()
}

/// B's comparisons in experiment `id`, its new failures and its 👎.
pub fn tally(app: &Shared, id: i64) -> Tally {
    type Pair = (i64, i64, Option<f64>, Option<f64>, i64, i64);
    let pairs: Vec<Pair> = app.query(PAIRS, [id], |r| {
        Ok((
            r.get(0)?,
            r.get(1)?,
            r.get(2)?,
            r.get(3)?,
            r.get(4)?,
            r.get(5)?,
        ))
    });
    let each = |f: &dyn Fn(&Pair) -> Option<(f64, f64)>| -> Vec<f64> {
        pairs
            .iter()
            .filter_map(f)
            .filter_map(|(a, b)| less(a, b))
            .collect()
    };
    let pair_tokens = each(&|p| Some((p.0 as f64, p.1 as f64)));
    let pair_costs = each(&|p| p.2.zip(p.3));
    let pair_times = each(&|p| Some((p.4 as f64, p.5 as f64)));
    let rows = app.query(TALLY, [id], |r| {
        Ok(Tally {
            pair_times: pair_times.clone(),
            pair_tokens: pair_tokens.clone(),
            pair_costs: pair_costs.clone(),
            wins: r.get(0)?,
            ties: r.get(1)?,
            losses: r.get(2)?,
            new_failures: r.get(3)?,
            down: r.get(4)?,
            voids: r.get(5)?,
        })
    });
    rows.into_iter().next().unwrap_or_default()
}

/// How many of experiment `id`'s comparisons wait for the judge's verdict.
pub fn unjudged(app: &Shared, id: i64) -> i64 {
    let sql = "SELECT COUNT(*) FROM lab_comparisons WHERE experiment = ?1 AND verdict IS NULL";
    app.query(sql, [id], |r| r.get(0))
        .into_iter()
        .next()
        .unwrap_or(0)
}
