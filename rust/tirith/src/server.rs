//! HTTP on 127.0.0.1: the page, its JSON API, the change stream (SSE) and the agents' hooks.

use crate::app::{bad, now_s, tail, Shared, R};
use crate::{attach, brain, guard, lanes, manager, merge, metrics, models, plan, projects, rooms, rules, sandbox};
use axum::body::Bytes;
use axum::extract::{Path, Query, Request, State};
use axum::http::{HeaderMap, StatusCode};
use axum::middleware::{self, Next};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::{json, Value};
use std::convert::Infallible;
use std::sync::OnceLock;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::{Stream, StreamExt};

static PORT: OnceLock<u16> = OnceLock::new();
pub fn s(v: &Value, k: &str) -> String {
    v[k].as_str().unwrap_or("").to_string()
}
pub fn router(app: Shared, port: u16) -> Router {
    let _ = PORT.set(port);
    Router::new()
        .route("/api/state", get(state))
        .route("/api/stream", get(stream))
        .route("/api/plan/{project}", get(plan_show).post(plan_add))
        .route("/api/lanes", post(lane_start))
        .route("/api/lanes/{id}/stop", post(lane_stop))
        .route("/api/lanes/{id}/log", get(lane_log))
        .route("/api/lanes/{id}/ask", post(lane_ask))
        .route("/api/lanes/{id}/answer", post(lane_answer))
        .route("/api/owner", post(owner_ask))
        .route("/api/owner/{id}", post(owner_answer))
        .route("/api/autopilot/{project}", post(autopilot))
        .route("/api/pipelines", get(pipelines))
        .route("/api/pipelines/{id}/retry", post(pipeline_retry))
        .route("/api/stage-limit/{project}/{stage}", post(stage_limit))
        .route("/api/runs/{id}/log", get(run_log))
        .route("/api/chat", get(chat_rows).post(chat))
        .route("/api/manager/interrupt", post(manager_interrupt))
        .route("/hook/{event}", post(hook))
        .merge(rooms::routes())
        .merge(sandbox::routes())
        .merge(models::routes())
        .merge(guard::routes())
        .merge(metrics::routes())
        .merge(brain::routes())
        .merge(attach::routes())
        .merge(projects::routes())
        .fallback(web_file)
        .layer(middleware::from_fn(same_origin))
        .with_state(app)
}

/// A browser page from another origin may not call the API (no CSRF against 127.0.0.1).
/// It also checks Host, so a DNS-rebinding page (Host: evil.example) cannot read the API either.
async fn same_origin(req: Request, next: Next) -> Response {
    let port = PORT.get().copied().unwrap_or(0);
    let ok = {
        let h = |k: &str| req.headers().get(k).map(|v| v.to_str().unwrap_or("\u{0}"));
        origin_host_ok(h("host"), h("origin"), port)
    };
    if ok {
        next.run(req).await
    } else {
        (StatusCode::FORBIDDEN, "foreign origin").into_response()
    }
}

fn origin_host_ok(host: Option<&str>, origin: Option<&str>, port: u16) -> bool {
    let host_ok = host.is_none_or(|h| ["127.0.0.1", "localhost"].iter().any(|n| h.eq_ignore_ascii_case(n) || h.eq_ignore_ascii_case(&format!("{n}:{port}"))));
    let origin_ok = origin.is_none_or(|o| o == format!("http://127.0.0.1:{port}") || o == format!("http://localhost:{port}"));
    host_ok && origin_ok
}

#[cfg(test)]
mod tests {
    use super::origin_host_ok;

    #[test]
    fn same_origin_checks_host() {
        let p = 7777;
        // CLI, hooks and the page
        assert!(origin_host_ok(Some("127.0.0.1"), None, p));
        assert!(origin_host_ok(Some("127.0.0.1:7777"), None, p));
        assert!(origin_host_ok(Some("localhost:7777"), Some("http://localhost:7777"), p));
        assert!(origin_host_ok(Some("127.0.0.1:7777"), Some("http://127.0.0.1:7777"), p));
        // DNS rebinding: foreign Host, even with no or a matching-looking Origin
        assert!(!origin_host_ok(Some("evil.example:7777"), None, p));
        assert!(!origin_host_ok(Some("evil.example"), Some("http://evil.example:7777"), p));
        assert!(!origin_host_ok(Some("127.0.0.1.evil.example"), None, p));
        assert!(!origin_host_ok(Some("127.0.0.1:9999"), None, p));
        // foreign Origin with a good Host
        assert!(!origin_host_ok(Some("127.0.0.1:7777"), Some("http://evil.example"), p));
    }
}

