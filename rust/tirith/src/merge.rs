//! The merge queue: a lane that reported `done` is merged with main in a throwaway worktree, the project's
//! checks run there, and only a green result moves main (fast-forward), ticks the plan item and is pushed.

use crate::app::{add_worktree, git, kill_group, now_iso, tail, Lane, Shared};
use crate::config::Stage;
use crate::{lanes, manager, plan, sandbox};
use regex::Regex;
use rusqlite::params;
use serde_json::json;
use std::path::Path;
use std::process::Stdio;
use std::time::Duration;

pub async fn worker(app: Shared) {
    loop {
        let next = app
            .lanes
            .lock()
            .unwrap()
            .values()
            .filter(|l| l.status == "merge-queued")
            .min_by(|a, b| a.ended_at.cmp(&b.ended_at))
            .cloned();
        match next {
            Some(lane) => merge(&app, lane).await,
            None => {
                let _ = tokio::time::timeout(Duration::from_secs(10), app.merge_wake.notified()).await;
            }
        }
    }
}

fn glob(pattern: &str) -> Regex {
    let mut re = String::from("^");
    let mut chars = pattern.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '*' if chars.peek() == Some(&'*') => {
                chars.next();
                if chars.peek() == Some(&'/') {
                    chars.next();
                    re += "(.*/)?";
                } else {
                    re += ".*";
                }
            }
            '*' => re += "[^/]*",
            '?' => re += "[^/]",
            c => re += &regex::escape(&c.to_string()),
        }
    }
    Regex::new(&(re + "$")).unwrap_or_else(|_| Regex::new("^$").unwrap())
}

/// A stage with `paths` runs only when a changed file matches them (`!pattern` excludes).
fn wanted(stage: &Stage, changed: &[String]) -> bool {
    if stage.paths.is_empty() {
        return true;
    }
    let (neg, pos): (Vec<&String>, Vec<&String>) = stage.paths.iter().partition(|p| p.starts_with('!'));
    let neg: Vec<Regex> = neg.iter().map(|p| glob(&p[1..])).collect();
    let pos: Vec<Regex> = pos.iter().map(|p| glob(p)).collect();
    changed.iter().any(|f| (pos.is_empty() || pos.iter().any(|r| r.is_match(f))) && !neg.iter().any(|r| r.is_match(f)))
}

async fn run_stage(app: &Shared, lane: &Lane, dir: &Path, stage: &Stage) -> Result<(), String> {
    let id = {
        let db = app.db.lock().unwrap();
        let _ = db.execute(
            "INSERT INTO runs (lane, project, stage, status, started_at) VALUES (?1,?2,?3,'running',?4)",
            params![lane.id, lane.project, stage.name, now_iso()],
        );
        db.last_insert_rowid()
    };
    let log = app.home.join(format!("logs/runs/{id}-{}.log", stage.name));
    let mut row = json!({"id": id, "lane": lane.id, "project": lane.project, "stage": stage.name, "status": "running", "log": log});
    app.emit("run", row.clone());
    let file = std::fs::File::create(&log).map_err(|e| e.to_string())?;
    let launch = sandbox::wrap(app, &format!("merge:{}", lane.id), vec!["sh".into(), "-c".into(), stage.run.clone()]);
    let mut command = launch.command();
    if !launch.env.iter().any(|(k, _)| k == "PATH") {
        command.env("PATH", app.child_path());
    }
    let mut child = command
        .current_dir(dir)
        .stdin(Stdio::null())
        .stdout(file.try_clone().map_err(|e| e.to_string())?)
        .stderr(file)
        .process_group(0)
        .spawn()
        .map_err(|e| e.to_string())?;
    let limit = Duration::from_secs(app.cfg().stage_timeout_min * 60);
    let status = match tokio::time::timeout(limit, child.wait()).await {
        Ok(Ok(s)) if s.success() => "passed",
        Ok(_) => "failed",
        Err(_) => {
            if let Some(pid) = child.id() {
                kill_group(pid, libc::SIGKILL);
            }
            let _ = child.wait().await;
            "timeout"
        }
    };
    let _ = app.db.lock().unwrap().execute(
        "UPDATE runs SET status = ?1, ended_at = ?2, log = ?3 WHERE id = ?4",
        params![status, now_iso(), log.display().to_string(), id],
    );
    row["status"] = json!(status);
    app.emit("run", row);
    if status == "passed" {
        Ok(())
    } else {
        Err(format!("check '{}' {status}. Last lines of its log:\n{}", stage.name, tail(&log, 60)))
    }
}

