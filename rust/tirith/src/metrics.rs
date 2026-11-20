//! Health metrics computed every minute from the daemon's own data, and the turn-role review that a metric red for long enough starts (the factory's delayed `metric.red` route names the role); while everything is green no model is called.

use crate::app::{now_iso, now_s, tail, Lane, Shared};
use crate::{manager, plan};
use axum::{extract::State, routing::get, Json, Router};
use serde_json::{json, Value};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{LazyLock, Mutex};
use std::{collections::HashMap, time::Duration};

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

fn owner_questions(app: &Shared) -> Vec<String> {
    manager::owner_open(app).iter().map(|o| format!("{}: {}", o["project"].as_str().unwrap_or(""), o["question"].as_str().unwrap_or("")).trim_start_matches(": ").to_string()).collect()
}

/// Why the autopilot starts no lane for the project (the scheduler's last «paused» reason) and the command that fixes it.
fn paused_text(project: &str, why: &str) -> String {
    format!(" [{project} paused: {why}; fix: {}]", crate::lanes::paused_fix(project, why))
}

/// Autopilot projects with nothing the autopilot can start in the plan, with the command that adds work.
fn empty_plan_note(empty: &[String]) -> String {
    empty.iter().map(|p| format!(" [{p} plan empty: nothing startable; fix: tirith plan add {p} <step> \"<text>\"]")).collect()
}
fn compute(app: &Shared) -> Vec<Metric> {
    let cfg = app.cfg();
    let now = now_s() as i64;
    let day = now - 86400;
    let lanes: Vec<Lane> = app.lanes.lock().unwrap().values().cloned().collect();
    let f = &cfg.factory;
    // A measure with the factory's label, target and side for its id; red when it counts now (`live`) and is on the wrong side of its target.
    let mk = |id: &'static str, value: f64, live: bool, facts: String| {
        let (label, target, at_least) = f.metric(id);
        let red = live && target.is_some_and(|t| if at_least { value < t } else { value > t });
        Metric { id, label, value: value.round(), target, at_least, red, facts }
    };
    let secs = |s: &str| chrono::DateTime::parse_from_rfc3339(s).ok().map(|d| d.timestamp());
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
    let merged: Vec<&Lane> = lanes.iter().filter(|l| l.status == "merged" && l.merged_at.as_deref().or(l.ended_at.as_deref()).and_then(secs).is_some_and(|m| m >= day)).collect();
    let ids = |v: &[&Lane]| v.iter().map(|l| l.id.as_str()).collect::<Vec<_>>().join(", ");
    let open_txt = open.iter().map(|(p, n)| format!("{p} {n}")).collect::<Vec<_>>().join(", ");
    let mut out = vec![];

    let facts = format!(
        "merged in 24 h: [{}]; open items in autopilot projects: [{open_txt}]; {}{}{}",
        ids(&merged),
        if empty.is_empty() { "no empty plans: lanes that do not reach main are the problem" } else { "no startable work in the plan (not a lane failure) for:" },
        empty_plan_note(&empty),
        open.iter().map(|(p, _)| app.kv(&format!("paused:{p}")).map(|w| paused_text(p, &w)).unwrap_or_default()).collect::<String>()
    );
    out.push(mk("merged", merged.len() as f64, work, facts));

    let mut mins: Vec<(f64, &str)> = merged.iter().filter_map(|l| Some(((l.merged_at.as_deref().or(l.ended_at.as_deref()).and_then(secs)? - secs(&l.created_at)?) as f64 / 60.0, l.id.as_str()))).collect();
    mins.sort_by(|a, b| a.0.total_cmp(&b.0));
    let median = if mins.is_empty() { 0.0 } else { (mins[(mins.len() - 1) / 2].0 + mins[mins.len() / 2].0) / 2.0 };
    let facts = mins.iter().rev().take(8).map(|(m, id)| format!("{id} {m:.0} min")).collect::<Vec<_>>().join(", ");
    out.push(mk("median", median, !mins.is_empty(), facts));

    let first: Vec<&Lane> = merged.iter().filter(|l| l.attempts == 0).copied().collect();
    let pct = if merged.is_empty() { 100.0 } else { (100 * first.len() / merged.len()) as f64 };
    let retried: Vec<&Lane> = merged.iter().filter(|l| l.attempts > 0).copied().collect();
    out.push(mk("first_try", pct, !merged.is_empty(), format!("merged after being sent back: [{}]", ids(&retried))));

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
    let facts = idle.iter().map(|(p, n, m)| format!("{p}: {n} open items, nothing running for {m} min{}", app.kv(&format!("paused:{p}")).map(|w| paused_text(p, &w)).unwrap_or_default())).collect::<Vec<_>>().join("; ");
    out.push(mk("idle", worst, true, facts));

    let half = f.limits.inactivity_minutes * 30;
    let stuck: Vec<String> = lanes
        .iter()
        .filter(|l| l.status == "running")
        .filter_map(|l| {
            let log_age = std::fs::metadata(app.home.join(format!("logs/lanes/{}.log", l.id))).and_then(|m| m.modified()).map(|t| t.elapsed().map(|d| d.as_secs()).unwrap_or(0)).unwrap_or(u64::MAX);
            let quiet = (now as u64).saturating_sub(l.last_activity).min(log_age);
            (quiet > half).then(|| format!("{} ({}) quiet {} min", l.id, l.project, quiet / 60))
        })
        .collect();
    out.push(mk("stuck", stuck.len() as f64, true, stuck.join("; ")));

    let red: Vec<String> = cfg.projects.iter().filter_map(|p| app.kv(&format!("main_red:{}", p.id)).map(|v| format!("{}: {v}", p.id))).collect();
    out.push(mk("main_red", red.len() as f64, true, red.join("; ")));

    let failed: Vec<String> = lanes.iter().filter(|l| l.status == "failed" && l.ended_at.as_deref().and_then(secs).is_some_and(|e| e >= day)).map(|l| format!("{}: {}", l.id, l.note.lines().next().unwrap_or(""))).collect();
    out.push(mk("failed", failed.len() as f64, false, failed.join("; ")));

    let qs = owner_questions(app);
    out.push(mk("owner", qs.len() as f64, false, qs.join("; ")));
    out
}

