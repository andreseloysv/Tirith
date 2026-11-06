//! Lanes: one agent, one plan item, one git worktree on its own branch. Started only from an open plan item,
//! admitted when the machine has room, stopped after a stretch without activity, merged only after it reports.

use crate::app::{add_worktree, alive, git, kill_group, now_iso, now_s, Lane, Report, Shared};
use crate::{guard, manager, models, plan, sandbox};
use std::path::Path;
use std::process::Stdio;
use std::time::Duration;

fn slug(text: &str) -> String {
    let s: String = text.to_lowercase().chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '-' }).collect();
    let s = s.split('-').filter(|p| !p.is_empty()).collect::<Vec<_>>().join("-");
    s.chars().take(32).collect::<String>().trim_end_matches('-').to_string()
}

pub fn start(app: &Shared, project: &str, step: &str, item: &str, agent: Option<String>) -> Result<Lane, String> {
    let item = plan::clean(item);
    if !plan::is_open(app, project, step, &item)? {
        return Err(format!(
            "\"{item}\" is not an open item of step {step} of {project}. Add it first: tirith plan add {project} {step} \"<text>\""
        ));
    }
    if let Some(l) = app.lanes.lock().unwrap().values().find(|l| l.active() && l.project == project && l.step == step && l.item == item) {
        return Err(format!("lane {} already works on that item ({})", l.id, l.status));
    }
    let lane = Lane {
        id: format!("{}-{:04x}", slug(&item), nanos() & 0xffff),
        project: project.into(),
        step: step.into(),
        item,
        agent: agent.unwrap_or_else(|| app.cfg().worker_agent()),
        status: "queued".into(),
        created_at: now_iso(),
        ..Default::default()
    };
    app.save_lane(&lane);
    app.sched_wake.notify_one();
    Ok(lane)
}

fn load_per_core() -> f64 {
    let mut l = [0f64; 3];
    let n = unsafe { libc::getloadavg(l.as_mut_ptr(), 3) };
    let cores = std::thread::available_parallelism().map(|c| c.get()).unwrap_or(1) as f64;
    if n > 0 { l[0] / cores } else { 0.0 }
}

/// Every few seconds (or when woken): admit queued lanes, stop idle ones, pick up lanes whose process ended.
pub async fn scheduler(app: Shared) {
    loop {
        let _ = tokio::time::timeout(Duration::from_secs(3), app.sched_wake.notified()).await;
        app.reload_config();
        let cfg = app.cfg();
        let load = load_per_core();
        let lanes: Vec<Lane> = app.lanes.lock().unwrap().values().cloned().collect();
        let running = lanes.iter().filter(|l| l.status == "running").count();
        app.emit("load", serde_json::json!({"load": (load * 100.0).round() / 100.0, "running": running, "max": cfg.max_parallel}));
        for l in lanes.iter().filter(|l| l.status == "running") {
            let tracked = app.children.lock().unwrap().contains(&l.id);
            let log_idle = std::fs::metadata(app.home.join(format!("logs/lanes/{}.log", l.id)))
                .and_then(|m| m.modified())
                .map(|t| t.elapsed().map(|d| d.as_secs()).unwrap_or(0))
                .unwrap_or(u64::MAX);
            let idle = (now_s().saturating_sub(l.last_activity)).min(log_idle);
            if idle > cfg.inactivity_min * 60 {
                if let Some(pid) = l.pid {
                    kill_group(pid, libc::SIGKILL);
                }
                app.update_lane(&l.id, |x| x.note = format!("stopped: no activity for {} min", cfg.inactivity_min));
            } else if !tracked && !l.pid.is_some_and(alive) {
                ended(&app, &l.id, None).await; // its process ended while the daemon was down
            }
        }
        if running < cfg.max_parallel && (running == 0 || load < cfg.max_load) {
            if let Some(next) = lanes.iter().filter(|l| l.status == "queued").min_by(|a, b| a.created_at.cmp(&b.created_at)) {
                if let Err(e) = launch(&app, next.clone()).await {
                    app.update_lane(&next.id, |x| {
                        x.status = "failed".into();
                        x.note = e.clone();
                        x.ended_at = Some(now_iso());
                    });
                    manager::notify(&app, &format!("lane {} could not start: {e}", next.id));
                }
            }
        }
    }
}

