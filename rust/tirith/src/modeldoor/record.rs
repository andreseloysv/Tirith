//! Every call the door forwarded: who, the model, the status, the tokens and the dollars (`door_calls`), and what a day has spent.
use super::tally::Tally;
use crate::app::{now_iso, Shared};
use crate::turns::{price_of, Usage};
use rusqlite::params;

/// One call, as it ended.
pub struct Call {
    pub alias: String,
    pub who: String,
    pub role: String,
    pub model: String,
    pub status: u16,
    pub ms: i64,
}

/// Keeps the call with its tokens; its cost is the config's price for its model (none without one: never a guess).
pub fn keep(app: &Shared, c: &Call, t: &Tally) {
    let mut usage = Usage::default();
    (usage.input, usage.output, usage.seen) = (t.input, t.output, t.seen);
    (usage.cache_read, usage.cache_write) = (t.cache_read, t.cache_write);
    let cost = t
        .seen
        .then(|| price_of(&app.cfg().raw, &c.model, &usage))
        .flatten();
    app.insert(
        "INSERT INTO door_calls (at, alias, who, role, model, status, ms, tokens_in, tokens_out, tokens_cache_read, tokens_cache_write, cost_usd) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",
        params![now_iso(), c.alias, c.who, c.role, c.model, c.status, c.ms, t.input, t.output, t.cache_read, t.cache_write, cost],
    );
}

/// What the door's calls cost since the start of today (UTC), of one role or of all.
pub fn spent_today(app: &Shared, role: Option<&str>) -> f64 {
    let day = now_iso()[..10].to_string();
    let sql = "SELECT COALESCE(SUM(cost_usd), 0) FROM door_calls WHERE at >= ?1 AND (?2 = '' OR role = ?2)";
    app.query(sql, params![day, role.unwrap_or("")], |r| {
        r.get::<_, f64>(0)
    })
    .first()
    .copied()
    .unwrap_or(0.0)
}

/// Why `role` may not call now: a budget of the machine or of its own is spent.
pub fn over_budget(app: &Shared, role: &str) -> Option<String> {
    let (all, own) = (super::on::machine(app), super::on::role(app, role));
    let machine = all.filter(|b| spent_today(app, None) >= *b).map(|b| {
        format!(
            "the machine's daily budget of ${b:.2} for the model door is spent ([door] budget_usd)"
        )
    });
    machine.or_else(|| own.filter(|b| spent_today(app, Some(role)) >= *b).map(|b| format!("{role}'s daily budget of ${b:.2} for the model door is spent (door_budget in [roles.{role}])")))
}

/// The last `n` calls, the last last, as JSON objects (the cost a number, or null without a price).
pub fn recent(app: &Shared, n: u32) -> Vec<serde_json::Value> {
    let sql = format!("SELECT at, alias, who, role, model, status, ms, tokens_in, tokens_out, tokens_cache_read, tokens_cache_write, cost_usd FROM (SELECT * FROM door_calls ORDER BY id DESC LIMIT {n}) ORDER BY id");
    app.query(&sql, [], |r| {
        let text = |i| r.get::<_, String>(i).unwrap_or_default();
        let num = |i| r.get::<_, Option<i64>>(i).ok().flatten().unwrap_or(0);
        Ok(serde_json::json!({"at": text(0), "alias": text(1), "who": text(2), "role": text(3), "model": text(4), "status": num(5), "ms": num(6),
            "tokens_in": num(7), "tokens_out": num(8), "tokens_cache_read": num(9), "tokens_cache_write": num(10),
            "cost_usd": r.get::<_, Option<f64>>(11).ok().flatten()}))
    })
}