async fn state(State(app): State<Shared>) -> Json<Value> {
    let lanes: Vec<Value> = app.lanes.lock().unwrap().values().map(|l| json!(l)).collect();
    Json(json!({
        "projects": projects::list(&app),
        "lanes": lanes,
        "events": app.recent("events", &["id", "at", "lane", "project", "kind", "tool", "summary"], None, 200),
        "owner": manager::owner_open(&app),
        "busy": manager::busy(),
    }))
}

async fn stream(State(app): State<Shared>) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let rx = BroadcastStream::new(app.tx.subscribe());
    Sse::new(rx.filter_map(|m| m.ok().map(|d| Ok(Event::default().data(d))))).keep_alive(KeepAlive::default())
}
async fn plan_show(State(app): State<Shared>, Path(project): Path<String>) -> R {
    let steps = plan::read(&app, &project).map_err(bad)?;
    let lanes = app.lanes.lock().unwrap().clone();
    let item = |num: &str, it: &plan::Item| {
        let lane = lanes.values().filter(|l| l.project == project && l.step == num && l.item == it.text).max_by(|a, b| a.created_at.cmp(&b.created_at));
        json!({"text": it.text, "done": it.done, "check": it.check, "lane": lane.map(|l| json!({"id": l.id, "status": l.status}))})
    };
    let steps: Vec<Value> = steps.iter().map(|st| json!({"num": st.num, "title": st.title, "items": st.items.iter().map(|it| item(&st.num, it)).collect::<Vec<_>>()})).collect();
    Ok(Json(json!({"project": project, "steps": steps, "autopilot": lanes::autopilot_on(&app, &project), "main_red": app.kv(&format!("main_red:{project}"))})))
}

async fn plan_add(State(app): State<Shared>, Path(project): Path<String>, Json(b): Json<Value>) -> R {
    plan::add(&app, &project, &s(&b, "step"), &s(&b, "text")).await.map(|_| Json(json!({"ok": true}))).map_err(bad)
}
async fn lane_start(State(app): State<Shared>, Json(b): Json<Value>) -> R {
    let agent = b["agent"].as_str().filter(|a| !a.is_empty()).map(String::from);
    Ok(Json(json!(lanes::start(&app, &s(&b, "project"), &s(&b, "step"), &s(&b, "item"), agent).map_err(bad)?)))
}
async fn lane_stop(State(app): State<Shared>, Path(id): Path<String>) -> R {
    Ok(Json(json!(lanes::stop(&app, &id).map_err(bad)?)))
}
async fn lane_log(State(app): State<Shared>, Path(id): Path<String>) -> String {
    let safe: String = id.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '-').collect();
    crate::app::redact_secrets(&tail(&app.home.join(format!("logs/lanes/{safe}.log")), 400))
}
async fn lane_ask(State(app): State<Shared>, Path(id): Path<String>, Json(b): Json<Value>) -> R {
    Ok(Json(json!(lanes::ask(&app, &id, &s(&b, "question")).map_err(bad)?)))
}
async fn lane_answer(State(app): State<Shared>, Path(id): Path<String>, Json(b): Json<Value>) -> R {
    Ok(Json(json!(lanes::answer(&app, &id, &s(&b, "answer")).map_err(bad)?)))
}
async fn owner_ask(State(app): State<Shared>, Json(b): Json<Value>) -> R {
    manager::ask_owner(&app, &s(&b, "project"), &s(&b, "kind"), &s(&b, "question")).map(Json).map_err(bad)
}
async fn owner_answer(State(app): State<Shared>, Path(id): Path<String>, Json(b): Json<Value>) -> R {
    manager::owner_answer(&app, &id, &s(&b, "answer")).map(Json).map_err(bad)
}
async fn autopilot(State(app): State<Shared>, Path(project): Path<String>, Json(b): Json<Value>) -> R {
    app.cfg().project(&project).ok_or(bad(format!("unknown project '{project}'")))?;
    let on = b["on"].as_bool().unwrap_or(false);
    app.set_kv(&format!("autopilot:{project}"), Some(if on { "on" } else { "off" }));
    app.emit("autopilot", json!({"project": project, "on": on}));
    app.sched_wake.notify_one();
    Ok(Json(json!({"project": project, "on": on})))
}

