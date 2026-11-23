//! Turns the daemon was stopped or restarted in the middle of: each is marked when it starts, unmarked when its row is kept, and what a start finds marked is kept as a failed row.
use super::record::Turn;
use crate::app::{now_iso, Shared};
use rusqlite::params;

/// The turn `t` starts: it is marked as open.
pub(super) fn mark(app: &Shared, t: &Turn) {
    let sql = "INSERT OR REPLACE INTO open_turns (conv, role, start_us, \"trigger\", routine, started_at, agent, model, effort) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)";
    app.insert(
        sql,
        params![
            t.conv,
            t.role,
            t.start_us,
            t.trigger,
            t.routine,
            t.started_at,
            t.agent,
            t.model,
            t.effort
        ],
    );
}

/// The turn `t` was kept (or never ran): it is no longer open.
pub(super) fn clear(app: &Shared, t: &Turn) {
    let sql = "DELETE FROM open_turns WHERE conv = ?1 AND role = ?2 AND start_us = ?3";
    app.insert(sql, params![t.conv, t.role, t.start_us]);
}

/// Every turn still marked open belongs to a daemon that is gone: its row is kept as `failed`, saying why. At the daemon's start.
pub fn start(app: Shared) {
    let sql =
        "SELECT conv, role, \"trigger\", routine, started_at, agent, model, effort FROM open_turns";
    let rows = app.query(sql, [], |r| {
        (0..8)
            .map(|i| r.get::<_, Option<String>>(i))
            .collect::<Result<Vec<_>, _>>()
    });
    for r in rows {
        let r: Vec<String> = r.into_iter().map(Option::unwrap_or_default).collect();
        let (factory, role_hash) = app.stamp_for(&r[1]);
        let why = "(interrupted: OpenBot was stopped or restarted during this turn)";
        let sql = "INSERT INTO turns (conv, role, \"trigger\", routine, started_at, ended_at, status, agent, model, effort, answer, factory, role_hash) VALUES (?1,?2,?3,?4,?5,?6,'failed',?7,?8,?9,?10,?11,?12)";
        let ended = now_iso();
        app.insert(
            sql,
            params![r[0], r[1], r[2], r[3], r[4], ended, r[5], r[6], r[7], why, factory, role_hash],
        );
    }
    app.insert("DELETE FROM open_turns", &[]);
}