fn describe(m: &Metric) -> String {
    format!(
        "{} [{}] is {}{}{}",
        m.label,
        m.id,
        m.value,
        m.target.map(|t| format!(" (target: {} {t})", if m.at_least { "at least" } else { "at most" })).unwrap_or_default(),
        if m.facts.is_empty() { String::new() } else { format!(". Facts: {}", m.facts) }
    )
}

/// One computation: episodes opened, fixed, noted and reviewed; the page gets the result.
fn tick(app: &Shared) {
    let iso = |s: u64| chrono::DateTime::from_timestamp(s as i64, 0).map(|d| d.format("%Y-%m-%dT%H:%M:%SZ").to_string()).unwrap_or_default();
    let metrics = compute(app);
    let now = now_s();
    let (reviewer, after) = delayed_review(app);
    let (mut due, mut notes) = (vec![], vec![]);
    {
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
        book.last = json!(metrics
            .iter()
            .map(|m| json!({"id": m.id, "label": m.label, "value": m.value, "target": m.target, "at_least": m.at_least, "red": m.red, "since": book.episodes.get(m.id).map(|e| iso(e.since)), "facts": m.facts}))
            .collect::<Vec<_>>());
    }
    // A red main is not reported again here: its project's manager already got it from the merge queue, with the facts.
    notes.into_iter().filter(|m| m.id != "main_red").for_each(|m| {
        let _ = (m.id == "idle").then(|| app.sched_wake.notify_one()); // the mechanical fix: wake the scheduler and the autopilot
        manager::emit_event(app, "", "metric.red", &format!("health: {}. Get it moving.", describe(m)), None);
    });
    let max = app.cfg().factory.role::<u64>(&reviewer, "max_per_day").unwrap_or(6);
    let today: u64 = app.db.lock().unwrap().query_row("SELECT COUNT(*) FROM ceo_reviews WHERE at >= ?1 AND metric != 'owner'", [iso(now.saturating_sub(86400))], |r| r.get(0)).unwrap_or(0);
    let _ = (!due.is_empty() && !reviewer.is_empty() && today < max).then(|| start_review(app, reviewer, due.iter().map(|d| d.0).collect::<Vec<_>>().join(","), due.iter().map(|d| d.1.as_str()).collect::<Vec<_>>().join("\n")));
    app.emit("metrics", payload(app));
}

fn payload(app: &Shared) -> Value {
    json!({"metrics": BOOK.lock().unwrap().last.as_array().cloned().unwrap_or_default(), "reviews": app.recent("ceo_reviews", &["id", "at", "metric", "text", "factory", "role_hash"], None, 10), "reviewing": REVIEWING.load(Ordering::SeqCst)})
}

/// Who reviews a long red stretch, and after how many seconds: the turn role of the delayed `metric.red` route (its `after_minutes`, or the role's `review_after_minutes`); without one, any turn role when asked, and never by itself.
fn delayed_review(app: &Shared) -> (String, u64) {
    let f = &app.cfg().factory;
    match f.routes_on("metric.red").into_iter().find(|r| f.role::<String>(&r.0, "session").as_deref() == Some("turn")) {
        Some(r) => (r.0.clone(), r.4.or_else(|| f.role::<u64>(&r.0, "review_after_minutes")).unwrap_or(15) * 60),
        None => (f.find_role("turn", None).unwrap_or_default(), u64::MAX),
    }
}