async fn pipelines(State(app): State<Shared>, Query(q): Query<std::collections::HashMap<String, String>>) -> Json<Value> {
    let limit = q.get("limit").and_then(|l| l.parse().ok()).unwrap_or(30u32).min(200);
    Json(json!(merge::list(&app, q.get("project").map_or("", String::as_str), limit)))
}
async fn pipeline_retry(State(app): State<Shared>, Path(id): Path<String>) -> R {
    merge::retry(&app, &id).map(|_| Json(json!({"ok": true}))).map_err(bad)
}

/// The manager sets a stage's time limit in minutes; it lasts until changed or 24 h.
async fn stage_limit(State(app): State<Shared>, Path((project, stage)): Path<(String, String)>, Json(b): Json<Value>) -> R {
    let minutes = b["minutes"].as_u64().filter(|m| *m > 0).ok_or_else(|| bad("minutes must be a positive number".into()))?;
    merge::set_stage_limit(&app, &project, &stage, minutes);
    Ok(Json(json!({"project": project, "stage": stage, "minutes": minutes})))
}

async fn run_log(State(app): State<Shared>, Path(id): Path<i64>) -> String {
    let log: Option<String> = app.db.lock().unwrap().query_row("SELECT log FROM runs WHERE id = ?1", [id], |r| r.get(0)).ok().flatten();
    log.map(|l| tail(std::path::Path::new(&l), 400)).unwrap_or_default()
}
async fn chat(State(app): State<Shared>, Json(b): Json<Value>) -> R {
    let files: Vec<String> = b["attachments"].as_array().into_iter().flatten().filter_map(|f| f.as_str().map(String::from)).collect();
    let text = s(&b, "text") + &attach::describe(&files);
    if text.trim().is_empty() {
        return Err(bad("empty message".into()));
    }
    manager::say(&app, &s(&b, "project"), &text).map(|_| Json(json!({"ok": true}))).map_err(bad)
}

/// The last rows of one conversation: `?project=<id>`, or the one of no project.
async fn chat_rows(State(app): State<Shared>, Query(q): Query<std::collections::HashMap<String, String>>) -> Json<Value> {
    Json(json!(app.recent("chat", &["id", "at", "role", "text", "project"], Some(q.get("project").map_or("", String::as_str)), 300)))
}
async fn manager_interrupt(Json(b): Json<Value>) -> R {
    manager::interrupt(&s(&b, "project")).map(|_| Json(json!({"ok": true}))).map_err(bad)
}

