//! Lanes: one agent, one plan item, one worktree slot of its own on its own branch (removed the moment the lane ends for good); the autopilot starts open plan items, and a lane that ends goes to the merge queue, where the checks decide.

use crate::app::{add_worktree, alive, git, kill_group, now_iso, now_s, tail, Lane, Shared};
use crate::{config::Project, guard, manager, models, plan, sandbox};
use regex::Regex;
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::{
    os::unix::ffi::OsStrExt,
    process::Stdio,
    sync::{LazyLock, Mutex},
    time::Duration,
};
static LIVE: Mutex<BTreeMap<String, Vec<String>>> = Mutex::new(BTreeMap::new());
pub fn tell(id: &str, text: &str) {
    LIVE.lock().unwrap().entry(id.into()).or_default().push(text.into());
}
pub fn drain(id: &str) -> Vec<String> {
    LIVE.lock().unwrap().remove(id).unwrap_or_default()
}
pub(crate) fn slug(text: &str) -> String {
    let s: String = text.to_lowercase().chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '-' }).collect();
    let s = s.split('-').filter(|p| !p.is_empty()).collect::<Vec<_>>().join("-");
    s.chars().take(32).collect::<String>().trim_end_matches('-').to_string()
}
pub fn start(app: &Shared, project: &str, step: &str, item: &str, agent: Option<String>) -> Result<Lane, String> {
    let item = plan::clean(item);
    let steps = plan::read(app, project)?;
    let st = steps.iter().find(|s| s.num == step).ok_or(format!("the plan of {project} has no step {step}"))?;
    st.items.iter().any(|i| !i.done && plan::same(&i.text, &item)).then_some(()).ok_or_else(|| format!("\"{item}\" is not an open item of step {step} of {project}. Add it first: tirith plan add {project} {step} \"<text>\""))?;
    app.lanes.lock().unwrap().values().find(|l| l.active() && l.project == project && l.step == step && plan::same(&l.item, &item)).map_or(Ok(()), |l| Err(format!("lane {} already works on that item ({})", l.id, l.status)))?;
    let (factory, role_hash) = app.stamp_for(&models::lane_role(&app.cfg()));
    let lane = Lane {
        id: format!("{}-{:04x}", slug(item.split(" (check:").next().unwrap_or("")), chrono::Utc::now().timestamp_subsec_nanos() & 0xffff),
        project: project.into(),
        step: step.into(),
        item,
        agent: agent.unwrap_or_else(|| models::role(app, &models::lane_role(&app.cfg())).agent),
        factory,
        role_hash,
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
pub(crate) fn free_gb(dir: &Path) -> f64 {
    let Ok(c) = std::ffi::CString::new(dir.as_os_str().as_bytes()) else { return f64::MAX };
    let mut s: libc::statvfs = unsafe { std::mem::zeroed() };
    let Some(()) = (unsafe { libc::statvfs(c.as_ptr(), &mut s) } == 0).then_some(()) else { return f64::MAX };
    s.f_bavail as f64 * s.f_frsize as f64 / 1e9
}
/// One cargo build folder per project, shared by its lanes and checks: a folder per worktree costs gigabytes each.
pub fn shared_target(app: &Shared, project: &str) -> PathBuf {
    app.home.join("build").join(project).join("cargo")
}
/// Folders git ignores that are only build caches: they go with the worktree and are never moved to the project.
fn is_cache(rel: &Path) -> bool {
    rel.components().any(|c| ["target", "node_modules", ".build", "build", ".gradle", "__pycache__", "venv", ".venv"].contains(&c.as_os_str().to_str().unwrap_or("")))
}
fn at_s(iso: Option<&str>) -> u64 {
    iso.and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok()).map_or(0, |t| t.timestamp().max(0) as u64)
}
/// Moves one git-ignored file of a lane to the same path in the project, never over what is there: an identical file there needs no move, a different one keeps its place and the lane's lands next to it with the lane's id in its name (`doc-<lane>.pdf`). The path it ends at.
fn move_ignored(src: &Path, repo: &Path, rel: &Path, lane: &str) -> Option<PathBuf> {
    let side = |d: &Path| {
        let n = d.file_name().unwrap_or_default().to_string_lossy();
        d.with_file_name(n.rfind('.').filter(|&i| i > 0 && i + 1 < n.len()).map_or_else(|| format!("{n}-{lane}"), |i| format!("{}-{lane}.{}", &n[..i], &n[i + 1..])))
    };
    let dest = std::iter::successors(Some(repo.join(rel)), |d| Some(side(d))).find(|d| !d.exists() || std::fs::read(d).ok() == std::fs::read(src).ok())?;
    let Some(()) = (!dest.exists()).then_some(()) else { return Some(dest) };
    (dest.parent().map(std::fs::create_dir_all), std::fs::rename(src, &dest).or_else(|_| std::fs::copy(src, &dest).and_then(|_| std::fs::remove_file(src))).ok()).1.map(|_| dest)
}
fn files_under(p: &Path, out: &mut Vec<PathBuf>) {
    std::fs::read_dir(p).into_iter().flatten().flatten().map(|e| e.path()).for_each(|e| ((e.is_symlink() || e.is_file()).then(|| out.push(e.clone())), (!e.is_symlink() && e.is_dir()).then(|| files_under(&e, out))).0.unwrap_or(()));
}
/// Keeps what a lane moved in out of git for good: when the project does not already ignore it, one line for the whole ignored folder moved (or for a lone file) joins `.git/info/exclude` there, with the pattern characters escaped (`\` in a name would escape the next character and the file would stay visible), so it is never committed or pushed from the project folder.
async fn ensure_excluded(repo: &Path, rel: &Path) {
    let dest = repo.join(rel);
    let (dir, rel, esc) = (dest.is_dir() && !dest.is_symlink(), rel.to_string_lossy().into_owned(), rel.to_string_lossy().replace('\\', "\\\\").replace('*', "\\*").replace('?', "\\?").replace('[', "\\[").replace(']', "\\]").replace(' ', "\\ "));
    let Some(()) = git(repo, &["check-ignore", "-q", "--", &rel]).await.is_err().then_some(()) else { return };
    let Ok(gitdir) = git(repo, &["rev-parse", "--git-common-dir"]).await else { return };
    let (file, line) = (repo.join(gitdir).join("info").join("exclude"), format!("/{esc}{}", if dir { "/" } else { "" }));
    let text = std::fs::read_to_string(&file).unwrap_or_default();
    let _ = (!text.lines().any(|l| l.trim() == line)).then(|| (file.parent().map(std::fs::create_dir_all), std::fs::write(&file, format!("{text}{}{line}\n", if text.is_empty() || text.ends_with('\n') { "" } else { "\n" }))));
}
/// Takes one worktree away in full (`git worktree remove`, then prune; a folder git will not remove is deleted): no folder and no registration left.
pub(crate) async fn remove_worktree(repo: &Path, ws: &Path) {
    drop(((ws.exists() && git(repo, &["worktree", "remove", "--force", &ws.to_string_lossy()]).await.is_err()).then(|| std::fs::remove_dir_all(ws)), git(repo, &["worktree", "prune"]).await));
}
/// A finished lane's paper: its log, the logs of its checks and stages, and its brief.
fn drop_lane_logs(app: &Shared, id: &str) {
    let _ = (std::fs::remove_file(app.home.join(format!("logs/lanes/{id}.log"))), std::fs::remove_file(app.home.join(format!("briefs/{id}.md"))));
    app.query("SELECT log FROM runs WHERE COALESCE(log, '') <> '' AND ',' || lane || ',' LIKE '%,' || ?1 || ',%'", [id], |r| r.get::<_, String>(0)).iter().for_each(|log| drop(std::fs::remove_file(log)));
}
/// A lane that merged, stopped or failed for good leaves nothing behind: first its uncommitted work is committed on its branch (a merged lane's work is on main already, and its branch goes below), the git-ignored files of its worktree that are no build cache move to the same path in the project (kept ignored there, never overwriting), then the worktree goes with its caches (`git worktree remove` and prune). A merged lane's branch, logs and brief go at once; a stopped or failed one keeps its branch, and its logs go after 24 h (the sweep).
pub async fn cleanup_lane(app: &Shared, lane: &Lane, merged: bool) {
    let Some(project) = app.cfg().project(&lane.project).cloned().filter(|_| !lane.pid.is_some_and(alive)) else { return };
    let (repo, ws) = (project.repo(), PathBuf::from(&lane.workspace));
    if !lane.workspace.is_empty() && ws.is_dir() && !app.lanes.lock().unwrap().values().any(|l| l.active() && l.id != lane.id && l.workspace == lane.workspace) {
        let dirty = !merged && !git(&ws, &["status", "--porcelain"]).await.unwrap_or_default().is_empty();
        if dirty {
            drop(git(&ws, &["add", "-A"]).await);
            drop(git(&ws, &["commit", "-qm", "Saved by Tirith"]).await.map_err(|e| app.event(&lane.id, &lane.project, "error", "cleanup", &format!("could not save the work left in {}: {e}", ws.display()), "")));
        }
        let (listed, mut targets) = (git(&ws, &["ls-files", "--others", "--ignored", "--exclude-standard", "--directory", "-z"]).await.unwrap_or_default(), Vec::<PathBuf>::new());
        for top in &listed.split('\0').filter(|r| !r.is_empty() && !is_cache(Path::new(r.trim_end_matches('/')))).map(|r| PathBuf::from(r.trim_end_matches('/'))).collect::<Vec<_>>() {
            let (src, is_dir, mut files) = (ws.join(top), ws.join(top).is_dir() && !ws.join(top).is_symlink(), Vec::<PathBuf>::new());
            ((src.is_symlink() || src.is_file()).then(|| files.push(src.clone())), is_dir.then(|| files_under(&src, &mut files))).0.unwrap_or(());
            let moved: Vec<PathBuf> = files.iter().filter_map(|p| p.strip_prefix(&ws).ok().filter(|rel| !is_cache(rel)).and_then(|rel| move_ignored(p, &repo, rel, &lane.id))).collect();
            targets.extend(if is_dir { (!moved.is_empty()).then(|| top.clone()).into_iter().collect::<Vec<_>>() } else { moved.iter().filter_map(|d| d.strip_prefix(&repo).ok().map(Path::to_path_buf)).collect() });
        }
        for rel in &targets {
            ensure_excluded(&repo, rel).await;
        }
        remove_worktree(&repo, &ws).await;
    }
    if merged && !lane.branch.is_empty() {
        drop(git(&repo, &["branch", "-D", &lane.branch]).await);
    }
    drop((merged.then(|| drop_lane_logs(app, &lane.id)), app.update_lane(&lane.id, |l| l.workspace = if l.workspace == lane.workspace { String::new() } else { l.workspace.clone() })));
}
/// The cleanup of a lane that just ended for good, off the caller's path: after `wait_s` (a stopped lane's agent is gone first).
pub fn cleanup_later(app: &Shared, lane: Lane, merged: bool, wait_s: u64) {
    let app = app.clone();
    tokio::spawn(async move { (tokio::time::sleep(Duration::from_secs(wait_s)).await, cleanup_lane(&app, &lane, merged).await).1 });
}
/// An idle project's build cache and merge folder go together once it has been `minutes` without work; busy, or not that long yet, keeps both.
async fn drop_idle_cache(repo: &Path, build: &Path, merge_dir: &Path, busy: bool, last: u64, now: u64, minutes: u64) -> bool {
    let Some(()) = (!busy && now.saturating_sub(last) >= minutes * 60).then_some(()) else { return false };
    (remove_worktree(repo, merge_dir).await, (!build.exists() || std::fs::remove_dir_all(build).is_ok()) && !merge_dir.exists()).1
}
/// The last time a project was seen working: the newest of its lanes' and stage runs' stamps, and its build cache's own age.
fn last_activity(app: &Shared, project: &Project, lanes: &[Lane], build: &Path) -> u64 {
    let run_at = app.query("SELECT MAX(COALESCE(ended_at, started_at)) FROM runs WHERE project = ?1", [&project.id], |r| r.get::<_, String>(0)).first().map(|s| at_s(Some(s))).unwrap_or(0);
    let dir_at = std::fs::metadata(build).and_then(|m| m.modified()).map(|t| t.duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)).unwrap_or(0);
    lanes.iter().filter(|l| l.project == project.id).map(|l| at_s(l.ended_at.as_deref()).max(at_s(l.started_at.as_deref())).max(l.last_activity)).max().unwrap_or(0).max(run_at).max(dir_at)
}
/// The pass at startup and then every few minutes, applying the same rules to everything that already meets them: a merged, stopped or failed lane still holding a worktree loses it now, the logs and brief of one stopped or failed more than 24 h ago go, what earlier cleanups moved into a project but git still sees as new (a `\`, space or `[` in its name broke its per-file exclude line) gets its folder excluded, and a project's build cache and merge folder go after `limits.build_idle_minutes` with no lane, stage or manager of it working.
async fn sweep_cleanups(app: &Shared, lanes: &[Lane]) {
    for l in lanes.iter().filter(|l| matches!(l.status.as_str(), "merged" | "stopped" | "failed")) {
        let (log, brief, ended) = (app.home.join(format!("logs/lanes/{}.log", l.id)), app.home.join(format!("briefs/{}.md", l.id)), at_s(l.ended_at.as_deref()));
        let (merged, held_dir) = (l.status == "merged", !l.workspace.is_empty() && Path::new(&l.workspace).is_dir());
        if (merged && (held_dir || log.exists() || brief.exists())) || (!merged && held_dir && now_s().saturating_sub(ended) > 60) {
            cleanup_lane(app, l, merged).await;
        }
        let _ = (l.status != "merged" && (log.exists() || brief.exists()) && ended > 0 && now_s().saturating_sub(ended) >= 24 * 3600).then(|| drop_lane_logs(app, &l.id));
    }
    let (now, minutes, talking, pipelines) = (now_s(), app.cfg().factory.limits.build_idle_minutes, manager::busy(), app.query("SELECT project FROM pipelines WHERE status = 'running'", [], |r| r.get::<_, String>(0)));
    for p in &app.cfg().projects {
        let (repo, Ok(gitdir)) = (p.repo(), git(&p.repo(), &["rev-parse", "--git-common-dir"]).await) else { continue };
        let moved_lines: Vec<String> = std::fs::read_to_string(repo.join(gitdir).join("info/exclude")).unwrap_or_default().lines().filter_map(|l| l.strip_prefix('/').map(|r| r.trim_end_matches('/').to_string())).collect();
        let fresh = git(&repo, &["ls-files", "--others", "--exclude-standard", "-z"]).await.unwrap_or_default();
        for rel in fresh.split('\0').filter(|r| !r.is_empty() && moved_lines.iter().any(|m| m == r || r.starts_with(&format!("{m}/")))) {
            ensure_excluded(&repo, Path::new(rel).parent().filter(|q| !q.as_os_str().is_empty()).unwrap_or(Path::new(rel))).await;
        }
        let (build, merge_dir) = (app.home.join("build").join(&p.id), p.lanes_dir().join("merge"));
        let Some(()) = (build.exists() || merge_dir.exists()).then_some(()) else { continue };
        let busy = talking.contains(&p.id) || pipelines.contains(&p.id) || lanes.iter().any(|l| l.project == p.id && (l.pid.is_some() || matches!(l.status.as_str(), "running" | "merging")));
        if drop_idle_cache(&repo, &build, &merge_dir, busy, last_activity(app, p, lanes, &build), now, minutes).await {
            app.event("", &p.id, "disk", "diet", &format!("removed {} and its merge folder ({minutes} min with no lane, stage or manager working)", build.display()), "");
        }
    }
}
/// Below the factory's `limits.low_disk_gb` free, before the autopilot pauses: the shared build caches (`<home>/build/<project>`) of projects with no lane or stage under way are removed; the next build makes them again.
fn clear_idle_builds(root: &Path, busy: &[String]) -> Vec<PathBuf> {
    std::fs::read_dir(root).into_iter().flatten().flatten().map(|e| e.path()).filter(|d| d.is_dir() && !busy.iter().any(|b| d.file_name() == Some(b.as_ref()))).filter(|d| std::fs::remove_dir_all(d).is_ok()).collect()
}
async fn free_idle_builds(app: &Shared, lanes: &[Lane]) {
    let mut busy: Vec<String> = lanes.iter().filter(|l| l.pid.is_some() || matches!(l.status.as_str(), "running" | "waiting" | "merging")).map(|l| l.project.clone()).collect();
    busy.extend(app.query("SELECT project FROM runs WHERE status = 'running'", [], |r| r.get::<_, String>(0)));
    let root = app.home.join("build");
    let gone = tokio::task::spawn_blocking(move || clear_idle_builds(&root, &busy)).await.unwrap_or_default();
    gone.iter().for_each(|d| _ = app.event("", &d.file_name().unwrap_or_default().to_string_lossy(), "disk", "diet", &format!("removed {} (less than 5 GB free; no lane or stage of its project under way)", d.display()), ""));
}
/// Ask once per low-disk spell: `asked` is whether a card is already out; it re-arms after the disk recovers (2 GB above the factory's limit).
fn low_disk_ask(free: f64, asked: bool) -> Option<bool> {
    let low = crate::factory::current().limits.low_disk_gb;
    (!asked && free < low).then_some(true).or((asked && free >= low + 2.0).then_some(false))
}
fn biggest_dirs(dir: &Path, n: usize) -> Vec<(String, u64)> {
    let text = std::process::Command::new("du").arg("-sk").args(std::fs::read_dir(dir).into_iter().flatten().flatten().map(|e| e.path())).output().map(|o| String::from_utf8_lossy(&o.stdout).into_owned()).unwrap_or_default();
    let mut rows: Vec<(String, u64)> = text.lines().filter_map(|l| l.split_once('\t')).filter_map(|(k, p)| Some((p.to_string(), k.trim().parse().ok()?))).collect();
    rows.sort_by_key(|a| std::cmp::Reverse(a.1));
    rows.into_iter().take(n).collect()
}
/// Below the factory's low-disk limit on the disk of the lanes' worktrees: one ask-owner card (kind device, of no project, since every project shares that disk) naming the biggest of Tirith's own folders.
async fn warn_low_disk(app: &Shared) {
    let (home, key, free) = (app.home.clone(), "low_disk_asked", free_gb(&app.home.join("lanes")));
    match low_disk_ask(free, app.kv(key).is_some()) {
        Some(true) => {
            app.set_kv(key, Some("1"));
            let rows = tokio::task::spawn_blocking(move || biggest_dirs(&home, 5)).await.unwrap_or_default();
            let list = rows.iter().map(|(p, k)| format!("{p} ({:.1} GB)", *k as f64 / 1e6)).collect::<Vec<_>>().join(", ");
            let q = format!("Only {free:.1} GB free on this disk: lanes will fail with No space left on device. The biggest of Tirith's own folders: {list}. Free some space (delete what you no longer need).");
            let _ = crate::manager::ask_owner(app, "", "device", &q, "");
        }
        Some(false) => app.set_kv(key, None),
        None => {}
    }
}
/// Why a project cannot start a lane now (merge backlog, disk, commit guard), or `None`.
fn blocked(app: &Shared, p: &Project, lanes: &[Lane]) -> Option<String> {
    let (lim, backlog) = (&app.cfg().factory.limits, lanes.iter().filter(|l| l.project == p.id && matches!(l.status.as_str(), "merge-queued" | "merging")).count());
    (backlog >= lim.merge_backlog)
        .then(|| format!("{backlog} lanes wait for the merge queue"))
        .or_else(|| (free_gb(&app.home.join("lanes")) < lim.low_disk_gb).then(|| format!("less than {} GB free on the disk of the lanes' worktrees", lim.low_disk_gb)))
        .or_else(|| guard::intact(app, p).err())
}
pub(crate) fn paused_fix(project: &str, why: &str) -> String {
    let said = why.find('(').zip(why.rfind(')')).map(|(a, b)| &why[a + 1..b]).filter(|f| f.starts_with("tirith ")).map(String::from);
    let known = [("merge queue", "wait for the merges (tirith status shows them)"), ("GB free", "free disk space")].into_iter().find(|(w, _)| why.contains(w)).map(|(_, f)| f.to_string());
    said.or(known).unwrap_or_else(|| if why.contains("core.hooksPath") || why.contains("not Tirith's") { "fix the git hooks by hand, then tirith guard install ".to_string() + project } else { format!("tirith guard install {project}") })
}
/// Role items (the factory's `[plan] role_prefixes`) are attended by a role, and items marked as waiting for the owner (`[plan] owner_wait`) wait: the autopilot starts no lane for either.
pub(crate) fn is_autopilot_skipped(text: &str) -> bool {
    let t = text.trim_start().to_lowercase();
    let f = crate::factory::current();
    f.words("plan", "role_prefixes").iter().any(|p| t.starts_with(&p.to_lowercase())) || f.words("plan", "owner_wait").iter().any(|w| t.contains(&w.to_lowercase()))
}
/// Items that wait for another open item (the factory's `[plan] waits_for` words) by quoting it or sharing most of its words.
fn waits_for_open_item(text: &str, open: &[&str]) -> bool {
    let t = text.to_lowercase();
    let words = |s: &str| -> Vec<String> { s.to_lowercase().split(|c: char| !c.is_alphanumeric()).filter(|w| w.chars().count() >= 5).map(String::from).collect() };
    crate::factory::current().words("plan", "waits_for").iter().any(|m| t.contains(&m.to_lowercase()))
        && open.iter().any(|o| {
            let (ol, ow) = (o.trim().to_lowercase(), words(o));
            ol != t.trim() && !ol.is_empty() && (t.contains(&ol) || (ow.len() >= 3 && ow.iter().filter(|w| t.contains(w.as_str())).count() * 10 >= ow.len() * 6))
        })
}
/// The autopilot's next item: in round-robin over the projects, an open item of the project's current step (the first step with an open item that did not fail or stop) that no lane works on.
fn next_item(app: &Shared, lanes: &[Lane], rr: &mut usize, ok: &mut dyn FnMut(&Project) -> bool) -> Option<Lane> {
    let cfg = app.cfg();
    let n = cfg.projects.len();
    for k in 0..n {
        let p = &cfg.projects[(*rr + k) % n];
        let Some(()) = (autopilot_on(app, &p.id) && app.kv(&format!("main_red:{}", p.id)).is_none()).then_some(()) else { continue };
        let Ok(steps) = plan::read(app, &p.id) else { continue };
        let open: Vec<&str> = steps.iter().flat_map(|s| s.items.iter()).filter(|i| !i.done).map(|i| i.text.as_str()).collect();
        let latest = |step: &str, item: &str| lanes.iter().filter(|l| l.project == p.id && l.step == step && plan::same(&l.item, item)).max_by(|a, b| a.created_at.cmp(&b.created_at));
        let skipped = |l: Option<&Lane>| l.is_some_and(|l| matches!(l.status.as_str(), "failed" | "stopped"));
        let startable = |i: &plan::Item| !i.done && !is_autopilot_skipped(&i.text) && !waits_for_open_item(&i.text, &open);
        let Some(st) = steps.iter().find(|s| s.items.iter().any(|i| startable(i) && !skipped(latest(&s.num, &i.text)))) else { continue };
        let Some(it) = st.items.iter().find(|i| startable(i) && latest(&st.num, &i.text).is_none_or(|l| !l.active() && !skipped(Some(l)))) else { continue };
        let Some(()) = ok(p).then_some(()) else { continue };
        *rr = (*rr + k + 1) % n;
        if let Ok(l) = start(app, &p.id, &st.num, &it.text, None).map_err(|e| app.event("", &p.id, "error", "autopilot", &e, "")) {
            return Some(l);
        }
    }
    None
}
/// Seconds since the agent's log last got a line other than an `api_retry` one (those keep the file fresh while the agent is stuck). `seen` remembers per lane the last real line and when it appeared.
fn quiet_s(seen: &mut HashMap<String, (String, u64)>, id: &str, tail: &str, log_age: u64, now: u64) -> u64 {
    let last = tail.lines().rev().find(|l| !l.contains("\"api_retry\"")).unwrap_or("");
    let e = seen.entry(id.to_string()).or_insert((last.to_string(), now.saturating_sub(log_age)));
    let _ = (e.0 != last).then(|| *e = (last.to_string(), now));
    now.saturating_sub(e.1)
}
/// Written into a lane's log at every launch: the log is appended across attempts, so `run_completed` only counts a terminal after the last launch line (or, for a run started before these lines existed, after its last `run.lifecycle.started`).
const RUN_START: &str = "--- tirith: lane run started ";
/// Whether the lane's current run already ended in its log: a `run.terminal.completed` event (the top-level `payload_type` of its own stream line, so a mention inside a file or command output the agent read never counts). The process may still be alive.
fn run_completed(tail: &str) -> bool {
    tail.lines().fold(false, |done, line| match serde_json::from_str::<serde_json::Value>(line).ok().and_then(|v| v["payload_type"].as_str().map(String::from)).as_deref() {
        Some("run.lifecycle.started") => false,
        Some("run.terminal.completed") => true,
        _ if line.starts_with(RUN_START) => false,
        _ => done,
    })
}
/// Whether a running lane has gone `min` minutes since it started and was not reported yet (`min` 0: never).
fn long_running_due(started_at: Option<&str>, now: u64, min: u64, reported: bool) -> bool {
    let start = started_at.and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok()).map_or(now, |t| t.timestamp().max(0) as u64);
    min > 0 && !reported && now.saturating_sub(start) >= min * 60
}
/// The last `n` shell commands the agent ran, from the tail of its stream-json log (oldest first).
fn last_commands(tail: &str, n: usize) -> Vec<String> {
    let mut cmds: Vec<String> = tail
        .lines()
        .filter_map(|l| serde_json::from_str::<serde_json::Value>(l).ok())
        .flat_map(|v| v["message"]["content"].as_array().cloned().unwrap_or_default())
        .filter_map(|c| c["input"]["command"].as_str().map(|s| s.chars().take(200).collect::<String>()))
        .collect();
    cmds.drain(..cmds.len().saturating_sub(n));
    cmds
}
/// Every few seconds (or when woken): stop idle lanes, pick up ended processes, and when the machine has room start the oldest queued lane (manual, sent back or answered) or else the autopilot's next item.
pub async fn scheduler(app: Shared) {
    let mut rr = 0;
    let mut said: HashMap<String, String> = HashMap::new();
    let mut said_error: Option<String> = None;
    let mut tick: u64 = 0;
    let mut seen: HashMap<String, (String, u64)> = HashMap::new();
    loop {
        let _ = tokio::time::timeout(Duration::from_secs(3), app.sched_wake.notified()).await;
        // config.toml changed by hand: the page gets the projects again
        let _ = app.reload_config(false).then(|| app.emit("projects", serde_json::json!(crate::projects::list(&app))));
        let cfg = app.cfg();
        if cfg.error != said_error {
            said_error.clone_from(&cfg.error);
            said_error.iter().for_each(|e| manager::emit_event(&app, "", "factory.invalid", &format!("Configuration: {e}"), None));
        }
        let lim = &cfg.factory.limits;
        let mut avg = [0f64; 3];
        unsafe { libc::getloadavg(avg.as_mut_ptr(), 3) }; // leaves zeros when it fails
        let load = avg[0] / std::thread::available_parallelism().map_or(1, |c| c.get()) as f64;
        let lanes: Vec<Lane> = app.lanes.lock().unwrap().values().cloned().collect();
        let running = lanes.iter().filter(|l| l.pid.is_some()).count();
        if free_gb(&app.home.join("lanes")) < lim.low_disk_gb {
            free_idle_builds(&app, &lanes).await; // first, so neither the owner's card nor the pause comes when this is enough
        }
        if tick.is_multiple_of(100) {
            warn_low_disk(&app).await;
            sweep_cleanups(&app, &lanes).await; // every ~5 minutes; the first pass runs at startup, applying the rules to everything that already meets them
        }
        let _ = tick.is_multiple_of(10).then(|| (manager::close_stale_questions(&app), crate::bots::tick(&app))); // every ~30 s
        tick += 1;
        app.emit("load", serde_json::json!({"load": (load * 100.0).round() / 100.0, "running": running, "max": lim.max_parallel}));
        for l in lanes.iter().filter(|l| l.pid.is_some()) {
            let text = tail(&app.home.join(format!("logs/lanes/{}.log", l.id)), 200);
            let _ = (l.agent == "codex").then(|| codex_session(&app, &l.id, &text));
            let log_age = std::fs::metadata(app.home.join(format!("logs/lanes/{}.log", l.id))).and_then(|m| m.modified()).map(|t| t.elapsed().map(|d| d.as_secs()).unwrap_or(0)).unwrap_or(u64::MAX); // seconds since its log last changed
            let quiet = quiet_s(&mut seen, &l.id, &text, log_age, now_s());
            if l.status == "running" && run_completed(&text) {
                // Its run is over (it committed and said so) but its process lingers: end it like a process exit, so it goes to the merge queue instead of sitting `running` until the idle stop calls it stuck.
                app.event(&l.id, &l.project, "ended", "", "its log says the run completed (run.terminal.completed) although its process is still alive", "");
                app.update_lane(&l.id, |x| x.note = if x.note.is_empty() { "run completed in its log (run.terminal.completed); its process was still alive".into() } else { std::mem::take(&mut x.note) });
                kill_group(l.pid.unwrap_or(0), libc::SIGKILL);
                if !app.children.lock().unwrap().contains(&l.id) {
                    ended(&app, &l.id, None).await; // no waiter task (daemon restart): the killed process ends here, else its waiter ends it
                }
                continue;
            }
            let key = format!("long_running:{}", l.id);
            if l.status == "running" && long_running_due(l.started_at.as_deref(), now_s(), lim.long_running_minutes, app.kv(&key).is_some()) {
                app.set_kv(&key, Some("1"));
                let cmds = last_commands(&text, 5).iter().map(|c| format!("- {c}")).collect::<Vec<_>>().join("\n");
                let text = format!("lane {} has been running for more than {} min. Check that it advances or whether it waits on something stuck. Its last commands:\n{cmds}", l.id, lim.long_running_minutes);
                manager::emit_event(&app, &l.project, "lane.long", &text, None);
            }
            if l.status == "running" && now_s().saturating_sub(l.last_activity).min(quiet) > lim.inactivity_minutes * 60 {
                kill_group(l.pid.unwrap_or(0), libc::SIGKILL);
                let why = format!("no new output for {} min (an agent stuck retrying its API call counts as silent)", lim.inactivity_minutes);
                app.event(&l.id, &l.project, "idle", "", &format!("stopped: {why}"), "");
                app.update_lane(&l.id, |x| x.pid = None);
                send_back(&app, l, &format!("Your agent was stopped: {why}. Continue the work on your branch."));
            } else if !app.children.lock().unwrap().contains(&l.id) && !l.pid.is_some_and(alive) {
                ended(&app, &l.id, None).await; // its process ended while the daemon was down
            }
        }
        let Some(()) = (!lim.max_parallel.is_some_and(|m| running >= m) && !(running > 0 && load >= lim.max_load)).then_some(()) else { continue };
        let mut ok = |p: &Project| {
            let why = blocked(&app, p, &lanes);
            why.as_ref().filter(|w| said.get(&p.id) != Some(*w)).into_iter().for_each(|w| _ = app.event("", &p.id, "paused", "", &format!("no new lane: {w}"), ""));
            app.set_kv(&format!("paused:{}", p.id), why.as_deref());
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
                app.update_lane(&next.id, |x| (x.status, x.note, x.ended_at) = ("failed".into(), e.clone(), Some(now_iso()))).into_iter().for_each(|l| cleanup_later(&app, l, false, 0));
                manager::emit_event(&app, &next.project, "lane.not_started", &format!("lane {} could not start: {e}", next.id), None);
            }
        }
    }
}

