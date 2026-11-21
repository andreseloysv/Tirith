//! Tirith decides an experiment alone, by the factory's `[lab]` rule, once it has `inputs` comparisons or is `max_days` old: adopted, its branch is merged into the bot's main (its own folder is that checkout, so the bot uses it from its next run); discarded, it goes. The worktree and the branch go in both cases, and the bot's chat gets one line.
use super::git::{clear, git};
use super::rule::{savings, Side, Tally};
use super::seat::{live, seated};
use super::store::{all_running, running_one, tree, Experiment};
use crate::app::{now_iso, Shared};
use crate::server::s;
use rusqlite::params;
use serde_json::json;

const SIDE: &str = "SELECT COUNT(*), AVG(COALESCE(tokens_in, 0) + COALESCE(tokens_out, 0)), CASE WHEN COUNT(cost_usd) = COUNT(*) THEN AVG(cost_usd) END, AVG(COALESCE(ms, 0)) FROM turns WHERE experiment = ?1 AND variant = ?2";
const TALLY: &str = "SELECT COALESCE(SUM(verdict = 'win'), 0), COALESCE(SUM(verdict = 'tie'), 0), COALESCE(SUM(verdict = 'lose'), 0), COALESCE(SUM(b.status = 'failed' AND a.status <> 'failed'), 0), (SELECT COUNT(*) FROM turns WHERE experiment = ?1 AND variant = 'B' AND reaction = -1) FROM lab_comparisons c JOIN turns a ON a.id = c.a_turn JOIN turns b ON b.id = c.b_turn WHERE c.experiment = ?1";

/// One side's runs in experiment `id`.
pub fn side(app: &Shared, id: i64, variant: &str) -> Side {
    let rows = app.query(SIDE, params![id, variant], |r| {
        Ok(Side {
            runs: r.get(0)?,
            tokens: r.get::<_, Option<f64>>(1)?.unwrap_or(0.0),
            cost: r.get(2)?,
            ms: r.get::<_, Option<f64>>(3)?.unwrap_or(0.0),
        })
    });
    rows.into_iter().next().unwrap_or_default()
}

/// B's comparisons in experiment `id`, its new failures and its 👎.
pub fn tally(app: &Shared, id: i64) -> Tally {
    let rows = app.query(TALLY, [id], |r| {
        Ok(Tally {
            wins: r.get(0)?,
            ties: r.get(1)?,
            losses: r.get(2)?,
            new_failures: r.get(3)?,
            down: r.get(4)?,
        })
    });
    rows.into_iter().next().unwrap_or_default()
}

/// Whether experiment `e` is due: `inputs` comparisons judged, or `max_days` since it started.
fn due(app: &Shared, e: &Experiment, t: &Tally) -> bool {
    let lab = app.cfg().factory.lab.clone();
    let started = chrono::DateTime::parse_from_rfc3339(&e.created_at).ok();
    let days = started.map_or(0, |s| (chrono::Utc::now() - s.to_utc()).num_days());
    t.wins + t.ties + t.losses >= lab.inputs as i64 || days >= lab.max_days as i64
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
        let msg = format!("lab: adopt experiment {} ({})", e.id, e.branch);
        if git(&dir, &["merge", "-q", "--no-edit", "-m", &msg, &e.branch]).is_err() {
            let _ = git(&dir, &["merge", "--abort"]);
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
