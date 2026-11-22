//! The expected runs as kept (`expected_runs`): one row for each occurrence, waiting, running, then ok or failed.
use crate::app::{now_iso, App};
use rusqlite::{params, Params};

pub(super) const WAITING: &str = "waiting";
pub(super) const RUNNING: &str = "running";
pub(super) const OK: &str = "ok";
pub(super) const FAILED: &str = "failed";

/// One occurrence: whose routine it is, what started it, the event it is handed as, when it was due (UTC), its task and where it is.
#[derive(Clone, Debug)]
pub struct Expected {
    pub id: i64,
    pub conv: String,
    pub routine: String,
    pub title: String,
    pub trigger: String,
    pub kind: String,
    pub due: String,
    pub task: String,
    pub state: String,
    pub why: String,
    pub started_at: String,
}

/// What is written down when an occurrence is due.
pub struct New<'a> {
    pub conv: &'a str,
    pub routine: &'a str,
    pub title: &'a str,
    pub trigger: &'a str,
    /// The event it is handed as: `schedule` or `webhook`.
    pub kind: &'a str,
    pub due: &'a str,
    pub task: &'a str,
}

/// An occurrence is expected from now on. Its id.
pub fn expect(app: &App, n: &New) -> i64 {
    let sql = "INSERT INTO expected_runs (conv, routine, title, \"trigger\", kind, due, task, state, created_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)";
    app.insert(
        sql,
        params![
            n.conv,
            n.routine,
            n.title,
            n.trigger,
            n.kind,
            n.due,
            n.task,
            WAITING,
            now_iso()
        ],
    )
}

/// The occurrences `rest` (an SQL WHERE and ORDER) selects.
pub(super) fn select<P: Params>(app: &App, rest: &str, with: P) -> Vec<Expected> {
    let sql = format!("SELECT id, conv, routine, title, \"trigger\", kind, due, COALESCE(task, ''), state, COALESCE(why, ''), COALESCE(started_at, '') FROM expected_runs WHERE {rest}");
    app.query(&sql, with, |r| {
        Ok(Expected {
            id: r.get(0)?,
            conv: r.get(1)?,
            routine: r.get(2)?,
            title: r.get(3)?,
            trigger: r.get(4)?,
            kind: r.get(5)?,
            due: r.get(6)?,
            task: r.get(7)?,
            state: r.get(8)?,
            why: r.get(9)?,
            started_at: r.get(10)?,
        })
    })
}

/// Occurrence `id` ended as `state`, in turn `turn`, for `why`.
pub(super) fn close(app: &App, id: i64, state: &str, turn: i64, why: &str) {
    let sql =
        "UPDATE expected_runs SET state = ?1, turn = ?2, why = ?3, ended_at = ?4 WHERE id = ?5";
    app.insert(sql, params![state, turn, why, now_iso(), id]);
}

/// Why occurrence `id` waits, or is being stopped.
pub(super) fn because(app: &App, id: i64, why: &str) {
    let sql = "UPDATE expected_runs SET why = ?1 WHERE id = ?2";
    app.insert(sql, params![why, id]);
}
