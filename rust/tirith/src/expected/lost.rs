//! The row in `turns` of a run that never was a turn: failed, with no agent, no time and no tokens (the numbers count the failure and nothing else), and why.
use super::store::Expected;
use crate::app::{now_iso, Shared};
use rusqlite::params;
use serde_json::json;

/// The row's id.
pub(super) fn row(app: &Shared, run: &Expected, why: &str) -> i64 {
    let (factory, role_hash) = app.stamp_for(&run.conv);
    let answer = format!("(no se hizo: {why})");
    let sql = "INSERT INTO turns (conv, role, \"trigger\", routine, started_at, ended_at, status, agent, answer, why, factory, role_hash) VALUES (?1,?1,?2,?3,?4,?5,'failed','',?6,?7,?8,?9)";
    let id = app.insert(
        sql,
        params![
            run.conv,
            run.trigger,
            run.routine,
            run.due,
            now_iso(),
            answer,
            why,
            factory,
            role_hash
        ],
    );
    app.emit(
        "turn",
        json!({"id": id, "conv": run.conv, "status": "failed"}),
    );
    id
}
