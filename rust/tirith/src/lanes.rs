//! Lanes: one agent, one plan item, one reusable worktree slot (`<repo>-lanes/slot-N`) on its own branch. The
//! autopilot starts the open items of each project's current plan step when the machine has room; a lane whose
//! process ends goes to the merge queue, where the checks (never the agent) decide that its item is done.

use crate::app::{add_worktree, alive, git, kill_group, log_age, now_iso, now_s, tail, Lane, Shared};
use crate::config::Project;
use crate::{brain, guard, manager, models, plan, sandbox};
use std::collections::{BTreeMap, HashMap};
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Mutex;
use std::time::Duration;

/// Messages for working lanes, handed to the agent by its next PostToolUse hook.
static LIVE: Mutex<BTreeMap<String, Vec<String>>> = Mutex::new(BTreeMap::new());

pub fn tell(id: &str, text: &str) {
    LIVE.lock().unwrap().entry(id.into()).or_default().push(text.into());
}

pub fn drain(id: &str) -> Vec<String> {
    LIVE.lock().unwrap().remove(id).unwrap_or_default()
}

fn slug(text: &str) -> String {
    let s: String = text.to_lowercase().chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '-' }).collect();
    let s = s.split('-').filter(|p| !p.is_empty()).collect::<Vec<_>>().join("-");
    s.chars().take(32).collect::<String>().trim_end_matches('-').to_string()
}

pub fn start(app: &Shared, project: &str, step: &str, item: &str, agent: Option<String>) -> Result<Lane, String> {
    let item = plan::clean(item);
    if !plan::is_open(app, project, step, &item)? {
        return Err(format!("\"{item}\" is not an open item of step {step} of {project}. Add it first: tirith plan add {project} {step} \"<text>\""));
    }
    if let Some(l) = app.lanes.lock().unwrap().values().find(|l| l.active() && l.project == project && l.step == step && l.item == item) {
        return Err(format!("lane {} already works on that item ({})", l.id, l.status));
    }
    let lane = Lane {
        id: format!("{}-{:04x}", slug(item.split(" (check:").next().unwrap_or("")), nanos() & 0xffff),
        project: project.into(),
        step: step.into(),
        item,
        agent: agent.unwrap_or_else(|| models::role(app, "workers").agent),
        status: "queued".into(),
        created_at: now_iso(),
        ..Default::default()
    };
    app.save_lane(&lane);
    app.sched_wake.notify_one();
    Ok(lane)
}

pub fn autopilot_on(app: &Shared, project: &str) -> bool {
    app.kv(&format!("autopilot:{project}")).map(|v| v == "on").unwrap_or(app.cfg().autopilot)
}

fn load_per_core() -> f64 {
    let mut l = [0f64; 3];
    let n = unsafe { libc::getloadavg(l.as_mut_ptr(), 3) };
    let cores = std::thread::available_parallelism().map(|c| c.get()).unwrap_or(1) as f64;
    if n > 0 {
        l[0] / cores
    } else {
        0.0
    }
}

fn free_gb(dir: &Path) -> f64 {
    let Ok(c) = std::ffi::CString::new(dir.as_os_str().as_bytes()) else { return f64::MAX };
    let mut s: libc::statvfs = unsafe { std::mem::zeroed() };
    if unsafe { libc::statvfs(c.as_ptr(), &mut s) } != 0 {
        return f64::MAX;
    }
    s.f_bavail as f64 * s.f_frsize as f64 / 1e9
}

/// One cargo build folder per project, shared by its lanes and checks: a folder per worktree costs gigabytes each.
pub fn shared_target(app: &Shared, project: &str) -> PathBuf {
    app.home.join("build").join(project).join("cargo")
}

