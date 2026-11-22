//! The evals: an event judged in the background by its judge role, kept as a score.
use super::judged::{card_id_of, judged};
use super::prompt::history;
use super::turn::run_turn;
use crate::app::{now_iso, Shared};
/// Every `[[evals]]` on this event, judged in the background by its judge role (a cheap turn role whose instructions are the rubric): the event with the conversation before it (and, for a card, the card itself) goes in, and a `scores` row — the label or score, one line why, the event, the eval and the judge's factory stamp — is kept. Nothing reaches the owner: the scores only feed the measures. A share of the events is judged (`sample`, by event id, so a restart judges no event twice for that share).
pub(super) fn judge_evals(app: &Shared, project: &str, on: &str, shown: &str, event: i64) {
    let evals: Vec<_> = app
        .cfg()
        .factory
        .evals_on(on)
        .into_iter()
        .filter(|e| event.rem_euclid(100) < (e.3 * 100.0) as i64)
        .collect();
    let Some(()) =
        (!evals.is_empty() && tokio::runtime::Handle::try_current().is_ok()).then_some(())
    else {
        return;
    };
    let (app, project, shown) = (
        app.clone(),
        app.conv(project).to_string(),
        shown.to_string(),
    );
    let on = on.to_string(); // the judge's turn says the event it judges
    let before = crate::turns::last(&app, &project); // the turn a correction is about
    let recent = history(&app, &project); // now: what came before this event
    tokio::spawn(async move {
        let card = card_context(&app, &shown);
        let input = format!(
            "{}\nThe event to judge: {shown}\n{}",
            recent,
            card.map(|c| format!("\n{c}\n")).unwrap_or_default()
        );
        for (id, judge, rubric, _) in evals {
            let rules = app.cfg().factory.prompt(&rubric);
            let mut asked = format!("{rules}\n\n{input}");
            let mut said = None;
            for _ in 0..2 {
                crate::turns::trigger(&format!("turn:{judge}"), &format!("event:{on}"), "");
                let Ok(text) = run_turn(&app, &judge, &project, &asked).await else {
                    break;
                };
                let decided = super::verdict::given(&text, &rules);
                said = Some(text);
                if decided {
                    break;
                }
                asked += super::verdict::AGAIN; // a small model may forget the shape: once more
            }
            let Some(text) = said else {
                continue;
            };
            let (label, score, why) = judged(super::verdict::last(&text, &rules));
            crate::turns::judged(&app, before, &id, &label);
            let (factory, role_hash) = app.stamp_for(&judge);
            app.insert("INSERT INTO scores (at, event, eval, label, score, why, factory, role_hash) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)", rusqlite::params![now_iso(), event, id, label, score, why, factory, role_hash]);
            crate::turns::scored(&app, event, &id, score);
            crate::lab::judged(&app, event, &id, &label, &why); // an ab-compare: B's verdict
        }
    });
}
/// The card an event's line names, with its question and its answer, for the judge.
fn card_context(app: &Shared, shown: &str) -> Option<String> {
    card_id_of(shown).and_then(|id| {
        app.query(
            "SELECT kind, question, status, answer FROM owner_items WHERE id = ?1",
            [&id],
            |r| {
                Ok(format!(
                    "Card {id} ({}, {}): {}\nIts answer or closing note: {}",
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(3)?
                ))
            },
        )
        .into_iter()
        .next()
    })
}
