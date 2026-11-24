//! A good run that no judge will read any more (NeoBot stopped while it was judged, or only a share is judged) is no success by that: code's plain words decide, as when a judge gave no verdict.
use super::admitted::admitted;
use super::calendar::utc;
use super::store::OK;
use crate::app::Shared;
use chrono::{DateTime, Local, TimeDelta};

/// The longest a judge's turns take, both tries.
pub(super) const WAITS: TimeDelta = TimeDelta::minutes(25);
/// How far back one is still looked at.
const BACK: TimeDelta = TimeDelta::days(1);

pub(super) fn check(app: &Shared, now: DateTime<Local>) {
    let sql = "SELECT e.turn, COALESCE(t.answer, '') FROM expected_runs e JOIN turns t ON t.id = e.turn WHERE e.state = ?1 AND e.ended_at > ?2 AND e.ended_at <= ?3 AND t.status = 'ok' AND t.event IS NOT NULL AND NOT EXISTS (SELECT 1 FROM scores s WHERE s.event = t.event AND s.eval = ?4)";
    let (since, until) = (utc(now - BACK), utc(now - WAITS));
    let args = rusqlite::params![OK, since, until, crate::turns::GRADE];
    let unread: Vec<(i64, String)> = app.query(sql, args, |r| Ok((r.get(0)?, r.get(1)?)));
    for (turn, answer) in unread {
        admitted(app, turn, &answer, None);
    }
}

/// Whether a run of experiment `experiment` still waits for its judge, whose word can make it a failure: no longer than a judge takes.
pub fn awaited(app: &Shared, experiment: i64) -> bool {
    let sql = "SELECT 1 FROM expected_runs e JOIN turns t ON t.id = e.turn WHERE t.experiment = ?1 AND e.state = ?2 AND e.ended_at > ?3 AND t.status = 'ok' AND t.event IS NOT NULL AND NOT EXISTS (SELECT 1 FROM scores s WHERE s.event = t.event AND s.eval = ?4) LIMIT 1";
    let args = rusqlite::params![
        experiment,
        OK,
        utc(Local::now() - WAITS),
        crate::turns::GRADE
    ];
    !app.query(sql, args, |r| r.get::<_, i64>(0)).is_empty()
}
