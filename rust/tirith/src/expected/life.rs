//! An expected run's life: it starts, and its turn ends it; or it never becomes a turn and ends without one (`lost.rs`).
use super::found::oldest;
use super::store::{close, select, Expected, FAILED, OK, RUNNING, WAITING};
use super::{lost, news, notice, PASS};
use crate::app::{now_iso, Shared};
use rusqlite::params;

/// The oldest waiting run of `routine` of `conv` starts now.
pub fn started(app: &Shared, conv: &str, routine: &str) {
    let Some(run) = oldest(app, conv, routine, WAITING) else {
        return;
    };
    let sql = "UPDATE expected_runs SET state = ?1, started_at = ?2, why = '' WHERE id = ?3";
    app.insert(sql, params![RUNNING, now_iso(), run.id]); // why it waited is over
}

/// Turn `turn` of `conv`, which ran `routine`, was kept with `status`: the run it was ends with it, and a failure is said. What the run held back from the phones goes out only when it is good, and once it was judged.
pub fn ended(
    app: &Shared,
    (conv, role, routine): (&str, &str, &str),
    turn: i64,
    status: &str,
    why: Option<&str>,
) {
    if routine.is_empty() || conv != role {
        return;
    }
    let good = status != "failed";
    let held = (routine != PASS) // a pass has no conversation's turn to hold
        .then(|| crate::manager::run_over(conv, good))
        .flatten();
    let run = oldest(app, conv, routine, RUNNING);
    if let Some(text) = held {
        match run.is_some() && status == "ok" {
            true => news::keep(app, conv, turn, &text), // before it closes: a failure may claim it from then on
            false => crate::rooms::to_phones(app, conv, &text),
        }
    }
    let Some(run) = run else {
        return;
    };
    if good {
        return close(app, run.id, OK, turn, "");
    }
    let why = why.unwrap_or("su agente falló");
    close(app, run.id, FAILED, turn, why);
    app.insert(
        "UPDATE turns SET why = ?1 WHERE id = ?2",
        params![why, turn],
    );
    notice::failed(app, &run, why, false);
}

/// An occurrence ends with no turn: its row says why, and so does one line.
pub(super) fn never(app: &Shared, run: &Expected, why: &str) {
    let turn = lost::row(app, run, why);
    close(app, run.id, FAILED, turn, why);
    if run.state == RUNNING && run.routine != PASS {
        crate::manager::release_run(app, &run.conv); // what it held: a seat in an experiment, its tuning
    }
    notice::lost(app, std::slice::from_ref(run), why);
}

/// A run of `conv` whose agent could not start, after its tries.
pub fn unstarted(app: &Shared, conv: &str, routine: &str, error: &str) {
    if let Some(run) = oldest(app, conv, routine, RUNNING) {
        never(
            app,
            &run,
            &format!("no arrancó: {}", super::reasons::failure(error)),
        );
    }
}

/// The session of `conv` ended, and runs it was handed left no turn.
pub fn orphaned(app: &Shared, conv: &str) {
    let rest = "conv = ?1 AND state = ?2 AND routine <> ?3 ORDER BY id";
    for run in select(app, rest, params![conv, RUNNING, PASS]) {
        let why = Some(run.why.as_str()).filter(|w| !w.is_empty());
        never(app, &run, why.unwrap_or("su agente terminó sin responder"));
    }
}

/// Nobody could be handed the run just written down.
pub fn unhanded(app: &Shared, conv: &str, routine: &str) {
    let rest = "conv = ?1 AND routine = ?2 AND state = ?3 ORDER BY id DESC LIMIT 1";
    for run in select(app, rest, params![conv, routine, WAITING]) {
        never(app, &run, "nadie puede hacerla: su rol no recibe tareas");
    }
}
