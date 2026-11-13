//! The merge queue. Up to five lanes of one project ride one train: in the project's merge slot (`<tirith home>/lanes/<project>/merge`) their branches are merged with main, the project's checks and the items' own checks run, and
//! only a green train moves main (fast-forward), with the plan ticks in the same commits. A red train is split into single lanes; a red single lane is first compared with main, so a broken main never blames a lane. A project's
//! deploy stages are no checks: they run after main has moved, on that main (`deploy`).

use crate::app::{add_worktree, git, kill_group, now_iso, now_s, tail, Lane, Shared};
use crate::config::{Manifest, Project, Stage};
use crate::{lanes, manager, plan, sandbox};
use regex::Regex;
use rusqlite::params;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::{collections::BTreeMap, process::Stdio, sync::Mutex, time::Duration};

/// Held while something uses a merge slot (a train or a main check).
static BUSY: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
/// Main checks already run: (project, main sha, stage) → green.
static MAIN_OK: Mutex<BTreeMap<(String, String, String), bool>> = Mutex::new(BTreeMap::new());

/// One lock per project: test stages of one project never run at the same time (servers on one port, one folder).
static STAGE_LOCKS: Mutex<BTreeMap<String, std::sync::Arc<tokio::sync::Mutex<()>>>> = Mutex::new(BTreeMap::new());

fn stage_lock(project: &str) -> std::sync::Arc<tokio::sync::Mutex<()>> {
    STAGE_LOCKS.lock().unwrap().entry(project.to_string()).or_default().clone()
}

/// One pipeline: a merge attempt ("merge") or a check of main ("main").
struct Pipe {
    id: String,
    project: String,
    lanes: String,
    sha: String,
}

pub async fn worker(app: Shared) {
    let cut: Vec<String> = app.lanes.lock().unwrap().values().filter(|l| l.status == "merging").map(|l| l.id.clone()).collect();
    cut.iter().for_each(|id| drop(app.update_lane(id, |l| l.status = "merge-queued".into()))); // a merge cut short by a restart starts over
    {
        // ... and its pipeline and runs, left "running" by the restart, end as interrupted.
        let db = app.db.lock().unwrap();
        let _ = db.execute("UPDATE pipelines SET status = 'interrupted', ended_at = ?1 WHERE status = 'running'", [now_iso()]);
        let _ = db.execute("UPDATE runs SET status = 'interrupted', ended_at = ?1 WHERE status = 'running'", [now_iso()]);
    }
    loop {
        recheck_red(&app).await;
        let batch = next_batch(&app).await;
        if batch.is_empty() {
            if tokio::time::timeout(Duration::from_secs(10), app.merge_wake.notified()).await.is_ok() {
                tokio::time::sleep(Duration::from_secs(1)).await; // lanes ending together ride one train
            }
            // A deploy that failed, or that a restart cut short, is tried again on main as it is now. The lock comes first: a deploy that the page asked for has settled what is due by the time the lock is had.
            let _busy = BUSY.lock().await;
            let due = |p: &&Project| app.kv(&format!("deploy_due:{}", p.id)).and_then(|v| v.split('@').next()?.parse().ok()).is_some_and(|t: u64| now_s() >= t);
            if let Some(project) = app.cfg().projects.iter().find(due).cloned() {
                deploy(&app, &project, &[]).await;
            }
            continue;
        }
        let _busy = BUSY.lock().await;
        train(&app, batch).await;
    }
}

/// A `main_red` value is "<stage> at <sha8>": it is stale when main's head is another commit.
fn red_stage_if_stale(why: &str, head: &str) -> Option<String> {
    if head.len() < 8 || why.ends_with(&head[..8]) {
        return None;
    }
    why.rsplit_once(" at ").map(|(stage, _)| stage.to_string())
}

/// After main moves while it is red (a merge, a manual commit), the queue tests the latest main again, so the health «Main en rojo» names the current commit and clears when it passes.
async fn recheck_red(app: &Shared) {
    let cfg = app.cfg();
    for project in &cfg.projects {
        let Some(why) = app.kv(&format!("main_red:{}", project.id)) else { continue };
        let head = git(&project.repo(), &["rev-parse", "HEAD"]).await.unwrap_or_default();
        let Some(stage) = red_stage_if_stale(&why, &head) else { continue };
        let _busy = BUSY.lock().await;
        let manifest = app.manifest(&project.repo());
        let (Some(st), Ok(slot)) = (manifest.stages.iter().find(|s| s.name == stage), merge_slot(project, &manifest).await) else { continue };
        if !main_check(app, project, &slot, &head, st, false).await {
            app.set_kv(&format!("main_red:{}", project.id), Some(&format!("{stage} at {}", &head[..8])));
            app.emit("plan", json!({"project": project.id}));
        }
    }
}

