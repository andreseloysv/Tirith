//! HTTP on 127.0.0.1: the page, its JSON API, the change stream (SSE) and the agents' hooks.

use crate::app::{now_s, tail, Shared};
use crate::{attach, brain, guard, lanes, manager, metrics, models, plan, rooms, rules, sandbox};
use axum::body::Bytes;
use axum::extract::{Path, Request, State};
use axum::http::{HeaderMap, StatusCode};
use axum::middleware::{self, Next};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::{json, Value};
use std::convert::Infallible;
use std::sync::OnceLock;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::{Stream, StreamExt};

static PORT: OnceLock<u16> = OnceLock::new();
type R = Result<Json<Value>, (StatusCode, String)>;

fn bad(e: String) -> (StatusCode, String) {
    (StatusCode::BAD_REQUEST, e)
}

fn s(v: &Value, k: &str) -> String {
    v[k].as_str().unwrap_or("").to_string()
}

pub fn router(app: Shared, port: u16) -> Router {
    let _ = PORT.set(port);
    Router::new()
        .route("/", get(|| async { Html(include_str!("../ui/index.html")) }))
        .route("/api/state", get(state))
        .route("/api/stream", get(stream))
        .route("/api/plan/{project}", get(plan_show).post(plan_add))
        .route("/api/lanes", post(lane_start))
        .route("/api/lanes/{id}/stop", post(lane_stop))
        .route("/api/lanes/{id}/report", post(lane_report))
        .route("/api/lanes/{id}/log", get(lane_log))
        .route("/api/chat", post(chat))
        .route("/hook/{event}", post(hook))
        .route("/ui/{name}", get(ui_file))
        .merge(rooms::routes())
        .merge(sandbox::routes())
        .merge(models::routes())
        .merge(guard::routes())
        .merge(metrics::routes())
        .merge(brain::routes())
        .merge(attach::routes())
        .layer(middleware::from_fn(same_origin))
        .with_state(app)
}

/// A browser page from another origin may not call the API (no CSRF against 127.0.0.1).
async fn same_origin(req: Request, next: Next) -> Response {
    let port = PORT.get().copied().unwrap_or(0);
    let ok = match req.headers().get("origin").and_then(|o| o.to_str().ok()) {
        None => true,
        Some(o) => o == format!("http://127.0.0.1:{port}") || o == format!("http://localhost:{port}"),
    };
    if ok { next.run(req).await } else { (StatusCode::FORBIDDEN, "foreign origin").into_response() }
}

async fn state(State(app): State<Shared>) -> Json<Value> {
    let cfg = app.cfg();
    let projects: Vec<Value> = cfg.projects.iter().map(|p| json!({"id": p.id, "name": p.name, "color": p.color})).collect();
    let lanes: Vec<Value> = app.lanes.lock().unwrap().values().map(|l| json!(l)).collect();
    Json(json!({
        "projects": projects,
        "lanes": lanes,
        "chat": app.recent("chat", &["id", "at", "role", "text"], 300),
        "events": app.recent("events", &["id", "at", "lane", "project", "kind", "tool", "summary"], 200),
        "runs": app.recent("runs", &["id", "lane", "project", "stage", "status", "started_at", "ended_at", "log"], 40),
        "busy": false,
    }))
}

async fn stream(State(app): State<Shared>) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let rx = BroadcastStream::new(app.tx.subscribe());
    Sse::new(rx.filter_map(|m| m.ok().map(|d| Ok(Event::default().data(d))))).keep_alive(KeepAlive::default())
}

async fn plan_show(State(app): State<Shared>, Path(project): Path<String>) -> R {
    let steps = plan::read(&app, &project).map_err(bad)?;
    let lanes = app.lanes.lock().unwrap().clone();
    let steps: Vec<Value> = steps
        .iter()
        .map(|st| {
            let items: Vec<Value> = st
                .items
                .iter()
                .map(|it| {
                    let lane = lanes
                        .values()
                        .filter(|l| l.project == project && l.step == st.num && l.item == it.text)
                        .max_by(|a, b| a.created_at.cmp(&b.created_at))
                        .map(|l| json!({"id": l.id, "status": l.status}));
                    json!({"text": it.text, "done": it.done, "lane": lane})
                })
                .collect();
            json!({"num": st.num, "title": st.title, "items": items})
        })
        .collect();
    Ok(Json(json!({"project": project, "steps": steps})))
}

async fn plan_add(State(app): State<Shared>, Path(project): Path<String>, Json(b): Json<Value>) -> R {
    plan::add(&app, &project, &s(&b, "step"), &s(&b, "text")).await.map_err(bad)?;
    Ok(Json(json!({"ok": true})))
}

async fn lane_start(State(app): State<Shared>, Json(b): Json<Value>) -> R {
    let agent = b["agent"].as_str().filter(|a| !a.is_empty()).map(String::from);
    let lane = lanes::start(&app, &s(&b, "project"), &s(&b, "step"), &s(&b, "item"), agent).map_err(bad)?;
    Ok(Json(json!(lane)))
}