fn start_review(app: &Shared, role: String, metric: String, why: String) {
    let Some(()) = (!REVIEWING.swap(true, Ordering::SeqCst)).then_some(()) else { return };
    let app = app.clone();
    tokio::spawn(async move {
        app.emit("review", json!({"running": true}));
        let result = manager::run_turn(&app, &role, "", &brief(&app, &why)).await;
        let text = result.clone().unwrap_or_else(|e| format!("(the review could not run: {e})"));
        let (factory, role_hash) = app.stamp_for(&role); // the review turn's stamp
        let _ = app.db.lock().unwrap().execute("INSERT INTO ceo_reviews (at, metric, text, factory, role_hash) VALUES (?1, ?2, ?3, ?4, ?5)", rusqlite::params![now_iso(), metric, text, factory, role_hash]);
        app.chat(&role, &text, "");
        let _ = result.is_ok().then(|| manager::emit_event(&app, "", "review.done", &format!("Review: {text}"), None));
        REVIEWING.store(false, Ordering::SeqCst);
        app.emit("review", json!({"running": false})); // the page refetches the metrics on it, reviews included
    });
}

/// The facts a review reads: the red metrics, the last lanes, failing checks, plan progress, owner questions.
fn brief(app: &Shared, why: &str) -> String {
    let mut lanes: Vec<Lane> = app.lanes.lock().unwrap().values().cloned().collect();
    lanes.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    let lanes: String = lanes.iter().take(20).map(|l| format!("- {} [{}] {}:{} \"{}\" tries {} created {}: {}\n", l.id, l.status, l.project, l.step, l.item, l.attempts + 1, l.created_at, l.note.lines().next().unwrap_or(""))).collect();
    let day = chrono::DateTime::from_timestamp(now_s().saturating_sub(86400) as i64, 0).map(|d| d.format("%Y-%m-%dT%H:%M:%SZ").to_string()).unwrap_or_default();
    let runs: String = app
        .query("SELECT lane, stage, status, COALESCE(log, '') FROM runs WHERE status IN ('failed', 'timeout') AND started_at >= ?1 ORDER BY id DESC LIMIT 5", [&day], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?, r.get::<_, String>(3)?))
        })
        .iter()
        .map(|(lane, stage, status, log)| format!("- {lane} check '{stage}' {status}:\n```\n{}\n```\n", tail(std::path::Path::new(log), 15)))
        .collect();
    let plans: String = app
        .cfg()
        .projects
        .iter()
        .map(|p| {
            let steps: String =
                plan::read(app, &p.id).unwrap_or_default().iter().map(|st| format!("- step {}. {}: {} open, {} done\n", st.num, st.title, st.items.iter().filter(|i| !i.done).count(), st.items.iter().filter(|i| i.done).count())).collect();
            format!("## {} (autopilot {}, main {})\n{steps}", p.id, if crate::lanes::autopilot_on(app, &p.id) { "on" } else { "off" }, app.kv(&format!("main_red:{}", p.id)).as_deref().unwrap_or("green"))
        })
        .collect();
    let qs: String = owner_questions(app).iter().map(|q| format!("- {q}\n")).collect();
    format!("# Red metrics\n{why}\n\n# Last lanes (newest first)\n{lanes}\n# Failing checks (last 24 h)\n{runs}\n# Plans\n{plans}\n# Open owner questions\n{qs}")
}

pub fn start(app: Shared) {
    let _ = app.db.lock().unwrap().execute_batch("CREATE TABLE IF NOT EXISTS ceo_reviews (id INTEGER PRIMARY KEY, at TEXT, metric TEXT, text TEXT, factory TEXT NOT NULL DEFAULT '', role_hash TEXT NOT NULL DEFAULT '');");
    tokio::spawn(async move {
        loop {
            tick(&app);
            tokio::time::sleep(Duration::from_secs(60)).await;
        }
    });
}

/// The page's "review now": the review turn role runs at once, asked by the owner (it does not count toward its `max_per_day`).
async fn review_now(State(app): State<Shared>) -> crate::app::R {
    let role = delayed_review(&app).0;
    (!role.is_empty()).then_some(()).ok_or_else(|| crate::app::bad("the factory has no turn role to review with".into()))?;
    start_review(&app, role, "owner".into(), "The owner asked for a review now.".into());
    Ok(Json(json!({"ok": true})))
}

pub fn routes() -> Router<Shared> {
    Router::new().route("/api/metrics", get(|State(app): State<Shared>| async move { Json(payload(&app)) })).route("/api/review", axum::routing::post(review_now))
}

#[cfg(test)]
#[path = "../checks/metrics.rs"]
mod tests;
