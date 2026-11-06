//! The merge queue. Up to five lanes of one project ride one train: in the project's merge slot
//! (`<repo>-lanes/merge`) their branches are merged with main, the project's checks and the items' own checks
//! run, and only a green train moves main (fast-forward), with the plan ticks in the same commits. A red train is
//! split into single lanes; a red single lane is first compared with main, so a broken main never blames a lane.

use crate::app::{add_worktree, git, kill_group, now_iso, tail, Lane, Shared};
use crate::config::{Manifest, Project, Stage};
use crate::{lanes, manager, plan, sandbox};
use regex::Regex;
use rusqlite::params;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Mutex;
use std::time::Duration;

/// Held while something uses a merge slot (a train or a main check).
static BUSY: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
/// Main checks already run: (project, main sha, stage) → green.
static MAIN_OK: Mutex<BTreeMap<(String, String, String), bool>> = Mutex::new(BTreeMap::new());

/// One pipeline: a merge attempt ("merge") or a check of main ("main").
struct Pipe {
    id: String,
    project: String,
    lanes: String,
    sha: String,
}

pub async fn worker(app: Shared) {
    let cut: Vec<String> = app.lanes.lock().unwrap().values().filter(|l| l.status == "merging").map(|l| l.id.clone()).collect();
    for id in cut {
        app.update_lane(&id, |l| l.status = "merge-queued".into()); // a merge cut short by a restart starts over
    }
    {
        // ... and its pipeline and runs, left "running" by the restart, end as interrupted.
        let db = app.db.lock().unwrap();
        let _ = db.execute("UPDATE pipelines SET status = 'interrupted', ended_at = ?1 WHERE status = 'running'", [now_iso()]);
        let _ = db.execute("UPDATE runs SET status = 'interrupted', ended_at = ?1 WHERE status = 'running'", [now_iso()]);
    }
    loop {
        let batch = next_batch(&app).await;
        if batch.is_empty() {
            if tokio::time::timeout(Duration::from_secs(10), app.merge_wake.notified()).await.is_ok() {
                tokio::time::sleep(Duration::from_secs(1)).await; // lanes ending together ride one train
            }
            continue;
        }
        let _busy = BUSY.lock().await;
        train(&app, batch).await;
    }
}

