//! An experiment's numbers from the store: each side's runs (their mean tokens, dollars and time) and B's comparisons.
use super::rule::{Side, Tally};
use crate::app::Shared;
use rusqlite::params;

const SIDE: &str = "SELECT COUNT(*), AVG(COALESCE(tokens_in, 0) + COALESCE(tokens_out, 0)), CASE WHEN COUNT(cost_usd) = COUNT(*) THEN AVG(cost_usd) END, AVG(COALESCE(ms, 0)) FROM turns WHERE experiment = ?1 AND variant = ?2";
const TALLY: &str = "SELECT COALESCE(SUM(verdict = 'win'), 0), COALESCE(SUM(verdict = 'tie'), 0), COALESCE(SUM(verdict = 'lose'), 0), COALESCE(SUM(b.status = 'failed' AND a.status <> 'failed'), 0), (SELECT COUNT(*) FROM turns WHERE experiment = ?1 AND variant = 'B' AND reaction = -1) FROM lab_comparisons c JOIN turns a ON a.id = c.a_turn JOIN turns b ON b.id = c.b_turn WHERE c.experiment = ?1";

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
    let rows = app.query(TALLY, [id], |r| {
        Ok(Tally {
            wins: r.get(0)?,
            ties: r.get(1)?,
            losses: r.get(2)?,
            new_failures: r.get(3)?,
            down: r.get(4)?,
        })
    });
    rows.into_iter().next().unwrap_or_default()
}
