//! Where one occurrence stands, for whoever reports it (`/status`): the register's word, and its turn's where it left one, so it is the truth the numbers and the notice have.
use super::found::occurrence;
use super::store::{FAILED, OK};
use crate::app::App;

/// `done`, `failed`, `waiting for you` or `started`; none when that occurrence was never written down.
pub fn stands(app: &App, (conv, routine, due): (&str, &str, &str)) -> Option<&'static str> {
    let run = occurrence(app, (conv, routine, due))?;
    let sql = "SELECT COALESCE(t.status, '') FROM expected_runs e JOIN turns t ON t.id = e.turn WHERE e.id = ?1";
    let turn: Vec<String> = app.query(sql, [run.id], |r| r.get(0));
    let turn = turn.first().map_or("", String::as_str);
    Some(match (turn, run.state.as_str()) {
        ("waiting", _) => "waiting for you",
        ("failed", _) | ("", FAILED) => "failed",
        ("ok", _) | ("", OK) => "done",
        _ => "started",
    })
}