/// A fresh worktree slot of the project for this lane alone, checked out on its `lane/<id>` branch: at main for a new branch, where the branch stands when a lane continues after its old worktree went. No slot is kept free or reused: a finished lane's slot is already gone (see `cleanup_lane`).
async fn take_slot(app: &Shared, project: &Project, lane: &Lane) -> Result<PathBuf, String> {
    let (repo, dir) = (project.repo(), project.lanes_dir());
    let mut nums: Vec<u32> = std::fs::read_dir(&dir).into_iter().flatten().flatten().filter_map(|e| e.file_name().to_str()?.strip_prefix("slot-")?.parse().ok()).collect();
    nums.sort();
    let slot = dir.join(format!("slot-{}", nums.last().map_or(1, |n| n + 1)));
    add_worktree(&repo, &slot, None, &app.manifest(&repo).lane_exclude).await.inspect_err(|_| drop(std::fs::remove_dir_all(&slot)))?;
    let (known, main) = (git(&repo, &["rev-parse", "--verify", "-q", &lane.branch]).await.is_ok(), git(&repo, &["rev-parse", "HEAD"]).await?);
    let args: Vec<&str> = if known { vec!["checkout", "-q", "-f", &lane.branch] } else { vec!["checkout", "-q", "-f", "-B", &lane.branch, &main] };
    git(&slot, &args).await?;
    Ok(slot)
}
fn brief(app: &Shared, lane: &Lane) -> String {
    let steps = plan::read(app, &lane.project).unwrap_or_default();
    let lanes: Vec<Lane> = app.lanes.lock().unwrap().values().filter(|l| l.project == lane.project).cloned().collect();
    let step = steps.iter().find(|s| s.num == lane.step);
    let check = step.and_then(|s| s.items.iter().find(|i| plan::same(&i.text, &lane.item))).and_then(|i| i.check.clone());
    let tag =
        |i: &plan::Item| lanes.iter().filter(|l| l.step == lane.step && plan::same(&l.item, &i.text)).max_by(|a, b| a.created_at.cmp(&b.created_at)).filter(|_| !i.done).map(|l| format!("  ({}: {})", l.id, l.status)).unwrap_or_default();
    let items: Vec<String> = step.map(|s| &s.items[..]).unwrap_or_default().iter().map(|i| format!("- [{}] {}{}", if i.done { "x" } else { " " }, i.text, tag(i))).collect();
    let others: Vec<String> = lanes.iter().filter(|l| l.id != lane.id && matches!(l.status.as_str(), "running" | "waiting")).map(|l| format!("- {} ({}): {}", l.id, l.status, l.item)).collect();
    let red = app.kv(&format!("main_red:{}", lane.project));
    let rules: Vec<String> = app.cfg().project(&lane.project).map(|p| app.manifest(&p.repo()).rules).unwrap_or_default().iter().map(|r| format!("- {}", r.message)).collect();
    let vars = HashMap::from([
        ("id", lane.id.clone()),
        ("project", lane.project.clone()),
        ("step", lane.step.clone()),
        ("item", lane.item.clone()),
        ("check", check.map(|c| format!("\nIts check (must pass on the merged result): `{c}`\n")).unwrap_or_default()),
        ("items", items.join("\n")),
        ("others", if others.is_empty() { "none".into() } else { others.join("\n") }),
        ("red", red.map(|r| format!("red ({r}): not your job unless your item is about it")).unwrap_or("green".into())),
        ("ws", lane.workspace.clone()),
        ("branch", lane.branch.clone()),
        ("clippy", if Path::new(&lane.workspace).join("Cargo.toml").exists() { app.cfg().factory.prompt("clippy") } else { String::new() }), // Rust lanes run the queue's clippy gate first, so a warning does not cost a second attempt
        ("rules", rules.join("\n")),
        ("at", now_iso()),
    ]);
    static PLACEHOLDER: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\{(\w+)\}").unwrap());
    PLACEHOLDER.replace_all(&app.cfg().factory.prompt("brief"), |c: &regex::Captures| vars[&c[1]].clone()).into_owned()
}
/// Runs the lane's agent in its slot (a new slot for a lane without one): the brief, or a resume of its session with the pending message.
async fn launch(app: &Shared, mut lane: Lane) -> Result<(), String> {
    let cfg = app.cfg();
    let project = cfg.project(&lane.project).ok_or("unknown project")?.clone();
    if lane.workspace.is_empty() || !Path::new(&lane.workspace).exists() {
        lane.branch = format!("lane/{}", lane.id);
        lane.workspace = take_slot(app, &project, &lane).await?.display().to_string();
        lane.session.clear(); // an agent session lives in its folder: in another slot the lane starts from the brief
        app.save_lane(&lane);
    }
    let ws = PathBuf::from(&lane.workspace);
    let msg = std::mem::take(&mut lane.resume);
    let brief_path = app.home.join(format!("briefs/{}.md", lane.id));
    let resume = (!lane.session.is_empty() && !msg.is_empty()).then_some((lane.session.as_str(), msg.as_str()));
    if resume.is_none() {
        std::fs::write(&brief_path, brief(app, &lane) + &if msg.is_empty() { String::new() } else { format!("\n## Message from Tirith\n{msg}\n") }).map_err(|e| e.to_string())?;
    }
    let role = models::role(app, &models::lane_role(&cfg));
    let over = (lane.agent != role.agent || lane.model.is_some()).then_some(lane.agent.as_str());
    let cmd = models::lane_argv(app, over, lane.model.as_deref(), &brief_path, &ws, resume)?;
    lane.model = lane.model.or_else(|| role.model.clone().filter(|_| lane.agent == role.agent));
    let used = lane.model.clone().or(role.model.filter(|_| over.is_none())).map_or_else(|| cmd.alias.clone(), |m| format!("{}:{m}", cmd.alias));
    let _ = (!lane.tried.contains(&used)).then(|| lane.tried.push(used)); // the fallback chain skips what this lane already used
    let launch = sandbox::wrap(app, &lane.id, cmd.argv);
    let mut log = std::fs::OpenOptions::new().create(true).append(true).open(app.home.join(format!("logs/lanes/{}.log", lane.id))).map_err(|e| e.to_string())?;
    let _ = std::io::Write::write_fmt(&mut log, format_args!("{RUN_START}{}\n", now_iso()));
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
    (lane.pid, lane.status, lane.started_at, lane.ended_at, lane.last_activity) = (child.id(), "running".into(), Some(now_iso()), None, now_s());
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
/// The agent's process is gone: a working lane goes to the merge queue (or, out of credits, to the next agent of the fallback chain); a waiting lane waits for its answer.
async fn ended(app: &Shared, id: &str, code: Option<i32>) {
    let Some(lane) = app.lane(id) else { return };
    let raw = tail(&app.home.join(format!("logs/lanes/{id}.log")), 200);
    let _ = (lane.agent == "codex").then(|| codex_session(app, id, &raw));
    // Only what the agent program itself said counts: its plain-text output and its final result event, never the files and command output it read (a lane working on Tirith reads the very patterns that detect it).
    let log = raw
        .lines()
        .filter_map(|l| match serde_json::from_str::<serde_json::Value>(l) {
            Ok(v) if l.trim_start().starts_with('{') => match v["type"].as_str() {
                Some("result") if v["is_error"] == true => Some(l.to_string()),
                Some("turn.failed") => v["error"]["message"].as_str().map(String::from),
                Some("error") => v["message"].as_str().map(String::from),
                _ => None,
            },
            _ if !l.trim_start().starts_with('{') => Some(l.to_string()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n");
    let log = crate::app::redact_secrets(&log);
    if lane.status == "running" && models::out_of_credits(&log) {
        let next = models::fallback(app, &models::lane_role(&app.cfg()), &lane.tried);
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
            manager::emit_event(app, &lane.project, "lane.failed", &format!("lane {id} ({}: {}) failed: out of credits on every agent of the lane role", lane.project, lane.item), None);
            app.lane(id).into_iter().for_each(|l| cleanup_later(app, l, false, 0));
        }
    } else if lane.status == "running" && no_commits(app, &lane).await && plan::find(app, &lane.project, &lane.step, &lane.item).is_none_or(|i| i.check.is_none()) {
        app.update_lane(id, |l| (l.pid, l.ended_at, l.status, l.note) = (None, Some(now_iso()), "failed".into(), format!("ended without commits (exit {}): nothing to merge", code.map(|c| c.to_string()).unwrap_or("?".into()))));
        manager::emit_event(app, &lane.project, "lane.failed", &format!("lane {id} ({}: {}) failed: it ended without commits, nothing to merge", lane.project, lane.item), None);
        app.lane(id).into_iter().for_each(|l| cleanup_later(app, l, false, 0));
    } else {
        app.update_lane(id, |l| {
            l.pid = None;
            match l.status.as_str() {
                "running" => {
                    l.status = "merge-queued".into();
                    l.ended_at = Some(now_iso());
                    l.note = if l.note.is_empty() { format!("ended (exit {})", code.map(|c| c.to_string()).unwrap_or("?".into())) } else { std::mem::take(&mut l.note) };
                }
                "waiting" if !l.resume.is_empty() => l.status = "queued".into(), // answered while it ended its turn
                _ => {}
            }
        });
        app.merge_wake.notify_one();
    }
    app.sched_wake.notify_one();
}
fn codex_session(app: &Shared, id: &str, log: &str) {
    let run = log.rsplit(RUN_START).next().unwrap_or(log);
    let session = run.lines().filter_map(|l| serde_json::from_str::<serde_json::Value>(l).ok()).filter(|v| v["type"] == "thread.started").filter_map(|v| v["thread_id"].as_str().map(String::from)).next_back();
    session.filter(|s| app.lane(id).is_some_and(|l| l.session != *s)).into_iter().for_each(|s| drop(app.update_lane(id, |l| l.session = s)));
}
/// True when the lane's branch has no commit beyond the project's HEAD (it is known and countable). Without a check on its item such a lane failed; with one it may still be done (the check can already be true): it goes to the merge queue, where the check runs on main.
async fn no_commits(app: &Shared, lane: &Lane) -> bool {
    let Some(p) = app.cfg().projects.iter().find(|p| p.id == lane.project).cloned() else { return false };
    !lane.branch.is_empty() && matches!(git(&p.repo(), &["rev-list", "--count", &format!("HEAD..{}", lane.branch)]).await.as_deref(), Ok("0"))
}
/// `tirith ask` from a lane: it waits, and the manager is told how to answer.
pub fn ask(app: &Shared, id: &str, question: &str) -> Result<Lane, String> {
    let lane = app.lane(id).ok_or(format!("no lane '{id}'"))?;
    (lane.status == "running" && !question.trim().is_empty()).then_some(()).ok_or_else(|| format!("lane {id} is {}: only a working lane asks, with a question", lane.status))?;
    // A question about money stops the agent: it must not spend before the answer (it resumes with it).
    let updated = app.update_lane(id, |l| {
        (l.status, l.question, l.resume) = ("waiting".into(), question.trim().into(), String::new());
        is_payment(question).then(|| kill_agent(l.pid.take()));
    });
    let (q, owner) = (question.trim(), format!("tirith ask-owner --lane {id} --kind <kind> \"<the question, in the owner's words>\""));
    let told = format!("lane {id} ({}: {}) asks: {q}\nAnswer with: tirith answer {id} \"<answer>\". When only the owner can answer, ask them with {owner}: their answer goes straight to the lane.", lane.project, lane.item);
    manager::emit_event(app, &lane.project, "lane.asked", &format!("lane {id} asks the manager: {q}"), Some(&told));
    updated.ok_or("lane vanished".into())
}
/// `tirith answer` from the manager, or the owner's answer (`who`): a live message when the agent still runs, else a resumed run.
pub fn answer(app: &Shared, id: &str, text: &str, who: &str) -> Result<Lane, String> {
    let lane = app.lane(id).ok_or(format!("no lane '{id}'"))?;
    (lane.status == "waiting").then_some(()).ok_or_else(|| format!("lane {id} is {}, not waiting for an answer", lane.status))?;
    let live = lane.pid.is_some_and(alive);
    let _ = live.then(|| tell(id, &format!("Answer from {who}: {text}")));
    let lane = app.update_lane(id, |l| {
        l.resume = if l.session.is_empty() { format!("You asked the manager: {}\nAnswer from {who}: {text}", l.question) } else { format!("Answer from {who}: {text}") };
        let _ = (!live).then(|| l.status = "queued".into());
    });
    app.sched_wake.notify_one();
    lane.ok_or("lane vanished".into())
}
/// SIGTERM to the agent's group, SIGKILL five seconds later.
fn kill_agent(pid: Option<u32>) {
    let Some(pid) = pid else { return };
    kill_group(pid, libc::SIGTERM);
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(5)).await;
        kill_group(pid, libc::SIGKILL);
    });
}
pub fn is_payment(question: &str) -> bool {
    let q = question.to_lowercase();
    ["$", "€", "usd", "eur", "pay", "paid", "spend", "cost", "price", "charge", "billing", "pago", "pagar", "gasto", "gastar", "coste", "precio", "cuesta"].iter().any(|w| q.contains(w))
}
pub fn stop(app: &Shared, id: &str) -> Result<Lane, String> {
    let lane = app.lane(id).ok_or(format!("no lane '{id}'"))?;
    matches!(lane.status.as_str(), "queued" | "running" | "waiting" | "merge-queued").then_some(()).ok_or_else(|| format!("lane {id} is {}", lane.status))?;
    kill_agent(lane.pid);
    app.update_lane(id, |l| (l.status, l.ended_at, l.pid) = ("stopped".into(), Some(now_iso()), None)).inspect(|l| cleanup_later(app, l.clone(), false, if lane.pid.is_some() { 6 } else { 0 })).ok_or("lane vanished".into())
}
/// `tirith lane delete`: the lane is thrown away. Its agent stops, its branch and unsaved work go, it leaves every list, and its open item leaves the plan, so nothing starts it again (stopping keeps all that: the autopilot only skips the item).
pub async fn delete(app: &Shared, id: &str) -> Result<(), String> {
    let lane = app.lane(id).ok_or(format!("no lane '{id}'"))?;
    (lane.status != "merging").then_some(()).ok_or_else(|| format!("lane {id} is being merged right now: try again in a minute"))?;
    kill_agent(lane.pid);
    app.lanes.lock().unwrap().remove(id);
    let _ = app.db.lock().unwrap().execute("DELETE FROM lanes WHERE id = ?1", [id]);
    app.emit("lane", serde_json::json!({"id": id, "deleted": true}));
    if lane.status != "merged" {
        let _ = plan::drop_item(app, &lane.project, &lane.step, &lane.item).await;
    }
    let Some(p) = app.cfg().project(&lane.project).cloned() else { return Ok(()) };
    if lane.pid.is_some() {
        tokio::time::sleep(Duration::from_secs(6)).await; // the agent is gone (kill_agent) before its folder is touched
    }
    // Its worktree goes in full, unless another lane has that folder now (this lane is out of the map already); with it gone, its branch can be deleted.
    let held = app.lanes.lock().unwrap().values().any(|l| l.workspace == lane.workspace);
    if !held && !lane.workspace.is_empty() {
        remove_worktree(&p.repo(), Path::new(&lane.workspace)).await;
    }
    if !lane.branch.is_empty() {
        drop(git(&p.repo(), &["branch", "-D", &lane.branch]).await);
    }
    Ok(())
}
/// The merge failed because of the lane: the same agent continues in its slot with the reason, twice at most.
pub fn send_back(app: &Shared, lane: &Lane, reason: &str) {
    if lane.attempts >= 2 {
        app.update_lane(&lane.id, |l| (l.status, l.note) = ("failed".into(), reason.chars().take(2000).collect())).into_iter().for_each(|l| cleanup_later(app, l, false, 0));
        manager::emit_event(app, &lane.project, "lane.failed", &format!("lane {} ({}: {}) failed after {} tries: {}", lane.id, lane.project, lane.item, lane.attempts + 1, reason.lines().next().unwrap_or("")), None);
        return;
    }
    app.update_lane(&lane.id, |l| {
        (l.attempts, l.solo, l.resume, l.note, l.status) =
            (l.attempts + 1, false, format!("Tirith sent your work back:\n\n{reason}\n\nFix it on your branch, commit, and end your turn."), format!("sent back: {}", reason.lines().next().unwrap_or("")), "queued".into())
    });
    app.sched_wake.notify_one();
}
#[cfg(test)]
#[path = "../checks/lanes.rs"]
mod tests;
