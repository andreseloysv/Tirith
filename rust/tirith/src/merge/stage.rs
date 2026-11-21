//! One stage of a pipeline, run in a folder under the sandbox, one at a time per project, within its time limit.
use super::limits::{learn_timeout, stage_minutes};
use super::{env::stage_env, locks::stage_lock, pipeline::pipeline, pipeline::Pipe};
use crate::app::{kill_group, now_iso, tail, Shared};
use crate::{config::Manifest, sandbox};
use rusqlite::params;
use serde_json::json;
use std::{path::Path, process::Stdio, time::Duration};
pub(super) async fn run_stage(
    app: &Shared,
    pipe: &Pipe,
    dir: &Path,
    name: &str,
    run: &str,
) -> Result<(), String> {
    let lock = stage_lock(&pipe.project);
    let _one_at_a_time = lock.lock().await;
    let id = app.insert("INSERT INTO runs (lane, project, stage, status, started_at, pipeline, sha) VALUES (?1,?2,?3,'running',?4,?5,?6)", params![pipe.lanes, pipe.project, name, now_iso(), pipe.id, pipe.sha]);
    let log = app.home.join(format!("logs/runs/{id}.log"));
    let mut row = json!({"id": id, "lane": pipe.lanes, "project": pipe.project, "stage": name, "status": "running", "pipeline": pipe.id, "log": log});
    app.emit("run", row.clone());
    app.emit("pipeline", pipeline(app, &pipe.id));
    let file = std::fs::File::create(&log).map_err(|e| e.to_string())?;
    let mut command = stage_command(app, pipe, dir, name, run);
    let child = command
        .current_dir(dir)
        .stdin(Stdio::null())
        .stdout(file.try_clone().map_err(|e| e.to_string())?)
        .stderr(file)
        .spawn()
        .map_err(|e| e.to_string())?;
    let status = wait_within_limit(app, pipe, name, child).await;
    let _ = app.db.lock().unwrap().execute(
        "UPDATE runs SET status = ?1, ended_at = ?2, log = ?3 WHERE id = ?4",
        params![status, now_iso(), log.display().to_string(), id],
    );
    row["status"] = json!(status);
    app.emit("run", row);
    app.emit("pipeline", pipeline(app, &pipe.id));
    (status == "passed").then_some(()).ok_or_else(|| {
        format!(
            "check '{name}' {status}. Last lines of its log:\n{}",
            tail(&log, 60)
        )
    })
}
/// The sandboxed `sh -c <run>` of a stage, as the pipeline's first lane (or main), with the project's shared target dir; an item's check also gets its stage's environment.
fn stage_command(
    app: &Shared,
    pipe: &Pipe,
    dir: &Path,
    name: &str,
    run: &str,
) -> tokio::process::Command {
    let who = format!(
        "merge:{}",
        pipe.lanes
            .split(',')
            .next()
            .filter(|l| !l.is_empty())
            .unwrap_or("main")
    );
    let launch = sandbox::wrap(app, &who, vec!["sh".into(), "-c".into(), run.into()]);
    let mut command = launch.command(app);
    command.env(
        "CARGO_TARGET_DIR",
        crate::lanes::shared_target(app, &pipe.project),
    );
    let _ = name
        .starts_with("check ")
        .then(|| command.envs(stage_env(&Manifest::load(dir).stages, run)));
    command
}
/// "passed", "failed" or "timeout": a stage past its limit is killed with its group, and learns a longer limit.
async fn wait_within_limit(
    app: &Shared,
    pipe: &Pipe,
    name: &str,
    mut child: tokio::process::Child,
) -> &'static str {
    let limit = Duration::from_secs(stage_minutes(app, &pipe.project, name) * 60);
    match tokio::time::timeout(limit, child.wait()).await {
        Ok(Ok(s)) if s.success() => "passed",
        Ok(_) => "failed",
        Err(_) => {
            child
                .id()
                .into_iter()
                .for_each(|pid| kill_group(pid, libc::SIGKILL));
            let _ = child.wait().await;
            learn_timeout(app, &pipe.project, name);
            "timeout"
        }
    }
}
