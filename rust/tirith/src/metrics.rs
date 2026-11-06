//! Health metrics and the CEO.
//!
//! Every minute code computes the metrics from the daemon's own data (lanes, runs, plans, kv). A metric that
//! turns red starts an episode: the mechanical fix when there is one, then one note to the manager. Still red
//! `[ceo] review_after_minutes` later, one CEO review runs for that episode: a fresh, independent, read-only
//! session whose answer goes to the owner's chat and to the manager. Back to green ends the episode.
//! While everything is green no model is called.

use crate::app::{kill_group, now_iso, now_s, tail, Lane, Shared};
use crate::{config, manager, models, plan, sandbox};
use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{LazyLock, Mutex};
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

const PROMPT: &str = "You are the CEO review inside Tirith: an independent, read-only, one-shot review of how the \
work is run (not of the code). Tirith runs coding agents from each project's plan: lanes (one agent, one plan item, \
one git worktree), a merge queue that only merges verified work, an autopilot that starts plan items, and a manager \
agent that runs the work and talks to the owner. You get the health metrics that are red, the facts behind them, \
the last lanes, failing checks, plan progress and open owner questions. You may read files and run read-only \
commands; change nothing. Decide, do not ask questions. Answer briefly in Spanish (the owner reads it): \
1) what is wrong and its most likely cause, 2) the concrete actions the manager must take now, as the commands it \
runs (tirith lane start <project> <step> \"<item>\", tirith lane stop <lane>, tirith plan add <project> <step> \
\"<item>\", tirith answer …, tirith autopilot …).";

struct Metric {
    id: &'static str,
    label: &'static str,
    value: f64,
    /// `None` for metrics shown without a target; `at_least` says which side of the target is green.
    target: Option<f64>,
    at_least: bool,
    red: bool,
    facts: String,
}

/// A red stretch of one metric: since when (the manager was told then) and whether its review ran.
struct Episode {
    since: u64,
    reviewed: bool,
}

#[derive(Default)]
struct Book {
    last: Value,
    episodes: HashMap<String, Episode>,
    idle_since: HashMap<String, u64>,
}

static BOOK: LazyLock<Mutex<Book>> = LazyLock::new(Mutex::default);
static REVIEWING: AtomicBool = AtomicBool::new(false);

fn secs(iso: &str) -> Option<i64> {
    chrono::DateTime::parse_from_rfc3339(iso).ok().map(|d| d.timestamp())
}

fn iso(s: u64) -> String {
    chrono::DateTime::from_timestamp(s as i64, 0).map(|d| d.format("%Y-%m-%dT%H:%M:%SZ").to_string()).unwrap_or_default()
}

fn ceo_cfg<T: serde::de::DeserializeOwned>(app: &Shared, path: &[&str], default: T) -> T {
    config::get(&app.cfg().raw, path).unwrap_or(default)
}

/// A kv flag set by another module: missing, empty, "0", "off" or "false" is off.
fn flag(app: &Shared, key: &str) -> Option<String> {
    app.kv(key).filter(|v| !matches!(v.trim(), "" | "0" | "off" | "false"))
}

/// The open owner questions (table of the core module; missing means none).
fn owner_questions(app: &Shared) -> Vec<String> {
    let db = app.db.lock().unwrap();
    let Ok(mut q) = db.prepare("SELECT question FROM owner_items WHERE status = 'open' ORDER BY id") else { return vec![] };
    q.query_map([], |r| r.get::<_, String>(0)).map(|r| r.flatten().collect()).unwrap_or_default()
}

fn merged_at(l: &Lane) -> Option<i64> {
    let v = json!(l);
    v["merged_at"].as_str().or(l.ended_at.as_deref()).and_then(secs)
}

