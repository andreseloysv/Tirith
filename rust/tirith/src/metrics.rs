//! Health metrics computed every minute from the daemon's own data, and the CEO review that a metric red for long enough starts; while everything is green no model is called.

use crate::app::{kill_group, log_age, now_iso, now_s, tail, Lane, Shared};
use crate::{manager, models, plan};
use axum::{extract::State, routing::get, Json, Router};
use serde_json::{json, Value};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{LazyLock, Mutex};
use std::{collections::HashMap, time::Duration};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

struct Metric {
    id: &'static str,
    label: String,
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
/// A kv flag set by another module: missing, empty, "0", "off" or "false" is off.
fn flag(app: &Shared, key: &str) -> Option<String> {
    app.kv(key).filter(|v| !matches!(v.trim(), "" | "0" | "off" | "false"))
}

fn owner_questions(app: &Shared) -> Vec<String> {
    manager::owner_open(app).iter().map(|o| format!("{}: {}", o["project"].as_str().unwrap_or(""), o["question"].as_str().unwrap_or("")).trim_start_matches(": ").to_string()).collect()
}

/// Why the autopilot starts no lane for the project (the scheduler's last «paused» reason) and the command that fixes it.
fn paused_note(app: &Shared, project: &str) -> String {
    app.kv(&format!("paused:{project}")).map(|w| paused_text(project, &w)).unwrap_or_default()
}
fn paused_text(project: &str, why: &str) -> String {
    format!(" [{project} paused: {why}; fix: {}]", crate::lanes::paused_fix(project, why))
}

/// Autopilot projects with nothing the autopilot can start in the plan, with the command that adds work.
fn empty_plan_note(empty: &[String]) -> String {
    empty.iter().map(|p| format!(" [{p} plan empty: nothing startable; fix: tirith plan add {p} <step> \"<text>\"]")).collect()
}
fn merged_at(l: &Lane) -> Option<i64> {
    l.merged_at.as_deref().or(l.ended_at.as_deref()).and_then(secs)
}
fn compute(app: &Shared) -> Vec<Metric> {
    let cfg = app.cfg();
    let now = now_s() as i64;
    let day = now - 86400;
    let lanes: Vec<Lane> = app.lanes.lock().unwrap().values().cloned().collect();
    let f = &cfg.factory;
    // A measure with the factory's label, target and side for its id.
    let mk = |id: &'static str, value: f64, red: bool, facts: String| {
        let (label, target, at_least) = f.metric(id);
        Metric { id, label, value, target, at_least, red, facts }
    };
    // Open items of the projects the autopilot works on.
    let mut open: Vec<(String, usize)> = vec![];
    let mut empty: Vec<String> = vec![];
    for p in cfg.projects.iter().filter(|p| crate::lanes::autopilot_on(app, &p.id)) {
        // Only items the autopilot can start: no step-0 role items, no owner-wait items.
        let n = plan::read(app, &p.id).map(|s| s.iter().flat_map(|s| &s.items).filter(|i| !i.done && !crate::lanes::is_autopilot_skipped(&i.text)).count()).unwrap_or(0);
        if n > 0 {
            open.push((p.id.clone(), n));
        } else {
            empty.push(p.id.clone());
        }
    }
    let work = !open.is_empty();
    let merged: Vec<&Lane> = lanes.iter().filter(|l| l.status == "merged" && merged_at(l).is_some_and(|m| m >= day)).collect();
    let ids = |v: &[&Lane]| v.iter().map(|l| l.id.as_str()).collect::<Vec<_>>().join(", ");
    let open_txt = open.iter().map(|(p, n)| format!("{p} {n}")).collect::<Vec<_>>().join(", ");
    let mut out = vec![];

    let target = f.metric("merged").1.unwrap_or(0.0);
    let facts = format!(
        "merged in 24 h: [{}]; open items in autopilot projects: [{open_txt}]; {}{}{}",
        ids(&merged),
        if empty.is_empty() { "no empty plans: lanes that do not reach main are the problem" } else { "no startable work in the plan (not a lane failure) for:" },
        empty_plan_note(&empty),
        open.iter().map(|(p, _)| paused_note(app, p)).collect::<String>()
    );
    out.push(mk("merged", merged.len() as f64, work && (merged.len() as f64) < target, facts));

    let mut mins: Vec<(f64, &str)> = merged.iter().filter_map(|l| Some(((merged_at(l)? - secs(&l.created_at)?) as f64 / 60.0, l.id.as_str()))).collect();
    mins.sort_by(|a, b| a.0.total_cmp(&b.0));
    let median = if mins.is_empty() { 0.0 } else { (mins[(mins.len() - 1) / 2].0 + mins[mins.len() / 2].0) / 2.0 };
    let target = f.metric("median").1.unwrap_or(0.0);
    let facts = mins.iter().rev().take(8).map(|(m, id)| format!("{id} {m:.0} min")).collect::<Vec<_>>().join(", ");
    out.push(mk("median", median.round(), !mins.is_empty() && median > target, facts));

