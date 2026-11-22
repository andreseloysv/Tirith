//! The `[lab]` rule an experiment started with: fixed before the test, so a change of the factory (or of the Coach) while it runs never moves the goal.
use super::store::Experiment;
use crate::app::App;
use crate::factory::Lab;
use rusqlite::params;
use serde_json::json;

/// Keeps the factory's current rule as the rule of experiment `id`.
pub(super) fn keep(app: &App, id: i64) {
    let lab = &app.cfg().factory.lab;
    let rule = json!({"inputs": lab.inputs, "not_worse_share": lab.not_worse_share, "min_saving": lab.min_saving, "max_days": lab.max_days});
    app.insert(
        "UPDATE experiments SET rule = ?1 WHERE id = ?2",
        params![rule.to_string(), id],
    );
}

/// The rule experiment `e` started with (the factory's current one for an experiment older than this column).
pub(super) fn of(app: &App, e: &Experiment) -> Lab {
    let kept = app.query("SELECT rule FROM experiments WHERE id = ?1", [e.id], |r| {
        r.get::<_, Option<String>>(0)
    });
    let rule = kept.into_iter().next().flatten();
    let parsed = rule.and_then(|r| serde_json::from_str::<Lab>(&r).ok());
    parsed.unwrap_or_else(|| app.cfg().factory.lab.clone())
}
