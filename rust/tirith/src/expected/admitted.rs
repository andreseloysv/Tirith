//! A run that code found good and that its judge read as "I could not", in words no list holds: it ends as a failure, once. Where no judge could read it, plain words of trouble do the same, so it is never kept as a success in silence.
use super::admits::trouble;
use super::claims::{after, could_not, NOTHING};
use super::store::{close, select, FAILED, OK};
use super::{notice, words::short};
use crate::app::{redact_secrets, Shared};
use rusqlite::params;
use serde_json::json;

/// Turn `turn`, which answered `answer`, could not, for `cause` (none: no judge read it): its run, its row, and one line.
pub fn admitted(app: &Shared, turn: i64, answer: &str, cause: Option<&str>) {
    let quiet = after(answer, NOTHING).is_some();
    if cause.is_none() && (quiet || !(trouble(answer) || could_not(answer))) {
        return; // of a «∅», code has seen all it can
    }
    let sql = "UPDATE expected_runs SET state = ?1 WHERE turn = ?2 AND state = ?3 RETURNING id";
    let claimed: Vec<i64> = app.query(sql, params![FAILED, turn, OK], |r| r.get(0));
    for run in claimed.iter().flat_map(|id| select(app, "id = ?1", [*id])) {
        let cause = cause.unwrap_or("lo dice su respuesta, y su juez no contestó");
        let cause = short(&redact_secrets(cause), 120);
        let why = if quiet {
            format!("dijo ∅, pero no pudo: {cause}")
        } else {
            let said = answer.rsplit("\n\n").next().unwrap_or(answer);
            format!("no pudo: {cause} («{}»)", short(&redact_secrets(said), 90))
        };
        close(app, run.id, FAILED, turn, &why);
        let sql = "UPDATE turns SET status = ?1, why = ?2 WHERE id = ?3";
        app.insert(sql, params![FAILED, why, turn]);
        app.emit(
            "turn",
            json!({"id": turn, "conv": run.conv, "status": FAILED}),
        );
        notice::failed(app, &run, &why);
    }
}
