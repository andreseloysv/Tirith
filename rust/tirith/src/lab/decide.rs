//! Tirith decides an experiment alone, by the factory's `[lab]` rule, once it has `inputs` comparisons or is `max_days` old: adopted, its branch is merged into the bot's main (its own folder is that checkout, so the bot uses it from its next run); discarded, it goes. The worktree and the branch go in both cases, and the bot's chat gets one line.
use super::git::{clear, merge};
use super::numbers::{side, tally};
use super::rule::{savings, Tally};
use super::seat::{live, seated};
use super::store::{all_running, running_one, tree, Experiment};
use crate::app::{now_iso, Shared};
use crate::server::s;
use rusqlite::params;
use serde_json::json;

/// Whether experiment `e` is due: `inputs` comparisons judged, a replay's pairs all run and judged, or `max_days` since it started.
fn due(app: &Shared, e: &Experiment, t: &Tally) -> bool {
    let lab = app.cfg().factory.lab.clone();
    let started = chrono::DateTime::parse_from_rfc3339(&e.created_at).ok();
    let days = started.map_or(0, |s| (chrono::Utc::now() - s.to_utc()).num_days());
    let replayed = e.mode == "replay"
        && app.kv(&super::replay::done_key(e.id)).is_some()
        && super::numbers::unjudged(app, e.id) == 0;
    t.wins + t.ties + t.losses >= lab.inputs as i64 || replayed || days >= lab.max_days as i64
}

/// Decides experiment `id` when it is due and no run of its bot is on its way.
pub fn decide(app: &Shared, id: i64) {
    let Some(e) = running_one(app, id).filter(|e| !seated(&live(app, &e.bot))) else {
        return;
    };
    let t = tally(app, id);
    let Some(()) = due(app, &e, &t).then_some(()) else {
        return;
    };
    let (a, b) = (side(app, id, "A"), side(app, id, "B"));
    let bot = crate::bots::bot(app, &e.bot).unwrap_or_default();
    let routines = bot["routines"].as_array().cloned().unwrap_or_default();
    let title = routines
        .iter()
        .find(|r| r["id"] == e.routine.as_str())
        .map(|r| s(r, "title"));
    let name = format!("{} {}", title.unwrap_or(e.routine.clone()), e.branch);
    let (mut adopt, mut line) = super::rule::decide(&app.cfg().factory.lab, &t, &a, &b, &name);
    if adopt {
        let dir = live(app, &e.bot);
        crate::bots::versioned(app, &e.bot, "before the lab"); // what the bot's folder holds now is in its main first
        if !merge(&dir, &e.branch, &format!("lab: adopt experiment {id}")) {
            (adopt, line) = (
                false,
                format!("Descarté {name}: no entra en su main (conflicto)"),
            );
        }
    }
    clear(&live(app, &e.bot), &tree(app, &e), &e.branch);
    let numbers = json!({"a": a, "b": b, "tally": t, "savings": savings(&a, &b), "line": line});
    let status = if adopt { "adopted" } else { "discarded" };
    let sql = "UPDATE experiments SET status = ?1, ended_at = ?2, numbers = ?3 WHERE id = ?4";
    app.insert(sql, params![status, now_iso(), numbers.to_string(), id]);
    app.set_kv(&format!("lab:last:{id}"), None);
    app.set_kv(&super::replay::done_key(id), None);
    let on = if adopt {
        "lab.adopted"
    } else {
        "lab.discarded"
    };
    crate::manager::emit_event(app, &e.bot, on, &line, None);
    app.emit("lab", json!({"id": id, "bot": e.bot, "status": status}));
}

/// Every running experiment that is due is decided (the scheduler's tick: `max_days` passes with no run too).
pub fn tick(app: &Shared) {
    all_running(app).iter().for_each(|e| decide(app, e.id));
}