fn brief(lane: &Lane) -> String {
    format!(
        "# Lane {id}\n\nProject: {project}. Plan step {step}.\n\n## Your item\n{item}\n\n\
         ## How you work\n- You are in your own git worktree ({ws}) on branch {branch}. Change files only here.\n\
         - Commit your work on this branch. Never push, never touch main: Tirith merges after the project's checks pass.\n\
         - Do not start other agents or sub-agents. If you need help, ask for a lane: tirith lane start {project} <step> \"<item>\".\n\
         - When you finish, report: tirith report --status done --note \"<what you changed and how you checked it>\"\n\
         - If you cannot finish: tirith report --status partial|blocked --note \"<why, and what is needed>\"\n",
        id = lane.id,
        project = lane.project,
        step = lane.step,
        item = lane.item,
        ws = lane.workspace,
        branch = lane.branch,
    ) + &format!("\nBrief written {} by Tirith.\n", now_iso())
}

async fn launch(app: &Shared, mut lane: Lane) -> Result<(), String> {
    let cfg = app.cfg();
    let project = cfg.project(&lane.project).ok_or("unknown project")?.clone();
    guard::intact(app, &project)?;
    let repo = project.repo();
    let ws = project.lanes_dir().join(&lane.id);
    lane.branch = format!("lane/{}", lane.id);
    lane.workspace = ws.display().to_string();
    if !ws.exists() {
        add_worktree(&repo, &ws, Some(&lane.branch), &app.manifest(&repo).lane_exclude).await?;
    }
    let brief_path = app.home.join(format!("briefs/{}.md", lane.id));
    let addendum = std::fs::read_to_string(&brief_path).ok().and_then(|b| b.split_once("\n## Sent back\n").map(|(_, r)| r.to_string()));
    let mut text = brief(&lane);
    if let Some(back) = addendum {
        text += &format!("\n## Sent back\n{back}");
    }
    std::fs::write(&brief_path, text).map_err(|e| e.to_string())?;
    let override_agent = (lane.agent != models::role(app, "workers").agent).then_some(lane.agent.as_str());
    let cmd = models::lane_argv(app, &lane.project, override_agent, None, &brief_path, &ws, None)?;
    let launch = sandbox::wrap(app, &lane.id, cmd.argv);
    let log = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(app.home.join(format!("logs/lanes/{}.log", lane.id)))
        .map_err(|e| e.to_string())?;
    let mut command = launch.command();
    if !launch.env.iter().any(|(k, _)| k == "PATH") {
        command.env("PATH", app.child_path());
    }
    let mut child = command
        .current_dir(&ws)
        .env("TIRITH_LANE", &lane.id)
        .env("TIRITH_PROJECT", &lane.project)
        .env("TIRITH_HOME", &app.home)
        .stdin(Stdio::null())
        .stdout(log.try_clone().map_err(|e| e.to_string())?)
        .stderr(log)
        .process_group(0)
        .spawn()
        .map_err(|e| format!("cannot start {}: {e}", launch.program))?;
    lane.pid = child.id();
    lane.status = "running".into();
    lane.started_at = Some(now_iso());
    lane.ended_at = None;
    lane.last_activity = now_s();
    app.save_lane(&lane);
    app.children.lock().unwrap().insert(lane.id.clone());
    let (app2, id) = (app.clone(), lane.id.clone());
    tokio::spawn(async move {
        let code = child.wait().await.ok().and_then(|s| s.code());
        app2.children.lock().unwrap().remove(&id);
        ended(&app2, &id, code).await;
    });
    Ok(())
}