/// With less than 10 GB free: the build caches git ignores (target, .build, node_modules, build, .gradle) are
/// removed from the worktree folders of `<repo>-lanes` that no active lane holds. Nothing tracked is touched.
async fn free_disk(app: &Shared, lanes: &[Lane]) {
    let cfg = app.cfg();
    for p in &cfg.projects {
        let dir = p.lanes_dir();
        if free_gb(&dir) >= 10.0 {
            return;
        }
        let merging = lanes.iter().any(|l| l.project == p.id && l.status == "merging");
        for slot in std::fs::read_dir(&dir).into_iter().flatten().flatten().map(|e| e.path()) {
            let name = slot.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            let held = lanes.iter().any(|l| l.active() && Path::new(&l.workspace) == slot) || (name == "merge" && merging);
            if held || !(name.starts_with("slot-") || name == "merge") {
                continue;
            }
            for cache in ["target", ".build", "node_modules", "build", ".gradle", "android/build", "android/app/build", "android/.gradle"] {
                let path = slot.join(cache);
                if path.is_dir() && git(&slot, &["check-ignore", "-q", cache]).await.is_ok() {
                    let _ = std::fs::remove_dir_all(&path);
                    app.event("", &p.id, "disk", "diet", &format!("removed {} (low disk)", path.display()), "");
                }
            }
        }
    }
}

/// Why a project cannot start a lane now (merge backlog, disk, commit guard), or `None`.
fn blocked(app: &Shared, p: &Project, lanes: &[Lane]) -> Option<String> {
    let backlog = lanes.iter().filter(|l| l.project == p.id && matches!(l.status.as_str(), "merge-queued" | "merging")).count();
    let dir = if p.lanes_dir().exists() { p.lanes_dir() } else { p.repo() };
    if backlog >= 3 {
        Some(format!("{backlog} lanes wait for the merge queue"))
    } else if free_gb(&dir) < 5.0 {
        Some(format!("less than 5 GB free on {}", dir.display()))
    } else {
        guard::intact(app, p).err()
    }
}

/// Role items of step 0 («Manager de guardia: …», «CEO: …») are attended by the manager or the CEO, not by a lane.
fn is_role_item(text: &str) -> bool {
    let t = text.trim_start().to_lowercase();
    t.starts_with("manager de guardia:") || t.starts_with("ceo:")
}

/// The autopilot's next item: in round-robin over the projects, an open item of the project's current step
/// (the first step with an open item that did not fail or stop) that no lane works on.
fn next_item(app: &Shared, lanes: &[Lane], rr: &mut usize, ok: &mut dyn FnMut(&Project) -> bool) -> Option<Lane> {
    let cfg = app.cfg();
    let n = cfg.projects.len();
    for k in 0..n {
        let p = &cfg.projects[(*rr + k) % n];
        if !autopilot_on(app, &p.id) || app.kv(&format!("main_red:{}", p.id)).is_some() {
            continue;
        }
        let Ok(steps) = plan::read(app, &p.id) else { continue };
        let latest = |step: &str, item: &str| lanes.iter().filter(|l| l.project == p.id && l.step == step && l.item == item).max_by(|a, b| a.created_at.cmp(&b.created_at));
        let skipped = |l: Option<&Lane>| l.is_some_and(|l| matches!(l.status.as_str(), "failed" | "stopped"));
        let Some(st) = steps.iter().find(|s| s.items.iter().any(|i| !i.done && !is_role_item(&i.text) && !skipped(latest(&s.num, &i.text)))) else { continue };
        let Some(it) = st.items.iter().find(|i| !i.done && !is_role_item(&i.text) && latest(&st.num, &i.text).is_none_or(|l| !l.active() && !skipped(Some(l)))) else {
            continue;
        };
        if !ok(p) {
            continue;
        }
        *rr = (*rr + k + 1) % n;
        match start(app, &p.id, &st.num, &it.text, None) {
            Ok(l) => return Some(l),
            Err(e) => app.event("", &p.id, "error", "autopilot", &e, ""),
        }
    }
    None
}