fn compute(app: &Shared) -> Vec<Metric> {
    let cfg = app.cfg();
    let now = now_s() as i64;
    let day = now - 86400;
    let lanes: Vec<Lane> = app.lanes.lock().unwrap().values().cloned().collect();
    let t = |k: &str, d: f64| ceo_cfg(app, &["ceo", "targets", k], d);
    // Open items of the projects the autopilot works on.
    let mut open: Vec<(String, usize)> = vec![];
    for p in cfg.projects.iter().filter(|p| flag(app, &format!("autopilot:{}", p.id)).is_some()) {
        let n = plan::read(app, &p.id).map(|s| s.iter().flat_map(|s| &s.items).filter(|i| !i.done).count()).unwrap_or(0);
        if n > 0 {
            open.push((p.id.clone(), n));
        }
    }
    let work = !open.is_empty();
    let merged: Vec<&Lane> = lanes.iter().filter(|l| l.status == "merged" && merged_at(l).is_some_and(|m| m >= day)).collect();
    let ids = |v: &[&Lane]| v.iter().map(|l| l.id.as_str()).collect::<Vec<_>>().join(", ");
    let open_txt = open.iter().map(|(p, n)| format!("{p} {n}")).collect::<Vec<_>>().join(", ");
    let mut out = vec![];

    let target = t("merged_per_day", 10.0);
    out.push(Metric { id: "merged", label: "En main (24 h)", value: merged.len() as f64, target: Some(target), at_least: true,
        red: work && (merged.len() as f64) < target,
        facts: format!("merged in 24 h: [{}]; open items in autopilot projects: [{open_txt}]", ids(&merged)) });

    let mut mins: Vec<(f64, &str)> = merged
        .iter()
        .filter_map(|l| Some(((merged_at(l)? - secs(&l.created_at)?) as f64 / 60.0, l.id.as_str())))
        .collect();
    mins.sort_by(|a, b| a.0.total_cmp(&b.0));
    let median = if mins.is_empty() { 0.0 } else { (mins[(mins.len() - 1) / 2].0 + mins[mins.len() / 2].0) / 2.0 };
    let target = t("median_merge_minutes", 120.0);
    out.push(Metric { id: "median", label: "Mediana lanzar→main (min)", value: median.round(), target: Some(target), at_least: false,
        red: !mins.is_empty() && median > target,
        facts: mins.iter().rev().take(8).map(|(m, id)| format!("{id} {m:.0} min")).collect::<Vec<_>>().join(", ") });

    let first: Vec<&Lane> = merged.iter().filter(|l| l.attempts == 0).copied().collect();
    let pct = if merged.is_empty() { 100.0 } else { (100 * first.len() / merged.len()) as f64 };
    let retried: Vec<&Lane> = merged.iter().filter(|l| l.attempts > 0).copied().collect();
    let target = t("first_try_pct", 50.0);
    out.push(Metric { id: "first_try", label: "A la primera (%)", value: pct, target: Some(target), at_least: true,
        red: !merged.is_empty() && pct < target, facts: format!("merged after being sent back: [{}]", ids(&retried)) });

    // Idle: a project with open items, autopilot on and nothing queued, running or merging.
    let mut idle = vec![];
    {
        let mut book = BOOK.lock().unwrap();
        book.idle_since.retain(|p, _| open.iter().any(|(o, _)| o == p));
        for (p, n) in &open {
            if lanes.iter().any(|l| &l.project == p && l.active()) {
                book.idle_since.remove(p);
            } else {
                let since = *book.idle_since.entry(p.clone()).or_insert(now as u64);
                idle.push((p.clone(), *n, (now as u64 - since) / 60));
            }
        }
    }
    let worst = idle.iter().map(|i| i.2).max().unwrap_or(0) as f64;
    let target = t("idle_minutes", 10.0);
    out.push(Metric { id: "idle", label: "Parado con trabajo (min)", value: worst, target: Some(target), at_least: false,
        red: worst > target,
        facts: idle.iter().map(|(p, n, m)| format!("{p}: {n} open items, nothing running for {m} min")).collect::<Vec<_>>().join("; ") });

    let half = cfg.inactivity_min * 30;
    let stuck: Vec<String> = lanes
        .iter()
        .filter(|l| l.status == "running")
        .filter_map(|l| {
            let log = std::fs::metadata(app.home.join(format!("logs/lanes/{}.log", l.id)))
                .and_then(|m| m.modified())
                .map(|t| t.elapsed().map(|d| d.as_secs()).unwrap_or(0))
                .unwrap_or(u64::MAX);
            let quiet = (now as u64).saturating_sub(l.last_activity).min(log);
            (quiet > half).then(|| format!("{} ({}) quiet {} min", l.id, l.project, quiet / 60))
        })
        .collect();
    let target = t("stuck", 0.0);
    out.push(Metric { id: "stuck", label: "Lanes atascadas", value: stuck.len() as f64, target: Some(target), at_least: false,
        red: stuck.len() as f64 > target, facts: stuck.join("; ") });

    let red: Vec<String> =
        cfg.projects.iter().filter_map(|p| flag(app, &format!("main_red:{}", p.id)).map(|v| format!("{}: {v}", p.id))).collect();
    let target = t("main_red", 0.0);
    out.push(Metric { id: "main_red", label: "Main en rojo", value: red.len() as f64, target: Some(target), at_least: false,
        red: red.len() as f64 > target, facts: red.join("; ") });

    let failed: Vec<String> = lanes
        .iter()
        .filter(|l| l.status == "failed" && l.ended_at.as_deref().and_then(secs).is_some_and(|e| e >= day))
        .map(|l| format!("{}: {}", l.id, l.note.lines().next().unwrap_or("")))
        .collect();
    out.push(Metric { id: "failed", label: "Fallidas (24 h)", value: failed.len() as f64, target: None, at_least: false, red: false,
        facts: failed.join("; ") });

    let qs = owner_questions(app);
    out.push(Metric { id: "owner", label: "Preguntas al dueño", value: qs.len() as f64, target: None, at_least: false, red: false,
        facts: qs.join("; ") });
    out
}

