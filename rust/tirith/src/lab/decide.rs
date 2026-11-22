//! Tirith decides an experiment alone, by the `[lab]` rule it started with, once it has `inputs` comparisons or is `max_days` old: adopted, its branch is merged into the bot's main (its own folder is that checkout, so the bot uses it from its next run); discarded, it goes. The worktree and the branch go in both cases, and the bot's chat gets one line.
use super::adopt::adopt;
use super::due::{due, minimum, on_way, Due};
use super::git::clear;
use super::numbers::{side, tally};
use super::seat::live;
use super::store::{all_running, running_one, tree, Experiment};
use crate::app::{now_iso, Shared};
use crate::server::s;
use rusqlite::params;
use serde_json::json;

/// The experiment's name for the line the bot's chat gets: its routine's title and its branch.
fn name_of(app: &Shared, e: &Experiment) -> String {
    let bot = crate::bots::bot(app, &e.bot).unwrap_or_default();
    let routines = bot["routines"].as_array().cloned().unwrap_or_default();
    let title = routines.iter().find(|r| r["id"] == e.routine.as_str());
    format!(
        "{} {}",
        title.map_or(e.routine.clone(), |r| s(r, "title")),
        e.branch
    )
}

/// Decides experiment `id` when it is due, no run of its bot is on its way (nor a shadow B beside one) and none waits for its judge, who may call it a failure.
pub fn decide(app: &Shared, id: i64) {
    let Some(e) = running_one(app, id).filter(|e| !on_way(app, e)) else {
        return;
    };
    let (lab, t) = (super::fixed::of(app, &e), tally(app, id));
    let name = name_of(app, &e);
    match due(app, &e, &t, &lab) {
        Due::No => {}
        Due::Expired(n) => {
            let line = format!(
                "Descarté {name}: sus días se acabaron con solo {n} comparaciones (mínimo {})",
                minimum(&lab)
            );
            close(app, &e, false, &line);
        }
        Due::Yes => {
            let (a, b) = (side(app, id, "A"), side(app, id, "B"));
            let (adopt, line) = super::rule::decide(&lab, &t, &a, &b, &name);
            close(app, &e, adopt, &line);
        }
    }
}

/// Experiment `e` ends, adopted (when its branch merges) or discarded, with `line` for the bot's chat: its worktree and its branch go, and its numbers are kept.
pub(super) fn close(app: &Shared, e: &Experiment, adopt_it: bool, line: &str) {
    let (a, b, t) = (side(app, e.id, "A"), side(app, e.id, "B"), tally(app, e.id));
    let (mut adopted, mut line) = (adopt_it, line.to_string());
    if adopted {
        crate::bots::versioned(app, &e.bot, "before the lab"); // what the bot's folder holds now is in its main first
        if !adopt(
            &live(app, &e.bot),
            &e.branch,
            &format!("lab: adopt experiment {}", e.id),
        ) {
            (adopted, line) = (
                false,
                format!(
                    "Descarté {}: no entra en su main (conflicto: cambió mientras se probaba)",
                    name_of(app, e)
                ),
            );
        }
    }
    let (own, its) = (live(app, &e.bot), tree(app, e));
    let _ = (e.mode == "live").then(|| super::workfiles::back(&own, &its)); // a restart mid-run left it there
    clear(&own, &its, &e.branch);
    let numbers =
        json!({"a": a, "b": b, "tally": t, "savings": super::saved::savings(&a, &b), "line": line});
    let status = if adopted { "adopted" } else { "discarded" };
    let sql = "UPDATE experiments SET status = ?1, ended_at = ?2, numbers = ?3 WHERE id = ?4";
    app.insert(sql, params![status, now_iso(), numbers.to_string(), e.id]);
    [
        format!("lab:last:{}", e.id),
        format!("lab:pairs:{}", e.id),
        super::replay::done_key(e.id),
    ]
    .iter()
    .for_each(|k| app.set_kv(k, None));
    let on = if adopted {
        "lab.adopted"
    } else {
        "lab.discarded"
    };
    crate::manager::emit_event(app, &e.bot, on, &line, None);
    app.emit("lab", json!({"id": e.id, "bot": e.bot, "status": status}));
}

/// Every running experiment that is due is decided (the scheduler's tick: `max_days` passes with no run too).
pub fn tick(app: &Shared) {
    all_running(app).iter().for_each(|e| decide(app, e.id));
}