/// The oldest merge-queued lane alone when it is solo, else up to five non-solo lanes of its project. Projects whose main is red wait until main changes (then the main check decides again).
async fn next_batch(app: &Shared) -> Vec<Lane> {
    let mut queued: Vec<Lane> = app.lanes.lock().unwrap().values().filter(|l| l.status == "merge-queued").cloned().collect();
    queued.sort_by(|a, b| a.ended_at.cmp(&b.ended_at));
    let mut red: Vec<String> = queued.iter().map(|l| l.project.clone()).collect();
    red.dedup();
    for p in red {
        let Some(why) = app.kv(&format!("main_red:{p}")) else { continue };
        let head = match app.cfg().project(&p).map(|p| p.repo()) {
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
    let re = regex::escape(pattern).replace(r"\*\*/", "(.*/)?").replace(r"\*\*", ".*").replace(r"\*", "[^/]*").replace(r"\?", "[^/]");
    Regex::new(&format!("^{re}$")).unwrap_or_else(|_| Regex::new("^$").unwrap())
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
    let mut rows = app.objects("SELECT id, project, kind, lanes, sha, status, started_at, ended_at FROM pipelines WHERE id = ?1", [id], &["id", "project", "kind", "lanes", "sha", "status", "started_at", "ended_at"]);
    let Some(mut row) = rows.pop() else { return Value::Null };
    row["stages"] = json!(app.objects("SELECT id, stage, status, started_at, ended_at FROM runs WHERE pipeline = ?1 ORDER BY id", [id], &["id", "stage", "status", "started_at", "ended_at"]));
    row
}

/// The newest pipelines (of one project, or of all with ""), newest first.
pub fn list(app: &Shared, project: &str, limit: u32) -> Vec<Value> {
    let sql = format!("SELECT id FROM pipelines WHERE ?1 = '' OR project = ?1 ORDER BY rowid DESC LIMIT {limit}");
    app.query(&sql, [project], |r| r.get::<_, String>(0)).iter().map(|id| pipeline(app, id)).collect()
}

const MANAGER_LIMIT_SECS: u64 = 24 * 3600;

/// The limit in minutes for a stage: the manager's (within 24 h), else the one learned from past timeouts, else the default. Each project and stage has its own.
fn limit_from(get: impl Fn(&str) -> Option<String>, project: &str, stage: &str, default: u64) -> u64 {
    let set = get(&format!("stage_limit:{project}:{stage}")).and_then(|v| {
        let (m, at) = v.split_once('@')?;
        (now_s().saturating_sub(at.parse().ok()?) < MANAGER_LIMIT_SECS).then(|| m.parse().ok()).flatten()
    });
    set.or_else(|| get(&format!("stage_learned:{project}:{stage}")).and_then(|v| v.parse().ok())).unwrap_or(default)
}

pub fn stage_limit_min(app: &Shared, project: &str, stage: &str) -> u64 {
    limit_from(|k| app.kv(k), project, stage, app.cfg().stage_timeout_min)
}
/// An item's check runs with the `VAR=value` words (JAVA_HOME, ANDROID_HOME) of the pipeline stage it belongs to: the stage named in it, or one running the same program.
fn stage_env(stages: &[Stage], check: &str) -> Vec<(String, String)> {
    let said: Vec<String> = check.split(|c: char| !c.is_alphanumeric() && c != '_').map(str::to_lowercase).collect();
    let assign = |w: &str| w.split_once('=').filter(|(k, _)| !k.is_empty() && k.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')).map(|(k, v)| (k.to_string(), v.trim_matches(['"', '\'']).to_string()));
    let same = |s: &Stage| said.contains(&s.name.to_lowercase()) || s.run.split_whitespace().any(|w| w.len() > 4 && assign(w).is_none() && said.contains(&w.rsplit('/').next().unwrap_or("").to_lowercase()));
    stages.iter().filter(|s| same(s)).flat_map(|s| s.run.split_whitespace().filter_map(assign)).collect()
}
async fn run_stage(app: &Shared, pipe: &Pipe, dir: &Path, name: &str, run: &str) -> Result<(), String> {
    let lock = stage_lock(&pipe.project);
    let _one_at_a_time = lock.lock().await;
    let id = app.insert("INSERT INTO runs (lane, project, stage, status, started_at, pipeline, sha) VALUES (?1,?2,?3,'running',?4,?5,?6)", params![pipe.lanes, pipe.project, name, now_iso(), pipe.id, pipe.sha]);
    let log = app.home.join(format!("logs/runs/{id}.log"));
    let mut row = json!({"id": id, "lane": pipe.lanes, "project": pipe.project, "stage": name, "status": "running", "pipeline": pipe.id, "log": log});
    app.emit("run", row.clone());
    app.emit("pipeline", pipeline(app, &pipe.id));
    let file = std::fs::File::create(&log).map_err(|e| e.to_string())?;
    let who = format!("merge:{}", pipe.lanes.split(',').next().filter(|l| !l.is_empty()).unwrap_or("main"));
    let launch = sandbox::wrap(app, &who, vec!["sh".into(), "-c".into(), run.into()]);
    let mut command = launch.command(app);
    command.env("CARGO_TARGET_DIR", crate::lanes::shared_target(app, &pipe.project));
    if name.starts_with("check ") {
        command.envs(stage_env(&Manifest::load(dir).stages, run));
    }
    let mut child = command.current_dir(dir).stdin(Stdio::null()).stdout(file.try_clone().map_err(|e| e.to_string())?).stderr(file).process_group(0).spawn().map_err(|e| e.to_string())?;
    let limit = Duration::from_secs(stage_limit_min(app, &pipe.project, name) * 60);
    let status = match tokio::time::timeout(limit, child.wait()).await {
        Ok(Ok(s)) if s.success() => "passed",
        Ok(_) => "failed",
        Err(_) => {
            if let Some(pid) = child.id() {
                kill_group(pid, libc::SIGKILL);
            }
            let _ = child.wait().await;
            // A stage that timed out gets a doubled limit next time, for that project and stage only.
            app.set_kv(&format!("stage_learned:{}:{name}", pipe.project), Some(&(stage_limit_min(app, &pipe.project, name) * 2).to_string()));
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
    batch.iter().for_each(|l| drop(app.update_lane(&l.id, |x| x.status = "merging".into())));
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
            return manager::notify(app, &pid, &format!("the merge of {pid} waits for disk: {e}"));
        }
        Err(e) => {
            batch.iter().for_each(|l| drop(app.update_lane(&l.id, |x| x.status = "failed".into())));
            return manager::notify(app, &pid, &format!("the merge slot of {pid} is broken: {e}"));
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
    let mut stages: Vec<(String, String)> = if merged.is_empty() { vec![] } else { manifest.stages.iter().filter(|s| !deploys(s) && wanted(s, &changed)).map(|s| (s.name.clone(), s.run.clone())).collect() };
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
                    let (was, sha) = (app.kv(&key), &base[..8.min(base.len())]);
                    app.set_kv(&key, Some(&format!("{name} at {sha}")));
                    app.emit("plan", json!({"project": pid}));
                    requeue(app, &[lane], true, "main is red: waits for a green main");
                    if was.is_none() {
                        manager::notify(app, &pid, &format!("main of {pid} is red: check '{name}' fails on main itself ({sha}). The autopilot pauses {pid} until main is green; fix main or retry the main pipeline."));
                    }
                    return;
                }
            }
            return lanes::send_back(app, &lane, &why);
        }
    }
    land(app, &project, &pipe, &slot, &base, merged, ticks, &changed).await;
}

/// A green train: under the plan lock, pending plan edits are committed on main, the items are ticked in the merge slot, and main fast-forwards to the result.
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
    // Main has moved: its deploy is owed from here on, so a daemon stopped before the end of this function deploys when it is back.
    let owed = format!("deploy_due:{}", project.id);
    if app.kv(&owed).is_none() {
        app.set_kv(&owed, Some("0@0"));
    }
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
    // A merge is news, not a question: the manager gets a turn for it only when it leaves the plan without open work.
    manager::inform(app, &project.id, &format!("merged on {} at {sha8}: {}", project.id, items.join("; ")));
    if plan::read(app, &project.id).is_ok_and(|steps| !steps.iter().flat_map(|s| &s.items).any(|i| !i.done && !lanes::is_autopilot_skipped(&i.text))) {
        manager::tell(app, &project.id, "The plan has no open item left: tell the owner what is done, and ask for the next goal if the project needs one.");
    }
    app.emit("plan", json!({"project": project.id}));
    app.sched_wake.notify_one();
    deploy(app, project, changed).await;
}