fn describe(m: &Metric) -> String {
    let side = if m.at_least { "at least" } else { "at most" };
    let target = m.target.map(|t| format!(" (target: {side} {t})")).unwrap_or_default();
    let facts = if m.facts.is_empty() { String::new() } else { format!(". Facts: {}", m.facts) };
    format!("{} [{}] is {}{target}{facts}", m.label, m.id, m.value)
}

/// One computation: episodes opened, fixed, noted and reviewed; the page gets the result.
fn tick(app: &Shared) {
    let metrics = compute(app);
    let now = now_s();
    let after = ceo_cfg(app, &["ceo", "review_after_minutes"], 15u64) * 60;
    let mut due = vec![];
    let mut notes = vec![];
    let list: Vec<Value> = {
        let mut book = BOOK.lock().unwrap();
        for m in &metrics {
            if !m.red {
                book.episodes.remove(m.id);
                continue;
            }
            match book.episodes.get_mut(m.id) {
                None => {
                    book.episodes.insert(m.id.to_string(), Episode { since: now, reviewed: false });
                    notes.push(m);
                }
                Some(e) if !e.reviewed && now >= e.since + after => {
                    e.reviewed = true;
                    due.push((m.id, describe(m)));
                }
                _ => {}
            }
        }
        let list = metrics
            .iter()
            .map(|m| {
                let since = book.episodes.get(m.id).map(|e| iso(e.since));
                json!({"id": m.id, "label": m.label, "value": m.value, "target": m.target, "at_least": m.at_least, "red": m.red, "since": since, "facts": m.facts})
            })
            .collect();
        book.last = json!(list);
        list
    };
    for m in notes {
        if m.id == "idle" {
            app.sched_wake.notify_one(); // the mechanical fix: wake the scheduler and the autopilot
        }
        manager::notify(app, &format!("health: {}. Get it moving.", describe(m)));
    }
    let max = ceo_cfg(app, &["ceo", "max_per_day"], 6u64);
    if !due.is_empty() && reviews_today(app) < max {
        let metric = due.iter().map(|d| d.0).collect::<Vec<_>>().join(",");
        start_review(app, metric, due.iter().map(|d| d.1.as_str()).collect::<Vec<_>>().join("\n"));
    }
    app.emit("metrics", payload(app, list));
}

fn reviews_today(app: &Shared) -> u64 {
    let day = iso(now_s().saturating_sub(86400));
    app.db
        .lock()
        .unwrap()
        .query_row("SELECT COUNT(*) FROM ceo_reviews WHERE at >= ?1 AND metric != 'owner'", [day], |r| r.get(0))
        .unwrap_or(0)
}