    let first: Vec<&Lane> = merged.iter().filter(|l| l.attempts == 0).copied().collect();
    let pct = if merged.is_empty() { 100.0 } else { (100 * first.len() / merged.len()) as f64 };
    let retried: Vec<&Lane> = merged.iter().filter(|l| l.attempts > 0).copied().collect();
    let target = f.metric("first_try").1.unwrap_or(0.0);
    out.push(mk("first_try", pct, !merged.is_empty() && pct < target, format!("merged after being sent back: [{}]", ids(&retried))));

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
    let target = f.metric("idle").1.unwrap_or(0.0);
    let facts = idle.iter().map(|(p, n, m)| format!("{p}: {n} open items, nothing running for {m} min{}", paused_note(app, p))).collect::<Vec<_>>().join("; ");
    out.push(mk("idle", worst, worst > target, facts));

    let half = f.limits.inactivity_minutes * 30;
    let stuck: Vec<String> = lanes
        .iter()
        .filter(|l| l.status == "running")
        .filter_map(|l| {
            let quiet = (now as u64).saturating_sub(l.last_activity).min(log_age(app, &l.id));
            (quiet > half).then(|| format!("{} ({}) quiet {} min", l.id, l.project, quiet / 60))
        })
        .collect();
    let target = f.metric("stuck").1.unwrap_or(0.0);
    out.push(mk("stuck", stuck.len() as f64, stuck.len() as f64 > target, stuck.join("; ")));

    let red: Vec<String> = cfg.projects.iter().filter_map(|p| flag(app, &format!("main_red:{}", p.id)).map(|v| format!("{}: {v}", p.id))).collect();
    let target = f.metric("main_red").1.unwrap_or(0.0);
    out.push(mk("main_red", red.len() as f64, red.len() as f64 > target, red.join("; ")));

    let failed: Vec<String> = lanes.iter().filter(|l| l.status == "failed" && l.ended_at.as_deref().and_then(secs).is_some_and(|e| e >= day)).map(|l| format!("{}: {}", l.id, l.note.lines().next().unwrap_or(""))).collect();
    out.push(mk("failed", failed.len() as f64, false, failed.join("; ")));

    let qs = owner_questions(app);
    out.push(mk("owner", qs.len() as f64, false, qs.join("; ")));
    out
}

fn describe(m: &Metric) -> String {
    let target = m.target.map(|t| format!(" (target: {} {t})", if m.at_least { "at least" } else { "at most" })).unwrap_or_default();
    let facts = if m.facts.is_empty() { String::new() } else { format!(". Facts: {}", m.facts) };
    format!("{} [{}] is {}{target}{facts}", m.label, m.id, m.value)
}

/// One computation: episodes opened, fixed, noted and reviewed; the page gets the result.
fn tick(app: &Shared) {
    let metrics = compute(app);
    let now = now_s();
    let after = app.cfg().factory.routes_on("metric.red").iter().find(|r| r.0 == "reviewer").and_then(|r| r.4).or_else(|| app.cfg().factory.role::<u64>("ceo", "review_after_minutes")).unwrap_or(15) * 60;
    let (mut due, mut notes) = (vec![], vec![]);
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
        let list =
            metrics.iter().map(|m| json!({"id": m.id, "label": m.label, "value": m.value, "target": m.target, "at_least": m.at_least, "red": m.red, "since": book.episodes.get(m.id).map(|e| iso(e.since)), "facts": m.facts})).collect();
        book.last = json!(list);
        list
    };
    // A red main is not reported again here: its project's manager already got it from the merge queue, with the facts.
    notes.into_iter().filter(|m| m.id != "main_red").for_each(|m| {
        let _ = (m.id == "idle").then(|| app.sched_wake.notify_one()); // the mechanical fix: wake the scheduler and the autopilot
        manager::emit_event(app, "", "metric.red", &format!("health: {}. Get it moving.", describe(m)), None);
    });
    let max = app.cfg().factory.role::<u64>("ceo", "max_per_day").unwrap_or(6);
    let today: u64 = app.db.lock().unwrap().query_row("SELECT COUNT(*) FROM ceo_reviews WHERE at >= ?1 AND metric != 'owner'", [iso(now.saturating_sub(86400))], |r| r.get(0)).unwrap_or(0);
    let _ = (!due.is_empty() && today < max).then(|| start_review(app, due.iter().map(|d| d.0).collect::<Vec<_>>().join(","), due.iter().map(|d| d.1.as_str()).collect::<Vec<_>>().join("\n")));
    app.emit("metrics", payload(app, list));
}

fn payload(app: &Shared, metrics: Vec<Value>) -> Value {
    json!({"metrics": metrics, "reviews": app.recent("ceo_reviews", &["id", "at", "metric", "text"], None, 10)})
}