/// One agent hook call: record it, refuse a tool call that breaks a rule, hand a lane its live messages, and
/// keep a lane from ending its turn on a question.
async fn hook(State(app): State<Shared>, Path(event): Path<String>, headers: HeaderMap, body: Bytes) -> Json<Value> {
    let p: Value = serde_json::from_slice(&body).unwrap_or(json!({}));
    let cwd = s(&p, "cwd");
    let header = |k: &str| headers.get(k).and_then(|v| v.to_str().ok()).filter(|v| !v.is_empty() && !v.starts_with('$')).map(String::from);
    let lane = header("x-tirith-lane").and_then(|id| app.lane(&id)).or_else(|| app.lane_for_cwd(&cwd));
    let cfg = app.cfg();
    let project = lane.as_ref().and_then(|l| cfg.project(&l.project)).or_else(|| cfg.project_for(&cwd)).cloned();
    let tool = s(&p, "tool_name");
    let session = s(&p, "session_id");
    let summary = if event == "UserPromptSubmit" { rules::summary("", &json!({"prompt": p["prompt"]})) } else { rules::summary(&tool, &p["tool_input"]) };
    let (lane_id, project_id) = (lane.as_ref().map(|l| l.id.clone()).unwrap_or_default(), project.as_ref().map(|p| p.id.clone()).unwrap_or_default());
    if let Some(l) = &lane {
        if !session.is_empty() && l.session != session {
            app.update_lane(&l.id, |x| x.session = session.clone());
        }
        if let Some(m) = app.lanes.lock().unwrap().get_mut(&l.id) {
            m.last_activity = now_s(); // in memory only: activity is not worth a disk write per tool call
        }
    }
    for (tag, target) in sandbox::tags(&tool, &p["tool_input"]) {
        app.event(&lane_id, &project_id, "access", tag, &target, &session);
    }
    if event == "PreToolUse" {
        let unsandboxed = lane.is_none() && header("x-tirith-role").is_none();
        let refusal = rules::check(&app, &tool, &p["tool_input"], lane.as_ref(), project.as_ref()).or_else(|| if unsandboxed { sandbox::check_command(&tool, &p["tool_input"]) } else { None });
        if let Some(why) = refusal {
            app.event(&lane_id, &project_id, "blocked", &tool, &format!("{summary} — {why}"), &session);
            return Json(json!({"hookSpecificOutput": {"hookEventName": "PreToolUse",
                "permissionDecision": "deny", "permissionDecisionReason": why}}));
        }
    }
    if event != "PostToolUse" {
        app.event(&lane_id, &project_id, &event, &tool, &summary, &session);
    }
    let Some(l) = lane else { return Json(json!({})) };
    if event == "PostToolUse" {
        let lines = lanes::drain(&l.id);
        if lines.is_empty() {
            return Json(json!({}));
        }
        if l.status == "waiting" && !l.resume.is_empty() {
            app.update_lane(&l.id, |x| {
                x.status = "running".into(); // the answer reached the live agent
                x.resume.clear();
            });
        }
        return Json(json!({"hookSpecificOutput": {"hookEventName": "PostToolUse", "additionalContext": lines.join("\n")}}));
    }
    if event == "PreToolUse" {
        // In a lane the rules above (with the network sandbox) are the barrier: an allowed call needs no
        // permission prompt, which a headless agent could never answer.
        return Json(json!({"hookSpecificOutput": {"hookEventName": "PreToolUse", "permissionDecision": "allow",
            "permissionDecisionReason": "allowed by Tirith's rules"}}));
    }
    if event == "Stop" {
        let last = s(&p, "last_assistant_message");
        if !last.trim().is_empty() {
            app.update_lane(&l.id, |x| x.note = last.trim().chars().take(2000).collect());
        }
        if l.status == "running" && p["stop_hook_active"].as_bool() != Some(true) && last.trim().ends_with('?') {
            return Json(json!({"decision": "block", "reason":
                "Do not end with a question: decide yourself. If you truly need the manager, run: tirith ask \"<question>\" and end your turn."}));
        }
    }
    Json(json!({}))
}

/// The page: the files of `web/` (`[app] web_dir` of config.toml, else this repository's), read from disk on each
/// request so a change to the page shows on reload. Nothing outside that folder is ever served.
async fn web_file(State(app): State<Shared>, uri: axum::http::Uri) -> Response {
    let rel = uri.path().trim_start_matches('/');
    let rel = if rel.is_empty() { "index.html" } else { rel };
    if rel.split('/').any(|p| p.is_empty() || p == ".." || p.starts_with('.')) {
        return (StatusCode::NOT_FOUND, "not found").into_response();
    }
    let dir = crate::config::get::<String>(&app.cfg().raw, &["app", "web_dir"]).map(|d| crate::config::expand(&d)).unwrap_or_else(|| std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../web")));
    let Ok(body) = tokio::fs::read(dir.join(rel)).await else { return (StatusCode::NOT_FOUND, "not found").into_response() };
    let kind = match rel.rsplit('.').next().unwrap_or("") {
        "html" => "text/html; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "js" | "mjs" => "application/javascript; charset=utf-8",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "json" => "application/json",
        "woff2" => "font/woff2",
        _ => "application/octet-stream",
    };
    ([("content-type", kind), ("cache-control", "no-cache")], body).into_response()
}