/// A deploy stage (its `kind`, else a name that says so) is no check of a lane: it publishes main.
fn deploys(s: &Stage) -> bool {
    let kind = s.kind.trim().to_ascii_lowercase();
    kind == "deploy" || (kind.is_empty() && ["deploy", "release", "publish", "ship"].contains(&s.name.as_str()))
}

/// Runs the project's deploy stages on main as it is now, in the merge slot, so what is on main is what is deployed. After a merge only the stages whose `paths` match what it changed run; with no files named, or while an earlier
/// deploy has not passed yet, all of them do. A failure never undoes or holds back a merge: the manager is told once, and the worker tries again after ten minutes, then after twice as long each time, six hours at most.
/// `deploy_due:<project>` is "<when to try again>@<the wait since the last failure, 0 for none>"; "0@.." while a deploy is owed or runs, so one that a restart cut short is due at once.
async fn deploy(app: &Shared, project: &Project, changed: &[String]) {
    let (manifest, key) = (app.manifest(&project.repo()), format!("deploy_due:{}", project.id));
    let prev = app.kv(&key).and_then(|v| v.split_once('@')?.1.parse::<u64>().ok()).unwrap_or(0);
    let changed = if prev > 0 { &[][..] } else { changed };
    let stages: Vec<&Stage> = manifest.stages.iter().filter(|s| deploys(s) && (changed.is_empty() || wanted(s, changed))).collect();
    if stages.is_empty() {
        return app.set_kv(&key, None); // nothing to deploy for this merge, or no deploy stage at all: nothing is owed
    }
    app.set_kv(&key, Some(&format!("0@{prev}")));
    let sha = git(&project.repo(), &["rev-parse", "HEAD"]).await.unwrap_or_default();
    let pipe = open_pipeline(app, &project.id, "deploy", "", &sha);
    let run = async {
        let slot = merge_slot(project, &manifest).await?;
        reset(&slot, &sha).await?;
        for st in stages {
            // Each try starts with the usual time limit: a deploy that hangs must not hold the merge slot twice as long every time.
            app.set_kv(&format!("stage_learned:{}:{}", project.id, st.name), None);
            run_stage(app, &pipe, &slot, &st.name, &st.run).await?;
        }
        Ok::<_, String>(())
    };
    let failed = run.await.err();
    close_pipeline(app, &pipe, if failed.is_none() { "passed" } else { "failed" });
    let (wait, sha8, pid) = (if prev == 0 { 600 } else { (prev * 2).min(6 * 3600) }, &sha[..8.min(sha.len())], &project.id);
    app.set_kv(&key, failed.as_ref().map(|_| format!("{}@{wait}", now_s() + wait)).as_deref());
    match failed {
        None => manager::inform(app, pid, &format!("deployed {pid} at {sha8}")),
        Some(e) if prev == 0 => manager::notify(app, pid, &format!("the deploy of {pid} at {sha8} failed. The merge stays on main; Tirith tries the deploy again by itself until it passes (Retry on its pipeline: at once).\n{e}")),
        _ => {}
    }
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
        manager::inform(app, project, &format!("main of {project} is green again: the autopilot continues"));
        app.sched_wake.notify_one();
        app.merge_wake.notify_one();
    }
}

