//! A run that code found good and that its judge read as "I could not", in words no list holds: it ends as a failure, once. Where no judge could read it, plain words of trouble do the same, so it is never kept as a success in silence.
use super::admits::{admits, pass_admits, trouble};
use super::claims::{after, could_not, NOTHING};
use super::store::{close, select, FAILED, OK};
use super::{news, notice, words::short};
use crate::app::{redact_secrets, Shared};
use rusqlite::params;
use serde_json::json;

/// Turn `turn`, which answered `answer`, could not, for `cause` (none: no judge read it): its run, its row, and one line.
pub fn admitted(app: &Shared, turn: i64, answer: &str, cause: Option<&str>) {
    let quiet = after(answer, NOTHING).is_some();
    if (cause.is_none() && !suspects(answer, quiet)) || crate::turns::asked(app, turn) {
        return; // (waiting for the owner's yes is how such a run is done)
    }
    let sql = "UPDATE expected_runs SET state = ?1 WHERE turn = ?2 AND state = ?3 RETURNING id";
    let claimed: Vec<i64> = app.query(sql, params![FAILED, turn, OK], |r| r.get(0));
    for run in claimed.iter().flat_map(|id| select(app, "id = ?1", [*id])) {
        let why = match cause.map(|c| short(&redact_secrets(c), 160)) {
            Some(cause) if quiet => format!("dijo ∅, pero no pudo: {cause}"),
            Some(cause) => format!("no pudo: {cause}"),
            None => {
                let said = answer.rsplit("\n\n").next().unwrap_or(answer);
                format!(
                    "no pudo, lo dice su respuesta: «{}»",
                    short(&redact_secrets(said), 120)
                )
            }
        };
        news::drop_them(app, turn);
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

/// What code's loose words make of an answer that no judge read; a «∅» may speak of trouble that is over, but not say that it could not.
pub(super) fn suspects(answer: &str, quiet: bool) -> bool {
    admits(answer) || could_not(answer) || (!quiet && trouble(answer))
}

/// Turn `turn` is no judge's to read (a pass, or no bot's): a first-person "I could not" is its run's failure, and its news wait for nobody.
pub fn ungraded(app: &Shared, turn: i64) {
    let sql =
        "SELECT COALESCE(answer, ''), routine FROM turns WHERE id = ?1 AND status = 'ok' AND routine <> ''";
    let said: Vec<(String, String)> = app.query(sql, [turn], |r| Ok((r.get(0)?, r.get(1)?)));
    for (answer, routine) in &said {
        let could_not = if routine == super::PASS {
            pass_admits(answer)
        } else {
            admits(answer)
        };
        let _ = could_not.then(|| admitted(app, turn, answer, None));
    }
    news::settled(app, turn);
}
