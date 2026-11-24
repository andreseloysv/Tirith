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
    let said = marked.then(|| rest.strip_prefix(':'))??.trim();
    Some(
        Some(said)
            .filter(|s| !s.is_empty())
            .unwrap_or("its judge gave no reason"),
    )
}

/// Event `event`'s run as its judge's last word leaves it. With no score in it, or with no such eval, code's loose words decide.
pub(super) fn check(app: &Shared, event: i64) {
    let sql = "SELECT id, COALESCE(answer, '') FROM turns WHERE event = ?1 AND status = 'ok' AND routine <> ''";
    let found = app.query(sql, [event], |r| Ok((r.get(0)?, r.get(1)?)));
    let Some((turn, answer)): Option<(i64, String)> = found.into_iter().next() else {
        return;
    };
    let sql = "SELECT score, COALESCE(why, '') FROM scores WHERE event = ?1 AND eval = ?2 ORDER BY id DESC LIMIT 1";
    let words = app.query(sql, params![event, EVAL], |r| Ok((r.get(0)?, r.get(1)?)));
    let word: Option<(Option<f64>, String)> = words.into_iter().next();
    let (judge, why, partial) = match &word {
        Some((Some(score), why)) => (true, cause(*score, why), *score > 0.0),
        Some((None, _)) => (false, None, false),
        None if graded(app, event) => return, // it is still reading
        None => (false, None, false),
    };
    if why.is_some() || !judge {
        crate::expected::admitted_by(app, turn, &answer, (why, partial));
    }
    crate::expected::settled(app, turn);
}

/// True when a judge grades the run of event `event`.
fn graded(app: &Shared, event: i64) -> bool {
    let evals = app.cfg().factory.evals_for("routine.done", event);
    evals.iter().any(|e| e.0 == EVAL)
}
