//! Health metrics computed every minute from the daemon's own data, and the turn-role pass that a metric red for long enough starts (the factory's delayed `metric.red` route names the role: the CEO; the factory's `schedule` route to it runs its daily pass, at 07:00, with no red measure); while everything is green and the day's pass ran, no model is called.

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
/// Phone messages of turns no owner message started: a manager message (at, project, text) counts when the last mark (at, project, unasked) of its conversation before it is unasked — an owner or room message asks, a `turn` route with `phone = true` does not — and a ∅ routine silence never reaches the phones.
fn unasked(marks: &[(String, String, bool)], msgs: &[(String, String, String)]) -> usize {
    msgs.iter().filter(|m| !m.2.trim_start().starts_with('∅') && marks.iter().rposition(|k| k.1 == m.1 && k.0 <= m.0).is_some_and(|i| marks[i].2)).count()
}

/// What reached the owner unasked since `since`: (phone messages of unasked turns, notices whose route has `phone = true`, cards opened).
fn interruptions(app: &Shared, since: &str) -> (usize, usize, usize) {
    let f = &app.cfg().factory;
    let phone = |k: &str, chat: bool| f.routes_on(k).iter().any(|r| r.3 && if chat { r.2 } else { r.1 == "turn" });
    let q = |sql: &str| app.query(sql, [since], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?)));
    let evs = q("SELECT at, project, kind FROM events WHERE at >= ?1 AND (kind LIKE '%.%' OR kind = 'schedule') ORDER BY id");
    let asked = |k: &str| matches!(k, "owner.message" | "room.message" | "card.answered"); // a card answer is the owner's word too: the turn it starts was asked for
    let marks: Vec<_> = evs.iter().filter(|e| asked(&e.2) || phone(&e.2, false)).map(|e| (e.0.clone(), e.1.clone(), !asked(&e.2))).collect();
    let cards = app.query("SELECT COUNT(*) FROM owner_items WHERE at >= ?1", [since], |r| r.get::<_, i64>(0)).first().copied().unwrap_or(0) as usize;
    (unasked(&marks, &q("SELECT at, project, text FROM chat WHERE at >= ?1 AND role = 'manager' ORDER BY id")), evs.iter().filter(|e| phone(&e.2, true)).count(), cards)
}
fn compute(app: &Shared) -> Vec<Metric> {
    let cfg = app.cfg();
    let now = now_s() as i64;
    let day = now - 86400;
    let since = chrono::DateTime::from_timestamp(day, 0).map(|d| d.format("%Y-%m-%dT%H:%M:%SZ").to_string()).unwrap_or_default();
    let lanes: Vec<Lane> = app.lanes.lock().unwrap().values().cloned().collect();
    let f = &cfg.factory;
    // A measure with the factory's label, target and side for its id; red when it counts now (`live`) and is on the wrong side of its target.
    let mk = |id: &'static str, value: f64, live: bool, facts: String| {
        let (label, target, at_least) = f.metric(id);
        let red = live && target.is_some_and(|t| if at_least { value < t } else { value > t });
        Metric { id, label, value: (value * 10.0).round() / 10.0, target, at_least, red, facts }
    };
    let secs = |s: &str| chrono::DateTime::parse_from_rfc3339(s).ok().map(|d| d.timestamp());
    // Open items of the projects the autopilot works on: only items it can start (no step-0 role items, no owner-wait items), split into projects with work and projects with none.
    let startable = |p: &crate::config::Project| plan::read(app, &p.id).map(|s| s.iter().flat_map(|s| &s.items).filter(|i| !i.done && !crate::lanes::is_autopilot_skipped(&i.text)).count()).unwrap_or(0);
    let (open, empty): (Vec<(String, usize)>, Vec<_>) = cfg.projects.iter().filter(|p| crate::lanes::autopilot_on(app, &p.id)).map(|p| (p.id.clone(), startable(p))).partition(|(_, n)| *n > 0);
    let empty: Vec<String> = empty.into_iter().map(|(p, _)| p).collect();
    let work = !open.is_empty();
    let merged: Vec<&Lane> = lanes.iter().filter(|l| l.status == "merged" && l.merged_at.as_deref().or(l.ended_at.as_deref()).and_then(secs).is_some_and(|m| m >= day)).collect();
    let ids = |v: &[&Lane]| v.iter().map(|l| l.id.as_str()).collect::<Vec<_>>().join(", ");
    let open_txt = open.iter().map(|(p, n)| format!("{p} {n}")).collect::<Vec<_>>().join(", ");
    let mut out = vec![];

    let why = if empty.is_empty() { "no empty plans: lanes that do not reach main are the problem" } else { "no startable work in the plan (not a lane failure) for:" };
    let paused: String = open.iter().map(|(p, _)| app.kv(&format!("paused:{p}")).map(|w| paused_text(p, &w)).unwrap_or_default()).collect();
    let facts = format!("merged in 24 h: [{}]; open items in autopilot projects: [{open_txt}]; {why}{}{paused}", ids(&merged), empty_plan_note(&empty));
    out.push(mk("merged", merged.len() as f64, work, facts));

    let mut mins: Vec<(f64, &str)> = merged.iter().filter_map(|l| Some(((l.merged_at.as_deref().or(l.ended_at.as_deref()).and_then(secs)? - secs(&l.created_at)?) as f64 / 60.0, l.id.as_str()))).collect();
    mins.sort_by(|a, b| a.0.total_cmp(&b.0));
    let median = if mins.is_empty() { 0.0 } else { (mins[(mins.len() - 1) / 2].0 + mins[mins.len() / 2].0) / 2.0 };
    out.push(mk("median", median, !mins.is_empty(), mins.iter().rev().take(8).map(|(m, id)| format!("{id} {m:.0} min")).collect::<Vec<_>>().join(", ")));

    let pct = if merged.is_empty() { 100.0 } else { (100 * merged.iter().filter(|l| l.attempts == 0).count() / merged.len()) as f64 };
    let retried: Vec<&Lane> = merged.iter().filter(|l| l.attempts > 0).copied().collect();
    out.push(mk("first_try", pct, !merged.is_empty(), format!("merged after being sent back: [{}]", ids(&retried))));

    // Idle: a project with open items, autopilot on and nothing queued, running or merging, and since when.
    let idle: Vec<(String, usize, u64)> = {
        let mut book = BOOK.lock().unwrap();
        let active = |p: &String| lanes.iter().any(|l| &l.project == p && l.active());
        book.idle_since.retain(|p, _| open.iter().any(|(o, _)| o == p) && !active(p));
        open.iter().filter(|(p, _)| !active(p)).map(|(p, n)| (p.clone(), *n, (now as u64 - *book.idle_since.entry(p.clone()).or_insert(now as u64)) / 60)).collect()
    };
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

    let (turns, notices, cards) = interruptions(app, &since);
    let facts = format!("unasked in 24 h: {turns} phone messages of turns no owner message started, {notices} notices with phone = true, {cards} cards, over {} merged items", merged.len());
    out.push(mk("interruptions", (turns + notices + cards) as f64 / merged.len().max(1) as f64, turns + notices + cards > 0, facts));

    // An autoclose («its lane no longer waits») whose question a later card of the same project and kind reopened (its question names the closed card's id) counts nowhere: the same question still waits, under the new card.
    let resolved = app.query("SELECT c.id, c.status FROM owner_items c WHERE c.at >= ?1 AND c.status != 'open' AND NOT (c.status = 'closed' AND COALESCE(c.answer, '') LIKE '%its lane no longer waits%' AND EXISTS (SELECT 1 FROM owner_items s WHERE s.rowid > c.rowid AND s.kind = c.kind AND s.project = c.project AND instr(COALESCE(s.question, ''), c.id) > 0 AND substr(COALESCE(s.question, ''), instr(COALESCE(s.question, ''), c.id) + length(c.id), 1) NOT GLOB '[0-9]' AND substr(COALESCE(s.question, ''), instr(COALESCE(s.question, ''), c.id) - 1, 1) NOT GLOB '[A-Za-z0-9]'))", [&since], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)));
    let closed: Vec<&str> = resolved.iter().filter(|c| c.1 == "closed").map(|c| c.0.as_str()).collect();
    // Plus the resolved cards the card-needed eval judged decidable alone (an answered one too: it bothered the owner needlessly), named by their event's line.
    let judged = app.query("SELECT events.summary FROM scores JOIN events ON events.id = scores.event WHERE scores.eval = 'card-needed' AND scores.label = 'alone' AND scores.at >= ?1", [&since], |r| r.get::<_, String>(0));
    let alone: Vec<String> = judged.iter().filter_map(|s| manager::card_id_of(s)).filter(|id| resolved.iter().any(|c| &c.0 == id) && !closed.contains(&id.as_str())).collect();
    let share = if resolved.is_empty() { 0.0 } else { 100.0 * (closed.len() + alone.len()) as f64 / resolved.len() as f64 };
    out.push(mk("needless_cards", share, !resolved.is_empty(), format!("closed without an answer in 24 h: [{}]; answered {}; judged decidable alone: [{}]", closed.join(", "), resolved.len() - closed.len(), alone.join(", "))));

    // Satisfaction from the owner-mood scores of the last 24 h, per owner message; judged from what the owner writes anyway, never asked.
    let moods = app.query("SELECT label, COUNT(*) FROM scores WHERE eval = 'owner-mood' AND at >= ?1 GROUP BY label", [&since], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)));
    let mood = |label: &str| moods.iter().find(|m| m.0 == label).map(|m| m.1).unwrap_or(0);
    let msgs = app.query("SELECT COUNT(*) FROM events WHERE kind = 'owner.message' AND at >= ?1", [&since], |r| r.get::<_, i64>(0)).into_iter().next().unwrap_or(0);
    let sat = if msgs > 0 { (mood("approval") - mood("correction") - mood("repeat") - mood("noise")) as f64 / msgs as f64 } else { 0.0 };
    let facts =
        format!("owner mood judged in 24 h: {} approvals, {} corrections, {} repeats, {} noise complaints, {} neutral, over {msgs} owner messages", mood("approval"), mood("correction"), mood("repeat"), mood("noise"), mood("neutral"));
    out.push(mk("satisfaction", sat, msgs > 0 && !moods.is_empty(), facts));
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
    let (ceo, after) = delayed_review(app);
    let (mut due, mut notes) = (vec![], vec![]);
    {
        let mut book = BOOK.lock().unwrap();
        book.episodes.retain(|id, _| metrics.iter().any(|m| m.id == id && m.red)); // back to green ends the episode
        for m in metrics.iter().filter(|m| m.red) {
            let fresh = !book.episodes.contains_key(m.id);
            let e = book.episodes.entry(m.id.to_string()).or_insert(Episode { since: now, reviewed: false });
            let _ = fresh.then(|| notes.push(m));
            let _ = (!fresh && !e.reviewed && now >= e.since + after).then(|| (e.reviewed = true, due.push((m.id, describe(m)))).1);
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
    let max = app.cfg().factory.role::<u64>(&ceo, "max_per_day").unwrap_or(6);
    let today = app.db.lock().unwrap().query_row("SELECT COUNT(*) FROM ceo_reviews WHERE at >= ?1 AND metric != 'owner'", [iso(now.saturating_sub(86400))], |r| r.get::<_, u64>(0)).unwrap_or(0);
    let last_daily = app.db.lock().unwrap().query_row("SELECT COALESCE(MAX(at), '') FROM ceo_reviews WHERE metric LIKE '%daily%'", [], |r| r.get::<_, String>(0)).unwrap_or_default();
    // The daily pass is configuration, not a decision here: the factory's `schedule` route to this role, due like a bot's routine when its time came today and no daily pass (kept as rows naming it) ran since.
    let daily = app.cfg().factory.schedules().into_iter().any(|(to, at, days)| to == ceo && crate::bots::due(&json!({"at": at, "days": days, "last": last_daily}), chrono::Local::now()));
    let _ = daily.then(|| due.push(("daily", format!("Scheduled CEO pass (no red measure needed).\n{}", metrics.iter().map(describe).collect::<Vec<_>>().join("\n")))));
    let _ = (!due.is_empty() && !ceo.is_empty() && today < max).then(|| start_review(app, ceo, due.iter().map(|d| d.0).collect::<Vec<_>>().join(","), due.iter().map(|d| d.1.as_str()).collect::<Vec<_>>().join("\n")));
    app.emit("metrics", payload(app));
}

fn payload(app: &Shared) -> Value {
    json!({"metrics": BOOK.lock().unwrap().last.as_array().cloned().unwrap_or_default(), "reviews": app.recent("ceo_reviews", &["id", "at", "metric", "text", "factory", "role_hash"], None, 10), "reviewing": REVIEWING.load(Ordering::SeqCst)})
}

/// Who reviews a long red stretch, and after how many seconds: the turn role of the delayed `metric.red` route (its `after_minutes`, or the role's `review_after_minutes`); without one, any turn role when asked, and never by itself.
fn delayed_review(app: &Shared) -> (String, u64) {
    let f = &app.cfg().factory;
    f.routes_on("metric.red")
        .into_iter()
        .find(|r| f.role::<String>(&r.0, "session").as_deref() == Some("turn"))
        .map_or_else(|| (f.find_role("turn", None).unwrap_or_default(), u64::MAX), |r| (r.0.clone(), r.4.or_else(|| f.role::<u64>(&r.0, "review_after_minutes")).unwrap_or(15) * 60))
}

fn start_review(app: &Shared, role: String, metric: String, why: String) {
    let Some(()) = (!REVIEWING.swap(true, Ordering::SeqCst)).then_some(()) else { return };
    let app = app.clone();
    tokio::spawn(async move {
        app.emit("review", json!({"running": true}));
        let result = manager::run_turn(&app, &role, &role, &brief(&app, &why)).await;
        let text = result.clone().unwrap_or_else(|e| format!("(the review could not run: {e})"));
        let (factory, role_hash) = app.stamp_for(&role); // the review turn's stamp
        let _ = app.db.lock().unwrap().execute("INSERT INTO ceo_reviews (at, metric, text, factory, role_hash) VALUES (?1, ?2, ?3, ?4, ?5)", rusqlite::params![now_iso(), metric, text, factory, role_hash]);
        app.chat("manager", &text, &role); // the review appears in the CEO's own conversation, as its answer there
        let _ = result.is_ok().then(|| manager::emit_event(&app, &role, "review.done", &format!("Review: {text}"), None));
        REVIEWING.store(false, Ordering::SeqCst);
        app.emit("review", json!({"running": false})); // the page refetches the metrics on it, reviews included
    });
}

/// The facts a CEO pass reads: the red metrics (or all of them, on the scheduled daily pass), the measures and eval scores by factory version (the stamps), the last lanes with their stamp, failing checks, plan progress, owner questions.
fn brief(app: &Shared, why: &str) -> String {
    let mut lanes: Vec<Lane> = app.lanes.lock().unwrap().values().cloned().collect();
    lanes.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    let lanes: String = lanes.iter().take(30).map(|l| format!("- {} [{}] {}:{} {} tries {} {} {}: {}\n", l.id, l.status, l.project, l.step, l.item, l.attempts + 1, l.created_at, l.factory, l.note.lines().next().unwrap_or(""))).collect();
    let day = chrono::DateTime::from_timestamp(now_s().saturating_sub(86400) as i64, 0).map(|d| d.format("%Y-%m-%dT%H:%M:%SZ").to_string()).unwrap_or_default();
    let runs: String = app
        .objects("SELECT lane, stage, status, COALESCE(log, '') FROM runs WHERE status IN ('failed', 'timeout') AND started_at >= ?1 ORDER BY id DESC LIMIT 5", [&day], &["lane", "stage", "status", "log"])
        .iter()
        .map(|r| format!("- {} check '{}' {}:\n```\n{}\n```\n", r["lane"].as_str().unwrap_or(""), r["stage"].as_str().unwrap_or(""), r["status"].as_str().unwrap_or(""), tail(std::path::Path::new(r["log"].as_str().unwrap_or("")), 15)))
        .collect();
    let grouped = |sql: &str, cols: &[&str]| app.objects(sql, [], cols).iter().map(|r| format!("- {r}\n")).collect::<String>(); // by factory version (the stamps), so two versions compare here
    let versions = grouped("SELECT factory, COUNT(*) AS merged, CAST(100 * AVG(json_extract(data, '$.attempts') = 0) AS INTEGER) AS first_try_pct, MIN(json_extract(data, '$.created_at')) AS first_lane FROM lanes WHERE factory != '' AND json_extract(data, '$.status') = 'merged' GROUP BY factory ORDER BY first_lane", &["factory", "merged", "first_try_pct", "first_lane"]);
    let scores = grouped("SELECT factory, eval, label, COUNT(*) AS n FROM scores WHERE factory != '' GROUP BY factory, eval, label ORDER BY factory, eval, label", &["factory", "eval", "label", "n"]);
    let stamped = grouped("SELECT factory, 'card.opened' AS kind, COUNT(*) AS n FROM owner_items WHERE factory != '' GROUP BY factory UNION ALL SELECT factory, kind, COUNT(*) AS n FROM events WHERE factory != '' GROUP BY factory, kind ORDER BY factory, kind", &["factory", "kind", "n"]);
    let cfg = app.cfg();
    let plans: String = cfg
        .projects
        .iter()
        .map(|p| {
            let steps: String =
                plan::read(app, &p.id).unwrap_or_default().iter().map(|st| format!("- step {}. {}: {} open, {} done\n", st.num, st.title, st.items.iter().filter(|i| !i.done).count(), st.items.iter().filter(|i| i.done).count())).collect();
            format!("## {} (repo {}, autopilot {}, main {})\n{steps}", p.id, p.repo().display(), if crate::lanes::autopilot_on(app, &p.id) { "on" } else { "off" }, app.kv(&format!("main_red:{}", p.id)).as_deref().unwrap_or("green"))
        })
        .collect();
    let qs: String = owner_questions(app).iter().map(|q| format!("- {q}\n")).collect();
    format!("# Red metrics\n{why}\n\n# By factory version (oldest first)\n{versions}# Eval scores by factory version\n{scores}# Cards and events by factory version and kind\n{stamped}\n# Last lanes (newest first: id, status, project:step, item, tries, created, factory stamp, note)\n{lanes}\n# Failing checks (last 24 h)\n{runs}\n# Plans\n{plans}\n# Open owner questions\n{qs}")
}

pub fn start(app: Shared) {
    tokio::spawn(async move {
        loop {
            tick(&app);
            tokio::time::sleep(Duration::from_secs(60)).await;
        }
    });
}

/// The page's "CEO now": the CEO turn role runs at once, asked by the owner (it does not count toward its `max_per_day`).
async fn review_now(State(app): State<Shared>) -> crate::app::R {
    let role = delayed_review(&app).0;
    (!role.is_empty()).then_some(()).ok_or_else(|| crate::app::bad("the factory has no turn role to review with".into()))?;
    start_review(&app, role, "owner".into(), "The owner asked for a CEO pass now.".into());
    Ok(Json(json!({"ok": true})))
}

pub fn routes() -> Router<Shared> {
    Router::new().route("/api/metrics", get(|State(app): State<Shared>| async move { Json(payload(&app)) })).route("/api/review", axum::routing::post(review_now))
}

#[cfg(test)]
#[path = "../checks/metrics.rs"]
mod tests;