async fn lane_stop(State(app): State<Shared>, Path(id): Path<String>) -> R {
    Ok(Json(json!(lanes::stop(&app, &id).map_err(bad)?)))
}

async fn lane_report(State(app): State<Shared>, Path(id): Path<String>, Json(b): Json<Value>) -> R {
    Ok(Json(json!(lanes::report(&app, &id, &s(&b, "status"), &s(&b, "note")).map_err(bad)?)))
}

async fn lane_log(State(app): State<Shared>, Path(id): Path<String>) -> String {
    let safe: String = id.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '-').collect();
    tail(&app.home.join(format!("logs/lanes/{safe}.log")), 400)
}

async fn chat(State(app): State<Shared>, Json(b): Json<Value>) -> R {
    let files: Vec<String> = b["attachments"].as_array().into_iter().flatten().filter_map(|f| f.as_str().map(String::from)).collect();
    let text = s(&b, "text") + &attach::describe(&files);
    if text.trim().is_empty() {
        return Err(bad("empty message".into()));
    }
    manager::say(&app, &text);
    Ok(Json(json!({"ok": true})))
}

/// One agent hook call: record it, refuse a tool call that breaks a rule, hold a lane's Stop until it reports.
async fn hook(State(app): State<Shared>, Path(event): Path<String>, headers: HeaderMap, body: Bytes) -> Json<Value> {
    let p: Value = serde_json::from_slice(&body).unwrap_or(json!({}));
    let cwd = s(&p, "cwd");
    let header = |k: &str| headers.get(k).and_then(|v| v.to_str().ok()).filter(|v| !v.is_empty() && !v.starts_with('$')).map(String::from);
    let lane = header("x-tirith-lane").and_then(|id| app.lane(&id)).or_else(|| app.lane_for_cwd(&cwd));
    let cfg = app.cfg();
    let project = lane.as_ref().and_then(|l| cfg.project(&l.project)).or_else(|| cfg.project_for(&cwd)).cloned();
    let tool = s(&p, "tool_name");
    let summary = if event == "UserPromptSubmit" { rules::summary("", &json!({"prompt": p["prompt"]})) } else { rules::summary(&tool, &p["tool_input"]) };
    let (lane_id, project_id) = (lane.as_ref().map(|l| l.id.clone()).unwrap_or_default(), project.as_ref().map(|p| p.id.clone()).unwrap_or_default());
    if let Some(l) = &lane {
        if let Some(m) = app.lanes.lock().unwrap().get_mut(&l.id) {
            m.last_activity = now_s(); // in memory only: activity is not worth a disk write per tool call
        }
    }
    for (tag, target) in sandbox::tags(&tool, &p["tool_input"]) {
        app.event(&lane_id, &project_id, "access", tag, &target, &s(&p, "session_id"));
    }
    if event == "PreToolUse" {
        let unsandboxed = lane.is_none() && header("x-tirith-role").is_none();
        let refusal = rules::check(&app, &tool, &p["tool_input"], lane.as_ref(), project.as_ref())
            .or_else(|| if unsandboxed { sandbox::check_command(&app, &tool, &p["tool_input"]) } else { None });
        if let Some(why) = refusal {
            app.event(&lane_id, &project_id, "blocked", &tool, &format!("{summary} — {why}"), &s(&p, "session_id"));
            return Json(json!({"hookSpecificOutput": {"hookEventName": "PreToolUse",
                "permissionDecision": "deny", "permissionDecisionReason": why}}));
        }
    }
    if event != "PostToolUse" {
        app.event(&lane_id, &project_id, &event, &tool, &summary, &s(&p, "session_id"));
    }
    if event == "Stop" {
        if let Some(l) = lane.filter(|l| l.report.is_none() && l.status == "running") {
            if p["stop_hook_active"].as_bool() != Some(true) {
                return Json(json!({"decision": "block", "reason": format!(
                    "Report your outcome before you stop: tirith report --status done|partial|blocked --note \"<what you did or why not>\" (lane {}).", l.id)}));
            }
        }
    }
    Json(json!({}))
}

/// The page's module scripts, compiled into the binary like the page itself.
async fn ui_file(Path(name): Path<String>) -> Response {
    let js = match name.as_str() {
        "rooms.js" => include_str!("../ui/rooms.js"),
        "net.js" => include_str!("../ui/net.js"),
        "models.js" => include_str!("../ui/models.js"),
        "health.js" => include_str!("../ui/health.js"),
        "pipelines.js" => include_str!("../ui/pipelines.js"),
        "attach.js" => include_str!("../ui/attach.js"),
        "guard.js" => include_str!("../ui/guard.js"),
        _ => return (StatusCode::NOT_FOUND, "no such script").into_response(),
    };
    ([("content-type", "application/javascript; charset=utf-8")], js).into_response()
}