fn start_review(app: &Shared, metric: String, why: String) {
    let Some(()) = (!REVIEWING.swap(true, Ordering::SeqCst)).then_some(()) else { return };
    let app = app.clone();
    tokio::spawn(async move {
        app.emit("ceo", json!({"running": true}));
        let result = review(&app, &why).await;
        let text = result.clone().unwrap_or_else(|e| format!("(the CEO review could not run: {e})"));
        let _ = app.db.lock().unwrap().execute("INSERT INTO ceo_reviews (at, metric, text) VALUES (?1, ?2, ?3)", rusqlite::params![now_iso(), metric, text]);
        app.chat("ceo", &text, "");
        let _ = result.is_ok().then(|| manager::emit_event(&app, "", "review.done", &format!("CEO review: {text}"), None));
        REVIEWING.store(false, Ordering::SeqCst);
        app.emit("ceo", json!({"running": false}));
        let metrics = BOOK.lock().unwrap().last.as_array().cloned().unwrap_or_default();
        app.emit("metrics", payload(&app, metrics));
    });
}

/// The facts a review reads: the red metrics, the last lanes, failing checks, plan progress, owner questions.
fn brief(app: &Shared, why: &str) -> String {
    let mut lanes: Vec<Lane> = app.lanes.lock().unwrap().values().cloned().collect();
    lanes.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    let mut s = format!("# Red metrics\n{why}\n\n# Last lanes (newest first)\n");
    lanes.iter().take(20).for_each(|l| s += &format!("- {} [{}] {}:{} \"{}\" tries {} created {}: {}\n", l.id, l.status, l.project, l.step, l.item, l.attempts + 1, l.created_at, l.note.lines().next().unwrap_or("")));
    s += "\n# Failing checks (last 24 h)\n";
    let day = iso(now_s().saturating_sub(86400));
    let runs: Vec<(String, String, String, String)> =
        app.query("SELECT lane, stage, status, COALESCE(log, '') FROM runs WHERE status IN ('failed', 'timeout') AND started_at >= ?1 ORDER BY id DESC LIMIT 5", [&day], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)));
    runs.iter().for_each(|(lane, stage, status, log)| s += &format!("- {lane} check '{stage}' {status}:\n```\n{}\n```\n", tail(std::path::Path::new(log), 15)));
    s += "\n# Plans\n";
    for p in &app.cfg().projects {
        let auto = if crate::lanes::autopilot_on(app, &p.id) { "on" } else { "off" };
        let main = flag(app, &format!("main_red:{}", p.id)).unwrap_or("green".into());
        s += &format!("## {} (autopilot {auto}, main {main})\n", p.id);
        for st in plan::read(app, &p.id).unwrap_or_default() {
            let open = st.items.iter().filter(|i| !i.done).count();
            s += &format!("- step {}. {}: {open} open, {} done\n", st.num, st.title, st.items.len() - open);
        }
    }
    s += "\n# Open owner questions\n";
    owner_questions(app).into_iter().for_each(|q| s += &format!("- {q}\n"));
    s
}

/// The CEO review is read-only: file-writing tools are denied; everything else, including every `tirith` command that only reads (status, plan show, lane log, timeout get, merge status), is allowed.
fn ceo_permission(tool: &str, input: &Value) -> Value {
    if crate::rules::EDIT_TOOLS.contains(&tool) {
        json!({"behavior": "deny", "message": "the CEO review is read-only"})
    } else {
        json!({"behavior": "allow", "updatedInput": input})
    }
}

/// One fresh, read-only session: one user message in, the assistant's text until the "result" event out.
async fn review(app: &Shared, why: &str) -> Result<String, String> {
    let prompt = app.cfg().factory.prompt("reviewer");
    if matches!(models::role(app, "ceo").agent.as_str(), "codex" | "muse") {
        let b = brief(app, why);
        return crate::codex::turn(app, "ceo", "", None, (&prompt, &b), |_| {}, |_| {}).await;
    }
    let mut cmd = models::chat_argv(app, "ceo", None)?;
    cmd.argv.extend(["--append-system-prompt".into(), prompt]);
    let (mut child, mut stdin, stdout) = manager::spawn_chat(app, "ceo", "", cmd.argv)?;
    let pid = child.id().unwrap_or(0);
    let mut lines = BufReader::new(stdout).lines();
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
                    let behavior = ceo_permission(msg["request"]["tool_name"].as_str().unwrap_or(""), &msg["request"]["input"]);
                    let _ = stdin.write_all(&send(manager::control_reply(&msg, behavior))).await;
                }
                _ => {}
            }
        }
        false
    };
    let finished = tokio::time::timeout(Duration::from_secs(600), read).await;
    kill_group(pid, libc::SIGKILL);
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

pub fn routes() -> Router<Shared> {
    Router::new().route("/api/metrics", get(metrics))
}

#[cfg(test)]
#[path = "../checks/metrics.rs"]
mod tests;
