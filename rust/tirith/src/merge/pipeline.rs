//! Pipelines: one row per train, main check or deploy, with its runs as stages.
use crate::app::{now_iso, Shared};
use rusqlite::params;
use serde_json::{json, Value};
pub(super) struct Pipe {
    pub(super) id: String,
    pub(super) project: String,
    pub(super) lanes: String,
    pub(super) sha: String,
}
pub(super) fn open_pipeline(
    app: &Shared,
    project: &str,
    kind: &str,
    lanes: &str,
    sha: &str,
) -> Pipe {
    let n = app
        .query(
            "SELECT COUNT(*) FROM pipelines WHERE project = ?1",
            [project],
            |r| r.get::<_, i64>(0),
        )
        .into_iter()
        .next()
        .unwrap_or(0);
    let id = format!("{project}-{}", n + 1);
    app.insert("INSERT INTO pipelines (id, project, kind, lanes, sha, status, started_at) VALUES (?1,?2,?3,?4,?5,'running',?6)", params![id, project, kind, lanes, sha, now_iso()]);
    let p = Pipe {
        id,
        project: project.into(),
        lanes: lanes.into(),
        sha: sha.into(),
    };
    app.emit("pipeline", pipeline(app, &p.id));
    p
}
pub(super) fn close_pipeline(app: &Shared, p: &Pipe, status: &str) {
    let _ = app.db.lock().unwrap().execute(
        "UPDATE pipelines SET status = ?1, ended_at = ?2 WHERE id = ?3",
        params![status, now_iso(), p.id],
    );
    app.emit("pipeline", pipeline(app, &p.id));
}
pub(super) fn pipeline(app: &Shared, id: &str) -> Value {
    let mut rows = app.objects("SELECT id, project, kind, lanes, sha, status, started_at, ended_at FROM pipelines WHERE id = ?1", [id], &["id", "project", "kind", "lanes", "sha", "status", "started_at", "ended_at"]);
    let Some(mut row) = rows.pop() else {
        return Value::Null;
    };
    row["stages"] = json!(app.objects(
        "SELECT id, stage, status, started_at, ended_at FROM runs WHERE pipeline = ?1 ORDER BY id",
        [id],
        &["id", "stage", "status", "started_at", "ended_at"]
    ));
    row
}
pub fn list(app: &Shared, project: &str, limit: u32) -> Vec<Value> {
    let sql = format!(
        "SELECT id FROM pipelines WHERE ?1 = '' OR project = ?1 ORDER BY rowid DESC LIMIT {limit}"
    );
    app.query(&sql, [project], |r| r.get::<_, String>(0))
        .iter()
        .map(|id| pipeline(app, id))
        .collect()
}