fn payload(app: &Shared, metrics: Vec<Value>) -> Value {
    json!({"metrics": metrics, "reviews": app.recent("ceo_reviews", &["id", "at", "metric", "text"], 10)})
}

/// Starts one review in the background; false when one is already running.
fn start_review(app: &Shared, metric: String, why: String) -> bool {
    if REVIEWING.swap(true, Ordering::SeqCst) {
        return false;
    }
    let app = app.clone();
    tokio::spawn(async move {
        app.emit("ceo", json!({"running": true}));
        let result = review(&app, &why).await;
        let text = result.clone().unwrap_or_else(|e| format!("(the CEO review could not run: {e})"));
        let _ = app.db.lock().unwrap().execute(
            "INSERT INTO ceo_reviews (at, metric, text) VALUES (?1, ?2, ?3)",
            rusqlite::params![now_iso(), metric, text],
        );
        app.chat("ceo", &text);
        if result.is_ok() {
            manager::notify(&app, &format!("CEO review: {text}"));
        }
        REVIEWING.store(false, Ordering::SeqCst);
        app.emit("ceo", json!({"running": false}));
        let metrics = BOOK.lock().unwrap().last.as_array().cloned().unwrap_or_default();
        app.emit("metrics", payload(&app, metrics));
    });
    true
}

/// The facts a review reads: the red metrics, the last lanes, failing checks, plan progress, owner questions.
fn brief(app: &Shared, why: &str) -> String {
    let mut lanes: Vec<Lane> = app.lanes.lock().unwrap().values().cloned().collect();
    lanes.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    let mut s = format!("# Red metrics\n{why}\n\n# Last lanes (newest first)\n");
    for l in lanes.iter().take(20) {
        let note = l.report.as_ref().map(|r| r.note.as_str()).filter(|n| !n.is_empty()).unwrap_or(&l.note);
        s += &format!("- {} [{}] {}:{} \"{}\" tries {} created {}: {}\n", l.id, l.status, l.project, l.step, l.item, l.attempts + 1, l.created_at, note.lines().next().unwrap_or(""));
    }
    s += "\n# Failing checks (last 24 h)\n";
    let day = iso(now_s().saturating_sub(86400));
    let runs: Vec<(String, String, String, String)> = {
        let db = app.db.lock().unwrap();
        let q = db.prepare(
            "SELECT lane, stage, status, COALESCE(log, '') FROM runs WHERE status IN ('failed', 'timeout') AND started_at >= ?1 ORDER BY id DESC LIMIT 5",
        );
        q.and_then(|mut q| q.query_map([&day], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))?.collect()).unwrap_or_default()
    };
    for (lane, stage, status, log) in &runs {
        s += &format!("- {lane} check '{stage}' {status}:\n```\n{}\n```\n", tail(std::path::Path::new(log), 15));
    }
    s += "\n# Plans\n";
    for p in &app.cfg().projects {
        let auto = flag(app, &format!("autopilot:{}", p.id)).map_or("off", |_| "on");
        let main = flag(app, &format!("main_red:{}", p.id)).unwrap_or("green".into());
        s += &format!("## {} (autopilot {auto}, main {main})\n", p.id);
        for st in plan::read(app, &p.id).unwrap_or_default() {
            let open = st.items.iter().filter(|i| !i.done).count();
            s += &format!("- step {}. {}: {open} open, {} done\n", st.num, st.title, st.items.len() - open);
        }
    }
    s += "\n# Open owner questions\n";
    for q in owner_questions(app) {
        s += &format!("- {q}\n");
    }
    s
}