async fn merge(app: &Shared, lane: Lane) {
    app.update_lane(&lane.id, |l| l.status = "merging".into());
    let cfg = app.cfg();
    let Some(project) = cfg.project(&lane.project).cloned() else {
        app.update_lane(&lane.id, |l| l.status = "failed".into());
        return;
    };
    let repo = project.repo();
    let manifest = app.manifest(&repo);
    plan::flush(app, Some(&lane.project)).await;
    let tmp = project.lanes_dir().join(format!("_merge-{}", lane.id));
    let cleanup = |tmp: std::path::PathBuf| {
        let repo = repo.clone();
        async move {
            let _ = git(&repo, &["worktree", "remove", "--force", &tmp.display().to_string()]).await;
            let _ = std::fs::remove_dir_all(&tmp);
        }
    };
    cleanup(tmp.clone()).await;
    let result: Result<String, String> = async {
        add_worktree(&repo, &tmp, None, &manifest.lane_exclude).await?;
        let base = git(&tmp, &["rev-parse", "HEAD"]).await?;
        let msg = format!("Merge {}: {}", lane.branch, lane.item.chars().take(120).collect::<String>());
        if let Err(e) = git(&tmp, &["merge", "--no-ff", "-q", "-m", &msg, &lane.branch]).await {
            let _ = git(&tmp, &["merge", "--abort"]).await;
            return Err(format!("conflict with main:\n{e}"));
        }
        let changed: Vec<String> = git(&tmp, &["diff", "--name-only", &base, "HEAD"]).await?.lines().map(String::from).collect();
        if changed.is_empty() {
            return Err("your branch has no commits: commit your work, then report again".into());
        }
        for stage in manifest.stages.iter().filter(|s| wanted(s, &changed)) {
            run_stage(app, &lane, &tmp, stage).await?;
        }
        git(&tmp, &["rev-parse", "HEAD"]).await
    }
    .await;
    cleanup(tmp).await;
    let sha = match result {
        Ok(sha) => sha,
        Err(reason) => return lanes::send_back(app, &lane, &reason),
    };
    if let Err(e) = git(&repo, &["merge", "--ff-only", "-q", &sha]).await {
        let moved = git(&repo, &["merge-base", "--is-ancestor", "HEAD", &sha]).await.is_err();
        app.update_lane(&lane.id, |l| {
            l.status = if moved { "merge-queued" } else { "failed" }.into();
            l.note = format!("main could not move to the tested commit: {e}");
        });
        if !moved {
            manager::notify(app, &format!("lane {} passed its checks but main could not fast-forward: {e}", lane.id));
        }
        return;
    }
    let _ = plan::tick(app, &lane.project, &lane.step, &lane.item).await;
    plan::flush(app, Some(&lane.project)).await;
    if cfg.push && git(&repo, &["remote", "get-url", "origin"]).await.is_ok() {
        if let Err(e) = git(&repo, &["push", "-q", "origin", "HEAD"]).await {
            app.event(&lane.id, &lane.project, "error", "push", &e, "");
        }
    }
    lanes::retire(&repo, &lane).await;
    app.update_lane(&lane.id, |l| {
        l.status = "merged".into();
        l.note = format!("on main at {}", &sha[..sha.len().min(8)]);
    });
    manager::notify(app, &format!("lane {} merged: {} ({})", lane.id, lane.item, lane.project));
}
