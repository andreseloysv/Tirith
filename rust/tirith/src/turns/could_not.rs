//! The judge's word that a routine's run could not do its task: whatever words the answer used, that run is a failure like one that code saw.
use super::quality::EVAL;
use crate::app::Shared;
use rusqlite::params;

/// What the judge's reason starts with (`factory/evals/turn-quality.md`).
const SAYS: &str = "could not";
/// And the most such a run scores.
const AT_MOST: f64 = 0.3;

/// The cause, when `score` and `why` say that the run could not.
fn cause(score: f64, why: &str) -> Option<&str> {
    let why = why.trim_start_matches(['`', '*', '"']);
    let head = why.get(..SAYS.len())?;
    let marked = head.eq_ignore_ascii_case(SAYS) && score <= AT_MOST;
    let rest = why[head.len()..].trim_start_matches(['`', '*']);
    marked.then(|| rest.strip_prefix(':'))?.map(str::trim)
}

/// The judge's last word on event `event`: a turn of a routine that it says could not ends as a failure.
pub(super) fn check(app: &Shared, event: i64) {
    let sql = "SELECT t.id, COALESCE(t.answer, ''), s.score, COALESCE(s.why, '') FROM turns t JOIN scores s ON s.event = t.event AND s.eval = ?2 WHERE t.event = ?1 AND t.status = 'ok' AND t.routine <> '' AND s.score IS NOT NULL ORDER BY s.id DESC LIMIT 1";
    let found = app.query(sql, params![event, EVAL], |r| {
        let (turn, answer): (i64, String) = (r.get(0)?, r.get(1)?);
        Ok((turn, answer, r.get::<_, f64>(2)?, r.get::<_, String>(3)?))
    });
    for (turn, answer, score, why) in found {
        if let Some(cause) = cause(score, &why) {
            crate::expected::admitted(app, turn, &answer, cause);
        }
    }
}