/// Every few seconds (or when woken): stop idle lanes, pick up ended processes, and when the machine has room
/// start the oldest queued lane (manual, sent back or answered) or else the autopilot's next item.
pub async fn scheduler(app: Shared) {
    let mut rr = 0;
    let mut said: HashMap<String, String> = HashMap::new();
    let mut tick: u64 = 0;
    loop {
        let _ = tokio::time::timeout(Duration::from_secs(3), app.sched_wake.notified()).await;
        app.reload_config();
        let cfg = app.cfg();
        let load = load_per_core();
        let lanes: Vec<Lane> = app.lanes.lock().unwrap().values().cloned().collect();
        let running = lanes.iter().filter(|l| l.pid.is_some()).count();
        if tick.is_multiple_of(100) {
            free_disk(&app, &lanes).await; // every ~5 minutes
        }
        tick += 1;
        app.emit("load", serde_json::json!({"load": (load * 100.0).round() / 100.0, "running": running, "max": cfg.max_parallel}));
        for l in lanes.iter().filter(|l| l.pid.is_some()) {
            let tracked = app.children.lock().unwrap().contains(&l.id);
            let log_idle = log_age(&app, &l.id);
            let idle = (now_s().saturating_sub(l.last_activity)).min(log_idle);
            if l.status == "running" && idle > cfg.inactivity_min * 60 {
                kill_group(l.pid.unwrap_or(0), libc::SIGKILL);
                app.event(&l.id, &l.project, "idle", "", &format!("stopped: no activity for {} min", cfg.inactivity_min), "");
            } else if !tracked && !l.pid.is_some_and(alive) {
                ended(&app, &l.id, None).await; // its process ended while the daemon was down
            }
        }
        if running >= cfg.max_parallel || (running > 0 && load >= cfg.max_load) {
            continue;
        }
        let mut ok = |p: &Project| {
            let why = blocked(&app, p, &lanes);
            if let Some(w) = why.as_ref().filter(|w| said.get(&p.id) != Some(*w)) {
                app.event("", &p.id, "paused", "", &format!("no new lane: {w}"), "");
            }
            said.insert(p.id.clone(), why.clone().unwrap_or_default());
            why.is_none()
        };
        let mut queued: Vec<&Lane> = lanes.iter().filter(|l| l.status == "queued").collect();
        queued.sort_by(|a, b| a.created_at.cmp(&b.created_at));
        let pick = queued.into_iter().find(|l| cfg.project(&l.project).is_none_or(&mut ok)).cloned();
        let Some(next) = pick.or_else(|| next_item(&app, &lanes, &mut rr, &mut ok)) else { continue };
        match launch(&app, next.clone()).await {
            Ok(()) => app.sched_wake.notify_one(),
            Err(e) => {
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

/// A free slot of the project checked out on a fresh `lane/<id>` branch at main. A slot is free when no active
/// lane holds it; uncommitted work of a stopped or failed lane is first committed on that lane's branch.
async fn take_slot(app: &Shared, project: &Project, lane: &Lane) -> Result<PathBuf, String> {
    let (repo, dir) = (project.repo(), project.lanes_dir());
    let all: Vec<Lane> = app.lanes.lock().unwrap().values().cloned().collect();
    let mut nums: Vec<u32> = std::fs::read_dir(&dir).into_iter().flatten().flatten().filter_map(|e| e.file_name().to_str()?.strip_prefix("slot-")?.parse().ok()).collect();
    nums.sort();
    let main = git(&repo, &["rev-parse", "HEAD"]).await?;
    let held = |s: &Path| all.iter().any(|l| l.active() && l.id != lane.id && Path::new(&l.workspace) == s);
    let slot = match nums.iter().map(|n| dir.join(format!("slot-{n}"))).find(|s| !held(s)) {
        Some(s) => {
            let prev = all.iter().filter(|l| Path::new(&l.workspace) == s).max_by(|a, b| a.created_at.cmp(&b.created_at));
            if let Some(p) = prev.filter(|p| matches!(p.status.as_str(), "stopped" | "failed")) {
                if !git(&s, &["status", "--porcelain"]).await.unwrap_or_default().is_empty() {
                    let saved = match git(&s, &["add", "-A"]).await {
                        Ok(_) => git(&s, &["commit", "-qm", "Saved by Tirith"]).await,
                        e => e,
                    };
                    if let Err(e) = saved {
                        app.event(&p.id, &p.project, "error", "slot", &format!("could not save the work left in {}: {e}", s.display()), "");
                    }
                }
            }
            s
        }
        None => {
            let s = dir.join(format!("slot-{}", nums.last().map_or(1, |n| n + 1)));
            add_worktree(&repo, &s, None, &app.manifest(&repo).lane_exclude).await?;
            s
        }
    };
    git(&slot, &["checkout", "-q", "-f", "-B", &lane.branch, &main]).await?;
    git(&slot, &["clean", "-fdq"]).await?;
    Ok(slot)
}

fn brief(app: &Shared, lane: &Lane) -> String {
    let steps = plan::read(app, &lane.project).unwrap_or_default();
    let lanes: Vec<Lane> = app.lanes.lock().unwrap().values().filter(|l| l.project == lane.project).cloned().collect();
    let step = steps.iter().find(|s| s.num == lane.step);
    let check = step.and_then(|s| s.items.iter().find(|i| i.text == lane.item)).and_then(|i| i.check.clone());
    let items: Vec<String> = step
        .map(|s| &s.items[..])
        .unwrap_or_default()
        .iter()
        .map(|i| {
            let l = lanes.iter().filter(|l| l.step == lane.step && l.item == i.text).max_by(|a, b| a.created_at.cmp(&b.created_at));
            let who = l.filter(|_| !i.done).map(|l| format!("  ({}: {})", l.id, l.status)).unwrap_or_default();
            format!("- [{}] {}{who}", if i.done { "x" } else { " " }, i.text)
        })
        .collect();
    let others: Vec<String> = lanes.iter().filter(|l| l.id != lane.id && matches!(l.status.as_str(), "running" | "waiting")).map(|l| format!("- {} ({}): {}", l.id, l.status, l.item)).collect();
    let red = app.kv(&format!("main_red:{}", lane.project));
    let rules: Vec<String> = app.cfg().project(&lane.project).map(|p| app.manifest(&p.repo()).rules).unwrap_or_default().iter().map(|r| format!("- {}", r.message)).collect();
    let mut b = format!(
        "# Lane {id}\n\nProject: {project}. Plan step {step}.\n\n## Your item\n{item}\n{check}\n## The step's items\n{items}\n\n\
         ## Lanes working beside you\n{others}\n\nMain is {red}.\n",
        id = lane.id,
        project = lane.project,
        step = lane.step,
        item = lane.item,
        check = check.map(|c| format!("\nIts check (must pass on the merged result): `{c}`\n")).unwrap_or_default(),
        items = items.join("\n"),
        others = if others.is_empty() { "none".into() } else { others.join("\n") },
        red = red.map(|r| format!("red ({r}): not your job unless your item is about it")).unwrap_or("green".into()),
    );
    if let Some(a) = brain::architecture(app, &lane.project) {
        b += &format!("\n## Architecture\n{a}\n");
    }
    b + &format!(
        "\n## How you work\n- You are in your own git worktree ({ws}) on branch {branch}. Change files only here.\n\
         - Commit your work on this branch. When you are finished just end your turn: Tirith runs the checks and merges.\n\
         - If you need a decision you cannot make yourself: tirith ask \"<question>\", then end your turn.\n\
         - Never push, never touch main, never start other agents or sub-agents. The plan is written only by Tirith.\n\
         - Commit checks are never skipped.\n{rules}\n\nBrief written {at} by Tirith.\n",
        ws = lane.workspace,
        branch = lane.branch,
        rules = rules.join("\n"),
        at = now_iso(),
    )
}

/// Runs the lane's agent in its slot (a new slot for a lane without one): the brief, or a resume of its session
/// with the pending message.
async fn launch(app: &Shared, mut lane: Lane) -> Result<(), String> {
    let cfg = app.cfg();
    let project = cfg.project(&lane.project).ok_or("unknown project")?.clone();
    if lane.workspace.is_empty() || !Path::new(&lane.workspace).exists() {
        lane.branch = format!("lane/{}", lane.id);
        lane.workspace = take_slot(app, &project, &lane).await?.display().to_string();
        app.save_lane(&lane);
    }
    let ws = PathBuf::from(&lane.workspace);
    let msg = std::mem::take(&mut lane.resume);
    let brief_path = app.home.join(format!("briefs/{}.md", lane.id));
    let resume = (!lane.session.is_empty() && !msg.is_empty()).then_some((lane.session.as_str(), msg.as_str()));
    if resume.is_none() {
        let extra = if msg.is_empty() { String::new() } else { format!("\n## Message from Tirith\n{msg}\n") };
        std::fs::write(&brief_path, brief(app, &lane) + &extra).map_err(|e| e.to_string())?;
    }
    let role = models::role(app, "workers");
    let over = (lane.agent != role.agent || lane.model.is_some()).then_some(lane.agent.as_str());
    let cmd = models::lane_argv(app, over, lane.model.as_deref(), &brief_path, &ws, resume)?;
    let used = match lane.model.clone().or(if over.is_none() { role.model } else { None }) {
        Some(m) => format!("{}:{m}", cmd.alias),
        None => cmd.alias.clone(),
    };
    if !lane.tried.contains(&used) {
        lane.tried.push(used); // the fallback chain skips what this lane already used
    }
    let launch = sandbox::wrap(app, &lane.id, cmd.argv);
    let log = std::fs::OpenOptions::new().create(true).append(true).open(app.home.join(format!("logs/lanes/{}.log", lane.id))).map_err(|e| e.to_string())?;
    let mut command = launch.command(app);
    let mut child = command
        .current_dir(&ws)
        .env("TIRITH_LANE", &lane.id)
        .env("TIRITH_PROJECT", &lane.project)
        .env("CARGO_TARGET_DIR", shared_target(app, &lane.project))
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

/// The agent's process is gone: a working lane goes to the merge queue (or, out of credits, to the next agent
/// of the fallback chain); a waiting lane waits for its answer.
async fn ended(app: &Shared, id: &str, code: Option<i32>) {
    let Some(lane) = app.lane(id) else { return };
    // Only what the agent program itself said counts: its plain-text output and its final result event, never the
    // files and command output it read (a lane working on Tirith reads the very patterns that detect it).
    let log: String = tail(&app.home.join(format!("logs/lanes/{id}.log")), 200).lines().filter(|l| !l.trim_start().starts_with('{') || (l.contains("\"type\":\"result\"") && l.contains("\"is_error\":true"))).collect::<Vec<_>>().join("\n");
    if lane.status == "running" && models::out_of_credits(&log) {
        let next = models::fallback(app, "workers", &lane.tried);
        app.update_lane(id, |l| {
            l.pid = None;
            l.ended_at = Some(now_iso());
            if let Some((alias, model)) = &next {
                l.agent = alias.clone();
                l.model = model.clone();
                l.session.clear();
                l.status = "queued".into();
                l.note = format!("out of credits: continues with {alias}");
                l.resume = "The previous agent ran out of credits. Continue the work already committed on your branch.".into();
            } else {
                l.status = "failed".into();
                l.note = format!("out of credits, no agent left to try (tried {})", l.tried.join(", "));
            }
        });
        if next.is_none() {
            manager::notify(app, &format!("lane {id} ({}: {}) failed: out of credits on every agent of the workers role", lane.project, lane.item));
        }
    } else {
        app.update_lane(id, |l| {
            l.pid = None;
            match l.status.as_str() {
                "running" => {
                    l.status = "merge-queued".into();
                    l.ended_at = Some(now_iso());
                    if l.note.is_empty() {
                        l.note = format!("ended (exit {})", code.map(|c| c.to_string()).unwrap_or("?".into()));
                    }
                }
                "waiting" if !l.resume.is_empty() => l.status = "queued".into(), // answered while it ended its turn
                _ => {}
            }
        });
        app.merge_wake.notify_one();
    }
    app.sched_wake.notify_one();
}

/// `tirith ask` from a lane: it waits, and the manager is told how to answer.
pub fn ask(app: &Shared, id: &str, question: &str) -> Result<Lane, String> {
    let lane = app.lane(id).ok_or(format!("no lane '{id}'"))?;
    if lane.status != "running" || question.trim().is_empty() {
        return Err(format!("lane {id} is {}: only a working lane asks, with a question", lane.status));
    }
    let lane = app.update_lane(id, |l| {
        l.status = "waiting".into();
        l.question = question.trim().into();
        l.resume.clear();
    });
    manager::notify(app, &format!("lane {id} ({}: {}) asks: {}\nAnswer with: tirith answer {id} \"<answer>\"", lane.as_ref().map_or("", |l| &l.project), lane.as_ref().map_or("", |l| &l.item), question.trim()));
    lane.ok_or("lane vanished".into())
}

/// `tirith answer` from the manager: a live message when the agent still runs, else a resumed run.
pub fn answer(app: &Shared, id: &str, text: &str) -> Result<Lane, String> {
    let lane = app.lane(id).ok_or(format!("no lane '{id}'"))?;
    if lane.status != "waiting" {
        return Err(format!("lane {id} is {}, not waiting for an answer", lane.status));
    }
    let live = lane.pid.is_some_and(alive);
    if live {
        tell(id, &format!("Answer from the manager: {text}"));
    }
    let lane = app.update_lane(id, |l| {
        l.resume = if l.session.is_empty() { format!("You asked the manager: {}\nAnswer from the manager: {text}", l.question) } else { format!("Answer from the manager: {text}") };
        if !live {
            l.status = "queued".into();
        }
    });
    app.sched_wake.notify_one();
    lane.ok_or("lane vanished".into())
}

pub fn stop(app: &Shared, id: &str) -> Result<Lane, String> {
    let lane = app.lane(id).ok_or(format!("no lane '{id}'"))?;
    if !matches!(lane.status.as_str(), "queued" | "running" | "waiting" | "merge-queued") {
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

/// The merge failed because of the lane: the same agent continues in its slot with the reason, twice at most.
pub fn send_back(app: &Shared, lane: &Lane, reason: &str) {
    if lane.attempts >= 2 {
        app.update_lane(&lane.id, |l| {
            l.status = "failed".into();
            l.note = reason.chars().take(2000).collect();
        });
        manager::notify(app, &format!("lane {} ({}: {}) failed after {} tries: {}", lane.id, lane.project, lane.item, lane.attempts + 1, reason.lines().next().unwrap_or("")));
        return;
    }
    app.update_lane(&lane.id, |l| {
        l.attempts += 1;
        l.solo = false;
        l.resume = format!("Tirith sent your work back:\n\n{reason}\n\nFix it on your branch, commit, and end your turn.");
        l.note = format!("sent back: {}", reason.lines().next().unwrap_or(""));
        l.status = "queued".into();
    });
    app.sched_wake.notify_one();
}

fn nanos() -> u32 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.subsec_nanos()).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    #[test]
    fn autopilot_skips_role_items() {
        assert!(super::is_role_item("Manager de guardia: atender el chat"));
        assert!(super::is_role_item("CEO: revisar el plan"));
        assert!(!super::is_role_item("Arreglar el CEO: bug"));
    }
}
