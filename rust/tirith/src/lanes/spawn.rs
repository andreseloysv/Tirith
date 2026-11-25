//! Starting the agent's process for a lane, its output appended to the lane's log, and ending the lane when it exits.
use super::builds::shared_target;
use super::ended::ended;
use super::logscan::RUN_START;
use crate::app::{now_iso, now_s, Lane, Shared};
use crate::sandbox;
use std::fs::File;
use std::path::Path;
use std::process::Stdio;

/// The lane's log, opened for appending, with the launch line written.
fn open_log(app: &Shared, id: &str) -> Result<File, String> {
    let mut log = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(app.home.join(format!("logs/lanes/{id}.log")))
        .map_err(|e| e.to_string())?;
    let _ = std::io::Write::write_fmt(&mut log, format_args!("{RUN_START}{}\n", now_iso()));
    Ok(log)
}
/// The process's waiter: when it exits, the lane ends.
fn wait_for(app: &Shared, id: String, mut child: tokio::process::Child) {
    let app = app.clone();
    tokio::spawn(async move {
        let code = child.wait().await.ok().and_then(|s| s.code());
        app.children.lock().unwrap().remove(&id);
        ended(&app, &id, code).await;
    });
}
/// Runs the agent's command line in the lane's worktree, sandboxed, and marks the lane running.
pub(super) fn spawn_agent(
    app: &Shared,
    mut lane: Lane,
    ws: &Path,
    argv: Vec<String>,
) -> Result<(), String> {
    let own_tmp = super::temp::env(app, argv.first().map_or("", String::as_str), ws);
    let launch = sandbox::wrap(app, &lane.id, argv);
    let log = open_log(app, &lane.id)?;
    let mut command = launch.command(app);
    let child = command
        .current_dir(ws)
        .env("TIRITH_LANE", &lane.id)
        .env("TIRITH_PROJECT", &lane.project)
        .env("CARGO_TARGET_DIR", shared_target(app, &lane.project))
        .env("TIRITH_HOME", &app.home)
        .envs(own_tmp) // its temporary files beside the worktree, to go with it
        .stdin(Stdio::null())
        .stdout(log.try_clone().map_err(|e| e.to_string())?)
        .stderr(log)
        .spawn()
        .map_err(|e| format!("cannot start {}: {e}", launch.program))?;
    (
        lane.pid,
        lane.status,
        lane.started_at,
        lane.ended_at,
        lane.last_activity,
    ) = (child.id(), "running".into(), Some(now_iso()), None, now_s());
    app.save_lane(&lane);
    app.children.lock().unwrap().insert(lane.id.clone());
    wait_for(app, lane.id.clone(), child);
    Ok(())
}