/// The oldest merge-queued lane alone when it is solo, else up to five non-solo lanes of its project. Projects
/// whose main is red wait until main changes (then the main check decides again).
async fn next_batch(app: &Shared) -> Vec<Lane> {
    let mut queued: Vec<Lane> = app.lanes.lock().unwrap().values().filter(|l| l.status == "merge-queued").cloned().collect();
    queued.sort_by(|a, b| a.ended_at.cmp(&b.ended_at));
    let mut red: Vec<String> = queued.iter().map(|l| l.project.clone()).collect();
    red.dedup();
    for p in red {
        let Some(why) = app.kv(&format!("main_red:{p}")) else { continue };
        let repo = app.cfg().project(&p).map(|p| p.repo());
        let head = match repo {
            Some(r) => git(&r, &["rev-parse", "HEAD"]).await.unwrap_or_default(),
            None => String::new(),
        };
        if head.len() < 8 || why.ends_with(&head[..8]) {
            queued.retain(|l| l.project != p);
        }
    }
    let Some(first) = queued.first().cloned() else { return vec![] };
    if first.solo {
        return vec![first];
    }
    queued.into_iter().filter(|l| l.project == first.project && !l.solo).take(5).collect()
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

fn open_pipeline(app: &Shared, project: &str, kind: &str, lanes: &str, sha: &str) -> Pipe {
    let id = {
        let db = app.db.lock().unwrap();
        let n: i64 = db.query_row("SELECT COUNT(*) FROM pipelines WHERE project = ?1", [project], |r| r.get(0)).unwrap_or(0);
        let id = format!("{project}-{}", n + 1);
        let _ = db.execute("INSERT INTO pipelines (id, project, kind, lanes, sha, status, started_at) VALUES (?1,?2,?3,?4,?5,'running',?6)", params![id, project, kind, lanes, sha, now_iso()]);
        id
    };
    let p = Pipe { id, project: project.into(), lanes: lanes.into(), sha: sha.into() };
    app.emit("pipeline", pipeline(app, &p.id));
    p
}

fn close_pipeline(app: &Shared, p: &Pipe, status: &str) {
    let _ = app.db.lock().unwrap().execute("UPDATE pipelines SET status = ?1, ended_at = ?2 WHERE id = ?3", params![status, now_iso(), p.id]);
    app.emit("pipeline", pipeline(app, &p.id));
}

/// One pipeline with its stages, as the page shows it.
fn pipeline(app: &Shared, id: &str) -> Value {
    let db = app.db.lock().unwrap();
    let row = db.query_row("SELECT id, project, kind, lanes, sha, status, started_at, ended_at FROM pipelines WHERE id = ?1", [id], |r| {
        let s = |i: usize| r.get::<_, Option<String>>(i).map(|v| v.unwrap_or_default());
        Ok(json!({"id": s(0)?, "project": s(1)?, "kind": s(2)?, "lanes": s(3)?, "sha": s(4)?, "status": s(5)?, "started_at": s(6)?, "ended_at": s(7)?}))
    });
    let Ok(mut row) = row else { return Value::Null };
    let stage = |r: &rusqlite::Row| {
        let s = |i: usize| r.get::<_, Option<String>>(i);
        Ok(json!({"id": r.get::<_, i64>(0)?, "stage": s(1)?, "status": s(2)?, "started_at": s(3)?, "ended_at": s(4)?}))
    };
    let sql = "SELECT id, stage, status, started_at, ended_at FROM runs WHERE pipeline = ?1 ORDER BY id";
    let stages: Vec<Value> = db.prepare(sql).and_then(|mut q| q.query_map([id], stage).map(|rows| rows.flatten().collect())).unwrap_or_default();
    row["stages"] = json!(stages);
    row
}

/// The newest pipelines, newest first.
pub fn list(app: &Shared, limit: u32) -> Vec<Value> {
    let ids: Vec<String> = {
        let db = app.db.lock().unwrap();
        db.prepare(&format!("SELECT id FROM pipelines ORDER BY rowid DESC LIMIT {limit}")).and_then(|mut q| q.query_map([], |r| r.get(0)).map(|rows| rows.flatten().collect())).unwrap_or_default()
    };
    ids.iter().map(|id| pipeline(app, id)).collect()
}

async fn run_stage(app: &Shared, pipe: &Pipe, dir: &Path, name: &str, run: &str) -> Result<(), String> {
    let id = {
        let db = app.db.lock().unwrap();
        let _ = db.execute("INSERT INTO runs (lane, project, stage, status, started_at, pipeline, sha) VALUES (?1,?2,?3,'running',?4,?5,?6)", params![pipe.lanes, pipe.project, name, now_iso(), pipe.id, pipe.sha]);
        db.last_insert_rowid()
    };
    let log = app.home.join(format!("logs/runs/{id}.log"));
    let mut row = json!({"id": id, "lane": pipe.lanes, "project": pipe.project, "stage": name, "status": "running", "pipeline": pipe.id, "log": log});
    app.emit("run", row.clone());
    app.emit("pipeline", pipeline(app, &pipe.id));
    let file = std::fs::File::create(&log).map_err(|e| e.to_string())?;
    let who = format!("merge:{}", pipe.lanes.split(',').next().filter(|l| !l.is_empty()).unwrap_or("main"));
    let launch = sandbox::wrap(app, &who, vec!["sh".into(), "-c".into(), run.into()]);
    let mut command = launch.command(app);
    command.env("CARGO_TARGET_DIR", crate::lanes::shared_target(app, &pipe.project));
    let mut child = command.current_dir(dir).stdin(Stdio::null()).stdout(file.try_clone().map_err(|e| e.to_string())?).stderr(file).process_group(0).spawn().map_err(|e| e.to_string())?;
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
    let _ = app.db.lock().unwrap().execute("UPDATE runs SET status = ?1, ended_at = ?2, log = ?3 WHERE id = ?4", params![status, now_iso(), log.display().to_string(), id]);
    row["status"] = json!(status);
    app.emit("run", row);
    app.emit("pipeline", pipeline(app, &pipe.id));
    if status == "passed" {
        Ok(())
    } else {
        Err(format!("check '{name}' {status}. Last lines of its log:\n{}", tail(&log, 60)))
    }
}

/// The project's persistent merge slot.
async fn merge_slot(project: &Project, manifest: &Manifest) -> Result<PathBuf, String> {
    let slot = project.lanes_dir().join("merge");
    if !slot.exists() {
        if let Err(e) = add_worktree(&project.repo(), &slot, None, &manifest.lane_exclude).await {
            drop_half_made(&slot, &e);
            return Err(e);
        }
    }
    Ok(slot)
}

/// A git error that means the disk is full.
fn disk_full(e: &str) -> bool {
    e.to_lowercase().contains("no space left on device")
}

/// A merge slot that failed to be made for lack of disk is deleted, so nothing half-made stays to eat space.
fn drop_half_made(slot: &Path, e: &str) -> bool {
    disk_full(e) && std::fs::remove_dir_all(slot).is_ok()
}

async fn reset(slot: &Path, sha: &str) -> Result<(), String> {
    git(slot, &["checkout", "-q", "-f", "--detach", sha]).await?;
    git(slot, &["clean", "-fdq"]).await.map(|_| ())
}

fn requeue(app: &Shared, lanes: &[Lane], solo: bool, note: &str) {
    for l in lanes {
        app.update_lane(&l.id, |x| {
            x.status = "merge-queued".into();
            x.solo = solo;
            x.note = note.into();
        });
    }
}

async fn train(app: &Shared, batch: Vec<Lane>) {
    let pid = batch[0].project.clone();
    for l in &batch {
        app.update_lane(&l.id, |x| x.status = "merging".into());
    }
    let Some(project) = app.cfg().project(&pid).cloned() else { return requeue(app, &batch, false, "unknown project") };
    let (repo, manifest) = (project.repo(), app.manifest(&project.repo()));
    plan::flush(app, Some(&pid)).await;
    let ready = async {
        let slot = merge_slot(&project, &manifest).await?;
        let base = git(&repo, &["rev-parse", "HEAD"]).await?;
        reset(&slot, &base).await?;
        Ok::<_, String>((slot, base))
    };
    let (slot, base) = match ready.await {
        Ok(x) => x,
        Err(e) if disk_full(&e) => {
            // Out of disk is not the lanes' fault: they leave the queue (no slot held) and wait for space.
            for l in &batch {
                app.update_lane(&l.id, |x| {
                    x.status = "waiting-disk".into();
                    x.note = "the merge worktree could not be made: no space left on device".into();
                });
            }
            return manager::notify(app, &format!("the merge of {pid} waits for disk: {e}"));
        }
        Err(e) => {
            for l in &batch {
                app.update_lane(&l.id, |x| x.status = "failed".into());
            }
            return manager::notify(app, &format!("the merge slot of {pid} is broken: {e}"));
        }
    };
    let ids: Vec<&str> = batch.iter().map(|l| l.id.as_str()).collect();
    let pipe = open_pipeline(app, &pid, "merge", &ids.join(","), &base);
    let check = |l: &Lane| plan::find(app, &pid, &l.step, &l.item).and_then(|i| i.check);
    let (mut merged, mut ticks, mut todo) = (vec![], vec![], vec![]);
    for lane in batch {
        let ahead = git(&repo, &["rev-list", "--count", &format!("{base}..{}", lane.branch)]).await.unwrap_or_default();
        if !matches!(ahead.as_str(), "0" | "") {
            todo.push(lane);
        } else if let Some(c) = check(&lane) {
            match run_stage(app, &pipe, &slot, &format!("check {}", lane.id), &c).await {
                Ok(()) => ticks.push(lane),
                Err(e) => lanes::send_back(app, &lane, &format!("nothing to merge, and the item's check fails on main: commit your work, or ask the manager with tirith ask.\n{e}")),
            }
        } else {
            lanes::send_back(app, &lane, "nothing to merge: commit your work, or ask the manager with tirith ask");
        }
    }
    for lane in todo {
        let msg = format!("Merge {}: {}", lane.branch, lane.item.chars().take(120).collect::<String>());
        match git(&slot, &["merge", "--no-ff", "-q", "-m", &msg, &lane.branch]).await {
            Ok(_) => merged.push(lane),
            Err(e) => {
                let _ = git(&slot, &["merge", "--abort"]).await;
                lanes::send_back(app, &lane, &format!("conflict with main: merge main into your branch and resolve it.\n{e}"));
            }
        }
    }
    if merged.is_empty() && ticks.is_empty() {
        return close_pipeline(app, &pipe, "failed");
    }
    let changed: Vec<String> = git(&slot, &["diff", "--name-only", &base, "HEAD"]).await.unwrap_or_default().lines().map(String::from).collect();
    let mut stages: Vec<(String, String)> = if merged.is_empty() { vec![] } else { manifest.stages.iter().filter(|s| wanted(s, &changed)).map(|s| (s.name.clone(), s.run.clone())).collect() };
    stages.extend(merged.iter().filter_map(|l| check(l).map(|c| (format!("check {}", l.id), c))));
    for (name, run) in &stages {
        if let Err(why) = run_stage(app, &pipe, &slot, name, run).await {
            close_pipeline(app, &pipe, "failed");
            requeue(app, &ticks, false, "its train was red: checked again");
            if merged.len() > 1 {
                requeue(app, &merged, true, "its train was red: merged alone next");
                app.merge_wake.notify_one();
                return;
            }
            let lane = merged.remove(0);
            if let Some(st) = manifest.stages.iter().find(|s| s.name == *name) {
                if !main_check(app, &project, &slot, &base, st, false).await {
                    let key = format!("main_red:{pid}");
                    let was = app.kv(&key);
                    app.set_kv(&key, Some(&format!("{name} at {}", &base[..8.min(base.len())])));
                    app.emit("plan", json!({"project": pid}));
                    requeue(app, &[lane], true, "main is red: waits for a green main");
                    if was.is_none() {
                        manager::notify(app, &format!("main of {pid} is red: check '{name}' fails on main itself ({}). The autopilot pauses {pid} until main is green; fix main or retry the main pipeline.", &base[..8.min(base.len())]));
                    }
                    return;
                }
            }
            return lanes::send_back(app, &lane, &why);
        }
    }
    land(app, &project, &pipe, &slot, &base, merged, ticks, &changed).await;
}

/// A green train: under the plan lock, pending plan edits are committed on main, the items are ticked in the
/// merge slot, and main fast-forwards to the result.
#[allow(clippy::too_many_arguments)]
async fn land(app: &Shared, project: &Project, pipe: &Pipe, slot: &Path, base: &str, merged: Vec<Lane>, ticks: Vec<Lane>, changed: &[String]) {
    let (repo, roadmap) = (project.repo(), app.manifest(&project.repo()).roadmap);
    let roadmap = roadmap.trim_start_matches("./").to_string();
    let done: Vec<Lane> = merged.iter().chain(&ticks).cloned().collect();
    let result = async {
        let _plan = app.plan_lock.lock().await;
        plan::flush_locked(app, &project.id).await;
        let head = git(&repo, &["rev-parse", "HEAD"]).await?;
        if head != base {
            if git(&repo, &["diff", "--name-only", base, &head]).await?.lines().any(|f| f != roadmap) {
                return Err("main moved while the checks ran".to_string());
            }
            git(slot, &["merge", "-q", "--no-edit", &head]).await?; // only plan edits: no new check needed
        }
        let file = slot.join(&roadmap);
        let items: Vec<(String, String)> = done.iter().map(|l| (l.step.clone(), l.item.clone())).collect();
        let (text, n) = plan::tick_text(&std::fs::read_to_string(&file).map_err(|e| e.to_string())?, &items);
        if n > 0 {
            std::fs::write(&file, text).map_err(|e| e.to_string())?;
            git(slot, &["commit", "-q", "-m", &format!("Plan: done {n} item(s)"), "--", &roadmap]).await?;
        }
        let sha = git(slot, &["rev-parse", "HEAD"]).await?;
        git(&repo, &["merge", "--ff-only", "-q", &sha]).await?;
        Ok(sha)
    };
    let sha = match result.await {
        Ok(sha) => sha,
        Err(e) => {
            close_pipeline(app, pipe, "failed");
            app.event("", &project.id, "error", "merge", &format!("main could not move: {e}"), "");
            return requeue(app, &done, false, &format!("main could not move: {e}"));
        }
    };
    close_pipeline(app, pipe, "passed");
    clear_red(app, &project.id);
    let sha8 = &sha[..8.min(sha.len())];
    if app.cfg().push && git(&repo, &["remote", "get-url", "origin"]).await.is_ok() {
        if let Err(e) = git(&repo, &["push", "-q", "origin", "HEAD"]).await {
            app.event("", &project.id, "error", "push", &e, "");
        }
    }
    for l in &done {
        if !l.workspace.is_empty() {
            let _ = git(Path::new(&l.workspace), &["checkout", "-q", "--detach"]).await;
        }
        let _ = git(&repo, &["branch", "-D", &l.branch]).await;
        let tick_only = ticks.iter().any(|t| t.id == l.id);
        app.update_lane(&l.id, |x| {
            x.status = "merged".into();
            x.merged_at = Some(now_iso());
            x.note = if tick_only { "already true on main".into() } else { format!("on main at {sha8}") };
        });
    }
    let items: Vec<&str> = done.iter().map(|l| l.item.as_str()).collect();
    let files: Vec<&str> = changed.iter().take(10).map(String::as_str).collect();
    let working: Vec<String> = app.lanes.lock().unwrap().values().filter(|l| l.project == project.id && l.pid.is_some()).map(|l| l.id.clone()).collect();
    for id in working {
        lanes::tell(&id, &format!("[tirith] main advanced: {} (files: {})", items.join("; "), files.join(", ")));
    }
    manager::notify(app, &format!("merged on {} at {sha8}: {}", project.id, items.join("; ")));
    app.emit("plan", json!({"project": project.id}));
    app.sched_wake.notify_one();
}

/// Runs one stage on a main commit in the merge slot (cached per commit unless `fresh`); green clears main_red.
async fn main_check(app: &Shared, project: &Project, slot: &Path, sha: &str, stage: &Stage, fresh: bool) -> bool {
    let key = (project.id.clone(), sha.to_string(), stage.name.clone());
    if let Some(ok) = MAIN_OK.lock().unwrap().get(&key).filter(|_| !fresh) {
        return *ok;
    }
    let pipe = open_pipeline(app, &project.id, "main", "", sha);
    let ok = reset(slot, sha).await.is_ok() && run_stage(app, &pipe, slot, &stage.name, &stage.run).await.is_ok();
    close_pipeline(app, &pipe, if ok { "passed" } else { "failed" });
    MAIN_OK.lock().unwrap().insert(key, ok);
    if ok {
        clear_red(app, &project.id);
    }
    ok
}

fn clear_red(app: &Shared, project: &str) {
    let key = format!("main_red:{project}");
    if app.kv(&key).is_some() {
        app.set_kv(&key, None);
        app.emit("plan", json!({"project": project}));
        manager::notify(app, &format!("main of {project} is green again: the autopilot continues"));
        app.sched_wake.notify_one();
        app.merge_wake.notify_one();
    }
}

/// Retry from the page: a merge pipeline's failed lanes go back to the queue; a main pipeline runs its checks
/// again on the current main.
pub fn retry(app: &Shared, id: &str) -> Result<(), String> {
    let p = pipeline(app, id);
    if p.is_null() {
        return Err(format!("no pipeline '{id}'"));
    }
    if p["kind"] == "merge" {
        let failed: Vec<Lane> = p["lanes"].as_str().unwrap_or("").split(',').filter_map(|l| app.lane(l)).filter(|l| l.status == "failed").collect();
        if failed.is_empty() {
            return Err("no lane of this pipeline is failed".into());
        }
        for l in &failed {
            app.update_lane(&l.id, |x| x.attempts = 0);
        }
        requeue(app, &failed, false, "retried");
        app.merge_wake.notify_one();
        return Ok(());
    }
    let project = app.cfg().project(p["project"].as_str().unwrap_or("")).cloned().ok_or("unknown project")?;
    let names: Vec<String> = p["stages"].as_array().into_iter().flatten().filter_map(|s| s["stage"].as_str().map(String::from)).collect();
    let app = app.clone();
    tokio::spawn(async move {
        let _busy = BUSY.lock().await;
        let manifest = app.manifest(&project.repo());
        let (Ok(slot), Ok(sha)) = (merge_slot(&project, &manifest).await, git(&project.repo(), &["rev-parse", "HEAD"]).await) else { return };
        for st in manifest.stages.iter().filter(|s| names.contains(&s.name)) {
            main_check(&app, &project, &slot, &sha, st, true).await;
        }
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mergeq_enospc_deletes_the_folder_and_holds_no_slot() {
        let dir = std::env::temp_dir().join(format!("tirith-enospc-{}", std::process::id()));
        std::fs::create_dir_all(dir.join(".git")).unwrap();
        let err = "git worktree add: fatal: write error: No space left on device";
        assert!(disk_full(err));
        assert!(drop_half_made(&dir, err));
        assert!(!dir.exists());
        // another failure keeps the folder
        std::fs::create_dir_all(&dir).unwrap();
        assert!(!drop_half_made(&dir, "git worktree add: fatal: conflict"));
        assert!(dir.exists());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
