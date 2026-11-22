//! A B run against the A run before it, judged blind by the `ab-compare` eval: the two answers in an order drawn at random (kept as the letter B was shown under), the judge's a, b or tie mapped back to B winning, losing or tying.
use super::store::Experiment;
use crate::app::Shared;
use crate::server::s;
use rusqlite::params;

/// The eval that judges a comparison (factory.toml `[[evals]]` on `lab.compare`).
const EVAL: &str = "ab-compare";

/// What the judge reads: the task, then the two answers as «Answer a» and «Answer b», B's first when `b_first`. With the letter B was shown under.
pub fn blind(task: &str, a: &str, b: &str, b_first: bool) -> (String, &'static str) {
    let (first, second, b_is) = if b_first { (b, a, "a") } else { (a, b, "b") };
    let cut = |t: &str| t.chars().take(1500).collect::<String>();
    let text = format!(
        "Two answers to one task of a routine, in no particular order.\nThe task: {}\n<<<Answer a\n{}\nAnswer a>>>\n<<<Answer b\n{}\nAnswer b>>>",
        cut(task),
        cut(first),
        cut(second)
    );
    (text, b_is)
}

/// The judge's `label` (a, b or tie) as B's verdict, B having been shown as `b_is`: win, lose or tie; anything else is no verdict.
pub fn verdict(label: &str, b_is: &str) -> Option<&'static str> {
    match label {
        "tie" => Some("tie"),
        "a" | "b" if label == b_is => Some("win"),
        "a" | "b" => Some("lose"),
        _ => None,
    }
}

/// The task of the experiment's routine, as its bot holds it.
fn task(app: &Shared, e: &Experiment) -> String {
    let bot = crate::bots::bot(app, &e.bot).unwrap_or_default();
    let routines = bot["routines"].as_array().cloned().unwrap_or_default();
    let r = routines.iter().find(|r| r["id"] == e.routine.as_str());
    r.map(|r| s(r, "prompt")).unwrap_or_default()
}

/// B run `b` ended: it is compared with the A run of its experiment before it (none yet: nothing to compare).
pub(super) fn compare(app: &Shared, e: &Experiment, b: i64) {
    let sql = "SELECT id FROM turns WHERE experiment = ?1 AND variant = 'A' AND id <= ?2 ORDER BY id DESC LIMIT 1";
    let a = app.query(sql, params![e.id, b], |r| r.get::<_, i64>(0));
    if let Some(a) = a.into_iter().next() {
        compare_pair(app, e, a, b);
    }
}

/// A run `a` against B run `b`, the judge's input kept as event `lab.compare` in the bot's conversation.
pub(super) fn compare_pair(app: &Shared, e: &Experiment, a: i64, b: i64) {
    let sql =
        "SELECT COALESCE(answer, ''), COALESCE(status, '') = 'failed' FROM turns WHERE id = ?1";
    let ran = |id: i64| {
        app.query(sql, [id], |r| Ok((r.get::<_, String>(0)?, r.get(1)?)))
            .pop()
    };
    let (Some((a_text, a_failed)), Some((b_text, b_failed))) = (ran(a), ran(b)) else {
        return;
    };
    if let Some(why) = super::void::pair(app, e, (a, b), a_failed && b_failed) {
        return super::void::void(app, e, (a, b), &why);
    }
    let (text, b_is) = blind(&task(app, e), &a_text, &b_text, rand::random());
    let row = app.insert(
        "INSERT INTO lab_comparisons (experiment, a_turn, b_turn, b_is) VALUES (?1, ?2, ?3, ?4)",
        params![e.id, a, b, b_is],
    );
    let event = crate::manager::emitted(app, &e.bot, "lab.compare", &text, None);
    app.insert(
        "UPDATE lab_comparisons SET event = ?1 WHERE id = ?2",
        params![event, row],
    );
}

/// A judge answered eval `eval` on event `event`: an `ab-compare` one is B's verdict on its comparison, and its experiment may be decided now.
pub fn judged(app: &Shared, event: i64, eval: &str, label: &str, why: &str) {
    let Some(()) = (eval == EVAL).then_some(()) else {
        return;
    };
    let sql =
        "SELECT id, experiment, b_is FROM lab_comparisons WHERE event = ?1 AND verdict IS NULL";
    let rows = app.query(sql, [event], |r| {
        Ok((
            r.get::<_, i64>(0)?,
            r.get::<_, i64>(1)?,
            r.get::<_, String>(2)?,
        ))
    });
    for (id, experiment, b_is) in rows {
        let Some(v) = verdict(label, &b_is) else {
            continue;
        };
        let sql = "UPDATE lab_comparisons SET verdict = ?1, why = ?2 WHERE id = ?3";
        app.insert(sql, params![v, why, id]);
        super::decide::decide(app, experiment);
    }
}
