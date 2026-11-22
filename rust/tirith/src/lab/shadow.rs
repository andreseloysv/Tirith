//! Shadow mode: each real run of the routine is A, as always (the bot's own folder, its answer to the owner), and its B starts beside it at the same time: a loose turn of the bot's role in the branch's worktree, handed the same task, its reads live and every outward action refused by the hook (`shadow_open.rs`). B's turn runs in a conversation of its own that no chat, phone, room or card knows (`shadow-<experiment>-<pair>`): its answer only goes to the experiment.
use super::seat::{sit, Seat};
use super::shadow_open::{close, open, SHADOW};
use super::store::{tree, Experiment};
use crate::app::Shared;
use rusqlite::params;

/// The shadow experiment `e` on the routine that fired with `task`: this run is A of the next pair, and its B starts beside it. While a B of `e` is still on its way, the run is no pair.
pub(super) fn fired(app: &Shared, e: Experiment, task: &str) {
    let dir = tree(app, &e);
    let Some(()) = open(&dir).then_some(()) else {
        return;
    };
    let key = format!("lab:pairs:{}", e.id);
    let pair = app
        .kv(&key)
        .and_then(|n| n.parse::<i64>().ok())
        .unwrap_or(0)
        + 1;
    app.set_kv(&key, Some(&pair.to_string()));
    let s = Seat {
        experiment: e.id,
        routine: e.routine.clone(),
        variant: "A",
        tree: dir.clone(),
        pair,
    };
    sit(app, &e.bot, s);
    super::fresh::session(app, &e.bot); // A starts as bare as its B
    let recent = crate::manager::recent(app, &e.bot);
    let (app, task) = (app.clone(), task.to_string());
    tokio::spawn(async move {
        let b = beside(&app, &e, pair, (&recent, &task)).await;
        close(&dir);
        if let Some(b) = b {
            super::shadow_pair::done(&app, &e, (&dir, pair), (None, Some(b)));
        }
        super::decide::decide(&app, e.id);
    });
}

/// Runs B of pair `pair`: its turn's id, none when it could not run.
async fn beside(
    app: &Shared,
    e: &Experiment,
    pair: i64,
    (recent, task): (&str, &str),
) -> Option<i64> {
    let conv = format!("shadow-{}-{pair}", e.id);
    let run = async {
        let memory = crate::bots::context(app, &e.bot); // the worktree's memory and skills: it is the bot's folder now
        let memory = memory.map_or(String::new(), |m| format!("{m}\n\n"));
        let input = format!("{recent}{memory}{task}"); // as A's fresh session is handed it
        crate::manager::run_turn(app, &e.bot, &conv, &input).await
    };
    let _ = SHADOW.scope((tree(app, e), e.routine.clone()), run).await;
    let sql = "UPDATE turns SET experiment = ?1, variant = 'B', routine = ?2, \"trigger\" = 'shadow' WHERE conv = ?3";
    app.insert(sql, params![e.id, e.routine, conv]);
    let sql = "SELECT MAX(id) FROM turns WHERE conv = ?1";
    let id = app.query(sql, [&conv], |r| r.get::<_, Option<i64>>(0));
    id.into_iter().next().flatten()
}