/// Retry from the page: a merge pipeline's failed lanes go back to the queue; a main pipeline runs its checks again on the current main; a deploy pipeline deploys the current main.
pub fn retry(app: &Shared, id: &str) -> Result<(), String> {
    let p = pipeline(app, id);
    (!(p.is_null())).then_some(()).ok_or_else(|| format!("no pipeline '{id}'"))?;
    if p["kind"] == "merge" {
        let failed: Vec<Lane> = p["lanes"].as_str().unwrap_or("").split(',').filter_map(|l| app.lane(l)).filter(|l| l.status == "failed").collect();
        if failed.is_empty() {
            return Err("no lane of this pipeline is failed".into());
        }
        failed.iter().for_each(|l| drop(app.update_lane(&l.id, |x| x.attempts = 0)));
        requeue(app, &failed, false, "retried");
        app.merge_wake.notify_one();
        return Ok(());
    }
    let project = app.cfg().project(p["project"].as_str().unwrap_or("")).cloned().ok_or("unknown project")?;
    let names: Vec<String> = p["stages"].as_array().into_iter().flatten().filter_map(|s| s["stage"].as_str().map(String::from)).collect();
    let (app, deploying) = (app.clone(), p["kind"] == "deploy");
    tokio::spawn(async move {
        let _busy = BUSY.lock().await;
        if deploying {
            return deploy(&app, &project, &[]).await;
        }
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
    fn stage_timeouts_per_project() {
        let kv = std::collections::HashMap::from([
            ("stage_learned:zoo:build".to_string(), "60".to_string()),
            ("stage_limit:tirith:test".to_string(), format!("99@{}", now_s())),
            ("stage_limit:tirith:old".to_string(), format!("99@{}", now_s() - MANAGER_LIMIT_SECS - 1)),
        ]);
        let get = |k: &str| kv.get(k).cloned();
        let got = [("zoo", "build"), ("tirith", "build"), ("tirith", "test"), ("tirith", "old")].map(|(p, st)| limit_from(get, p, st, 30));
        assert_eq!(got, [60, 30, 99, 30]);
    }
    #[tokio::test]
    async fn project_test_stages_serialized() {
        assert!(std::sync::Arc::ptr_eq(&stage_lock("p1"), &stage_lock("p1")) && !std::sync::Arc::ptr_eq(&stage_lock("p1"), &stage_lock("p2")));
        let (lock, running) = (stage_lock("ser"), std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0)));
        let mut tasks = vec![];
        for _ in 0..4 {
            let (lock, running) = (stage_lock("ser"), running.clone());
            tasks.push(tokio::spawn(async move {
                let _g = lock.lock().await;
                assert_eq!(running.fetch_add(1, std::sync::atomic::Ordering::SeqCst), 0);
                tokio::time::sleep(Duration::from_millis(20)).await;
                running.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
            }));
        }
        drop(lock);
        for t in tasks {
            t.await.unwrap();
        }
        assert!(include_str!("merge.rs").contains("let _one_at_a_time = lock.lock().await;"));
    }
    #[test]
    fn main_red_uses_latest_main() {
        let head = "0e89bb4aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        assert_eq!(red_stage_if_stale("test at d7fa7cd8", head), Some("test".into()));
        assert_eq!((red_stage_if_stale("cargo test at 0e89bb4a", head), red_stage_if_stale("test at d7fa7cd8", "")), (None, None));
    }
    #[test]
    fn mergeq_enospc_deletes_the_folder_and_holds_no_slot() {
        let dir = std::env::temp_dir().join(format!("tirith-enospc-{}", std::process::id()));
        let err = "git worktree add: fatal: write error: No space left on device";
        std::fs::create_dir_all(dir.join(".git")).unwrap();
        assert!(disk_full(err) && drop_half_made(&dir, err) && !dir.exists());
        // another failure keeps the folder
        std::fs::create_dir_all(&dir).unwrap();
        assert!(!drop_half_made(&dir, "git worktree add: fatal: conflict") && dir.exists());
        std::fs::remove_dir_all(&dir).unwrap();
    }
    #[test]
    fn item_check_uses_stage_env() {
        let st = |name: &str, run: &str| Stage { name: name.into(), run: run.into(), paths: vec![], kind: String::new() };
        let stages = [st("android", "cd android && JAVA_HOME=/jdk17 ANDROID_HOME=\"/sdk\" ./gradlew assembleDebug"), st("build", "cargo build")];
        let env = stage_env(&stages, "cd android && ./gradlew lint");
        assert_eq!(env, [("JAVA_HOME".into(), "/jdk17".into()), ("ANDROID_HOME".into(), "/sdk".into())]);
        assert_eq!(stage_env(&stages, "cargo test -q x"), []);
    }
}