/// The agent's process is gone: a `done` report goes to the merge queue, anything else is the manager's to see.
async fn ended(app: &Shared, id: &str, code: Option<i32>) {
    let Some(lane) = app.lane(id) else { return };
    if lane.status != "running" {
        return;
    }
    let status = match lane.report.as_ref().map(|r| r.status.as_str()) {
        Some("done") => "merge-queued",
        Some(s) => s,
        None => "no-report",
    }
    .to_string();
    let lane = app
        .update_lane(id, |l| {
            l.status = status.clone();
            l.ended_at = Some(now_iso());
            l.pid = None;
            if l.report.is_none() && l.note.is_empty() {
                l.note = format!("ended without a report (exit {})", code.map(|c| c.to_string()).unwrap_or("?".into()));
            }
        })
        .unwrap();
    if status == "merge-queued" {
        app.merge_wake.notify_one();
    } else {
        let why = lane.report.as_ref().map(|r| r.note.clone()).unwrap_or(lane.note.clone());
        manager::notify(app, &format!("lane {} ({}: {}) ended {status}: {why}", lane.id, lane.project, lane.item));
    }
    app.sched_wake.notify_one();
}

pub fn report(app: &Shared, id: &str, status: &str, note: &str) -> Result<Lane, String> {
    if !matches!(status, "done" | "partial" | "blocked") {
        return Err("status must be done, partial or blocked".into());
    }
    let lane = app.lane(id).ok_or(format!("no lane '{id}'"))?;
    if lane.status != "running" {
        return Err(format!("lane {id} is {}, not running", lane.status));
    }
    app.update_lane(id, |l| l.report = Some(Report { status: status.into(), note: note.into() })).ok_or("lane vanished".into())
}

pub fn stop(app: &Shared, id: &str) -> Result<Lane, String> {
    let lane = app.lane(id).ok_or(format!("no lane '{id}'"))?;
    if !matches!(lane.status.as_str(), "queued" | "running" | "merge-queued") {
        return Err(format!("lane {id} is {}", lane.status));
    }
    if let Some(pid) = lane.pid {
        kill_group(pid, libc::SIGTERM);
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(5)).await;
            kill_group(pid, libc::SIGKILL);
        });
    }
    app.update_lane(id, |l| {
        l.status = "stopped".into();
        l.ended_at = Some(now_iso());
        l.pid = None;
    })
    .ok_or("lane vanished".into())
}

/// The merge failed: the same agent continues on its branch with the reason, at most twice; then it is failed.
pub fn send_back(app: &Shared, lane: &Lane, reason: &str) {
    if lane.attempts >= 2 {
        app.update_lane(&lane.id, |l| {
            l.status = "failed".into();
            l.note = reason.chars().take(2000).collect();
        });
        manager::notify(app, &format!("lane {} failed to merge after {} tries: {reason}", lane.id, lane.attempts + 1));
        return;
    }
    let brief = app.home.join(format!("briefs/{}.md", lane.id));
    let old = std::fs::read_to_string(&brief).unwrap_or_default();
    let base = old.split("\n## Sent back\n").next().unwrap_or("").to_string();
    let _ = std::fs::write(
        &brief,
        format!(
            "{base}\n## Sent back\nTirith could not merge your branch:\n\n{reason}\n\nFix it on your branch \
             (merge main into it if there is a conflict), commit, and report again.\n"
        ),
    );
    app.update_lane(&lane.id, |l| {
        l.attempts += 1;
        l.report = None;
        l.note = format!("sent back: {}", reason.lines().next().unwrap_or(""));
        l.status = "queued".into();
    });
    app.sched_wake.notify_one();
}

/// Removes a merged lane's worktree and branch (never with --force: uncommitted work stays).
pub async fn retire(repo: &Path, lane: &Lane) {
    let _ = git(repo, &["worktree", "remove", &lane.workspace]).await;
    let _ = git(repo, &["branch", "-d", &lane.branch]).await;
}

fn nanos() -> u32 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.subsec_nanos()).unwrap_or(0)
}