/// One fresh, read-only session: one user message in, the assistant's text until the "result" event out.
async fn review(app: &Shared, why: &str) -> Result<String, String> {
    let mut cmd = models::chat_argv(app, "ceo", None)?;
    cmd.argv.extend(["--append-system-prompt".into(), PROMPT.into()]);
    let launch = sandbox::wrap(app, "ceo", cmd.argv);
    let dir = app.home.join("ceo");
    let _ = std::fs::create_dir_all(&dir);
    let log = std::fs::OpenOptions::new().create(true).append(true).open(app.home.join("logs/ceo.log")).map_err(|e| e.to_string())?;
    let mut command = launch.command();
    if !launch.env.iter().any(|(k, _)| k == "PATH") {
        command.env("PATH", app.child_path());
    }
    let mut child = command
        .current_dir(&dir)
        .env("TIRITH_ROLE", "ceo")
        .env("TIRITH_HOME", &app.home)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(log)
        .process_group(0)
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| format!("cannot start the CEO ({}): {e}", launch.program))?;
    let pid = child.id().unwrap_or(0);
    let mut stdin = child.stdin.take().ok_or("no stdin")?;
    let mut lines = BufReader::new(child.stdout.take().ok_or("no stdout")?).lines();
    let msg = json!({"type": "user", "message": {"role": "user", "content": [{"type": "text", "text": brief(app, why)}]}});
    let send = |line: String| (line + "\n").into_bytes();
    stdin.write_all(&send(msg.to_string())).await.map_err(|e| e.to_string())?;
    let mut text = String::new();
    let read = async {
        while let Ok(Some(line)) = lines.next_line().await {
            let Ok(msg) = serde_json::from_str::<Value>(&line) else { continue };
            match msg["type"].as_str().unwrap_or("") {
                "assistant" => {
                    for b in msg["message"]["content"].as_array().into_iter().flatten().filter(|b| b["type"] == "text") {
                        text += b["text"].as_str().unwrap_or("");
                        text += "\n";
                    }
                }
                "result" => return true,
                "control_request" if msg["request"]["subtype"] == "can_use_tool" => {
                    let tool = msg["request"]["tool_name"].as_str().unwrap_or("");
                    let behavior = if matches!(tool, "Write" | "Edit" | "MultiEdit" | "NotebookEdit") {
                        json!({"behavior": "deny", "message": "the CEO review is read-only"})
                    } else {
                        json!({"behavior": "allow", "updatedInput": msg["request"]["input"]})
                    };
                    let reply = json!({"type": "control_response", "response": {"subtype": "success", "request_id": msg["request_id"], "response": behavior}});
                    let _ = stdin.write_all(&send(reply.to_string())).await;
                }
                _ => {}
            }
        }
        false
    };
    let finished = tokio::time::timeout(Duration::from_secs(600), read).await;
    if pid != 0 {
        kill_group(pid, libc::SIGKILL);
    }
    let _ = child.wait().await;
    let text = text.trim().to_string();
    match finished {
        Err(_) => Err(format!("no answer within 10 minutes{}", if text.is_empty() { String::new() } else { format!(": {text}") })),
        Ok(false) if text.is_empty() => Err("the session ended without an answer".into()),
        _ if text.is_empty() => Err("the answer was empty".into()),
        _ => Ok(text),
    }
}

pub fn start(app: Shared) {
    let _ = app.db.lock().unwrap().execute_batch("CREATE TABLE IF NOT EXISTS ceo_reviews (id INTEGER PRIMARY KEY, at TEXT, metric TEXT, text TEXT);");
    tokio::spawn(async move {
        loop {
            tick(&app);
            tokio::time::sleep(Duration::from_secs(60)).await;
        }
    });
}

async fn metrics(State(app): State<Shared>) -> Json<Value> {
    let list = BOOK.lock().unwrap().last.as_array().cloned().unwrap_or_default();
    let mut v = payload(&app, list);
    v["reviewing"] = json!(REVIEWING.load(Ordering::SeqCst));
    Json(v)
}

async fn review_now(State(app): State<Shared>) -> Result<Json<Value>, (StatusCode, String)> {
    let red: Vec<String> = compute(&app).iter().filter(|m| m.red).map(describe).collect();
    let why = format!("The owner asked for a review now. {}", if red.is_empty() { "Every metric is green.".into() } else { red.join("\n") });
    if start_review(&app, "owner".into(), why) {
        Ok(Json(json!({"ok": true, "started": true})))
    } else {
        Err((StatusCode::CONFLICT, "a CEO review is already running".into()))
    }
}

pub fn routes() -> Router<Shared> {
    Router::new().route("/api/metrics", get(metrics)).route("/api/ceo/review", post(review_now))
}

pub fn cli(_pos: &[&str], _flag: crate::client::Flag) -> Option<Result<(), String>> {
    None
}
