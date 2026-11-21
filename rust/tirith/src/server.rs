//! HTTP on 127.0.0.1: the page, its JSON API, the change stream (SSE) and the agents' hooks.

use crate::app::{bad, now_s, tail, Shared, R};
use crate::{attach, bots, guard, lanes, manager, merge, metrics, models, plan, projects, rooms, rules, sandbox};
use axum::extract::{Path, Query, Request, State};
use axum::http::{HeaderMap, StatusCode};
use axum::middleware::{self, Next};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{body::Bytes, Json, Router};
use serde_json::{json, Value};
use std::{convert::Infallible, sync::OnceLock};
use tokio_stream::{wrappers::BroadcastStream, StreamExt};

static PORT: OnceLock<u16> = OnceLock::new();
pub fn s(v: &Value, k: &str) -> String {
    v[k].as_str().unwrap_or("").to_string()
}
pub fn router(app: Shared, port: u16) -> Router {
    let _ = PORT.set(port);
    Router::new()
        .route("/api/state", get(state))
        .route("/api/stream", get(|State(app): State<Shared>| async move { Sse::new(BroadcastStream::new(app.tx.subscribe()).filter_map(|m| m.ok().map(|d| Ok::<Event, Infallible>(Event::default().data(d))))).keep_alive(KeepAlive::default()) }))
        .route(
            "/api/plan/{project}",
            get(plan_show).post(|State(app): State<Shared>, Path(project): Path<String>, Json(b): Json<Value>| async move { plan::add(&app, &project, &s(&b, "step"), &s(&b, "text")).await.map(|_| Json(json!({"ok": true}))).map_err(bad) }),
        )
        .route("/api/lanes", post(|State(app): State<Shared>, Json(b): Json<Value>| async move { lanes::start(&app, &s(&b, "project"), &s(&b, "step"), &s(&b, "item"), b["agent"].as_str().filter(|a| !a.is_empty()).map(String::from)).map(|l| Json(json!(l))).map_err(bad) }))
        .route("/api/lanes/{id}/stop", post(|State(app): State<Shared>, Path(id): Path<String>| async move { lanes::stop(&app, &id).map(|l| Json(json!(l))).map_err(bad) }))
        .route("/api/lanes/{id}/delete", post(|State(app): State<Shared>, Path(id): Path<String>| async move { lanes::delete(&app, &id).await.map(|_| Json(json!({"ok": true}))).map_err(bad) }))
        .route(
            "/api/lanes/{id}/log",
            get(|State(app): State<Shared>, Path(id): Path<String>| async move {
                crate::app::redact_secrets(&tail(&app.home.join(format!("logs/lanes/{}.log", id.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '-').collect::<String>())), 400))
            }),
        )
        .route("/api/lanes/{id}/ask", post(|State(app): State<Shared>, Path(id): Path<String>, Json(b): Json<Value>| async move { lanes::ask(&app, &id, &s(&b, "question")).map(|l| Json(json!(l))).map_err(bad) }))
        .route("/api/lanes/{id}/answer", post(|State(app): State<Shared>, Path(id): Path<String>, Json(b): Json<Value>| async move { lanes::answer(&app, &id, &s(&b, "answer"), "the manager").map(|l| Json(json!(l))).map_err(bad) }))
        .route("/api/owner", post(|State(app): State<Shared>, Json(b): Json<Value>| async move { manager::ask_owner(&app, &s(&b, "project"), &s(&b, "kind"), &s(&b, "question"), &s(&b, "lane")).map(Json).map_err(bad) }))
        .route("/api/owner/{id}", post(|State(app): State<Shared>, Path(id): Path<String>, Json(b): Json<Value>| async move { manager::owner_answer(&app, &id, &s(&b, "answer"), s(&b, "by") == "manager").map(Json).map_err(bad) }))
        .route("/api/owner/{id}/close", post(|State(app): State<Shared>, Path(id): Path<String>, Json(b): Json<Value>| async move { manager::close_card(&app, &id, if s(&b, "by") == "manager" { "the manager" } else { "the owner" }, &s(&b, "why")).map(Json).map_err(bad) }))
        .route("/api/autopilot/{project}", post(autopilot))
        .route(
            "/api/pipelines",
            get(|State(app): State<Shared>, Query(q): Query<std::collections::HashMap<String, String>>| async move {
                Json(json!(merge::list(&app, q.get("project").map_or("", String::as_str), q.get("limit").and_then(|l| l.parse().ok()).unwrap_or(30u32).min(200))))
            }),
        )
        .route("/api/pipelines/{id}/retry", post(|State(app): State<Shared>, Path(id): Path<String>| async move { merge::retry(&app, &id).map(|_| Json(json!({"ok": true}))).map_err(bad) }))
        .route("/api/stage-limit/{project}/{stage}", post(stage_limit))
        .route(
            "/api/runs/{id}/log",
            get(|State(app): State<Shared>, Path(id): Path<i64>| async move {
                app.db.lock().unwrap().query_row("SELECT log FROM runs WHERE id = ?1", [id], |r| r.get::<_, Option<String>>(0)).ok().flatten().map(|l| tail(std::path::Path::new(&l), 400)).unwrap_or_default()
            }),
        )
        // The last rows of one conversation: `?project=<id>`, or the one of no project.
        .route(
            "/api/chat",
            get(|State(app): State<Shared>, Query(q): Query<std::collections::HashMap<String, String>>| async move {
                Json(json!(app.recent("chat", &["id", "at", "role", "text", "project", "factory", "role_hash"], Some(app.conv(q.get("project").map_or("", String::as_str))), 300)))
            })
            .post(chat),
        )
        .route("/api/manager/interrupt", post(|State(app): State<Shared>, Json(b): Json<Value>| async move { manager::interrupt(app.conv(&s(&b, "project"))).map(|_| Json(json!({"ok": true}))).map_err(bad) }))
        .route("/hook/{event}", post(hook))
        .merge(rooms::routes())
        .merge(sandbox::routes())
        .merge(models::routes())
        .merge(guard::routes())
        .merge(metrics::routes())
        .merge(attach::routes())
        .merge(projects::routes())
        .merge(bots::routes())
        .fallback(web_file)
        .layer(middleware::from_fn(same_origin))
        .with_state(app)
}

/// A browser page from another origin may not call the API (no CSRF against 127.0.0.1). It also checks Host, so a DNS-rebinding page (Host: evil.example) cannot read the API either.
async fn same_origin(req: Request, next: Next) -> Response {
    let ok = {
        let h = |k: &str| req.headers().get(k).map(|v| v.to_str().unwrap_or("\u{0}"));
        origin_host_ok(h("host"), h("origin"), PORT.get().copied().unwrap_or(0))
    };
    match ok {
        true => next.run(req).await,
        false => (StatusCode::FORBIDDEN, "foreign origin").into_response(),
    }
}

fn origin_host_ok(host: Option<&str>, origin: Option<&str>, port: u16) -> bool {
    let host_ok = host.is_none_or(|h| ["127.0.0.1", "localhost"].iter().any(|n| h.eq_ignore_ascii_case(n) || h.eq_ignore_ascii_case(&format!("{n}:{port}"))));
    let origin_ok = origin.is_none_or(|o| o == format!("http://127.0.0.1:{port}") || o == format!("http://localhost:{port}"));
    host_ok && origin_ok
}

#[cfg(test)]
#[path = "../checks/server.rs"]
mod tests;

async fn state(State(app): State<Shared>) -> Json<Value> {
    let lanes: Vec<Value> = app.lanes.lock().unwrap().values().map(|l| json!(l)).collect();
    Json(json!({
        "projects": projects::list(&app),
        "lanes": lanes,
        "events": app.recent("events", &["id", "at", "lane", "project", "kind", "tool", "summary", "factory", "role_hash"], None, 200),
        "owner": manager::owner_open(&app),
        "busy": manager::busy(),
        "factory_error": app.cfg().error,
    }))
}

async fn plan_show(State(app): State<Shared>, Path(project): Path<String>) -> R {
    let steps = plan::read(&app, &project).map_err(bad)?;
    let lanes = app.lanes.lock().unwrap().clone();
    let item = |num: &str, it: &plan::Item| {
        let lane = lanes.values().filter(|l| l.project == project && l.step == num && plan::same(&l.item, &it.text)).max_by(|a, b| a.created_at.cmp(&b.created_at));
        json!({"text": it.text, "done": it.done, "check": it.check, "lane": lane.map(|l| json!({"id": l.id, "status": l.status}))})
    };
    let steps: Vec<Value> = steps.iter().map(|st| json!({"num": st.num, "title": st.title, "items": st.items.iter().map(|it| item(&st.num, it)).collect::<Vec<_>>()})).collect();
    Ok(Json(json!({"project": project, "steps": steps, "autopilot": lanes::autopilot_on(&app, &project), "main_red": app.kv(&format!("main_red:{project}"))})))
}

async fn autopilot(State(app): State<Shared>, Path(project): Path<String>, Json(b): Json<Value>) -> R {
    app.cfg().project(&project).ok_or(bad(format!("unknown project '{project}'")))?;
    let on = b["on"].as_bool().unwrap_or(false);
    app.set_kv(&format!("autopilot:{project}"), Some(if on { "on" } else { "off" }));
    app.emit("autopilot", json!({"project": project, "on": on}));
    app.sched_wake.notify_one();
    Ok(Json(json!({"project": project, "on": on})))
}

/// The manager sets a stage's time limit in minutes; it lasts until changed or 24 h.
async fn stage_limit(State(app): State<Shared>, Path((project, stage)): Path<(String, String)>, Json(b): Json<Value>) -> R {
    let minutes = b["minutes"].as_u64().filter(|m| *m > 0).ok_or_else(|| bad("minutes must be a positive number".into()))?;
    app.set_kv(&format!("stage_limit:{project}:{stage}"), Some(&format!("{minutes}@{}", now_s())));
    Ok(Json(json!({"project": project, "stage": stage, "minutes": minutes})))
}

async fn chat(State(app): State<Shared>, headers: HeaderMap, Json(b): Json<Value>) -> R {
    let text = s(&b, "text") + &attach::describe(&b["attachments"].as_array().into_iter().flatten().filter_map(|f| f.as_str().map(String::from)).collect::<Vec<_>>());
    (!text.trim().is_empty()).then_some(()).ok_or_else(|| bad("empty message".into()))?;
    let agent = ["x-tirith-lane", "x-tirith-role"].iter().find_map(|k| headers.get(*k).and_then(|v| v.to_str().ok()).filter(|v| !v.is_empty() && !v.starts_with('$'))).unwrap_or("");
    manager::say(&app, &s(&b, "project"), &text, agent).map(|_| Json(json!({"ok": true}))).map_err(bad)
}

/// One agent hook call: record it, refuse a tool call that breaks a rule, hand a lane its live messages, and keep a lane from ending its turn on a question.
async fn hook(State(app): State<Shared>, Path(event): Path<String>, headers: HeaderMap, body: Bytes) -> Json<Value> {
    let p: Value = serde_json::from_slice(&body).unwrap_or(json!({}));
    let cwd = s(&p, "cwd");
    let header = |k: &str| headers.get(k).and_then(|v| v.to_str().ok()).filter(|v| !v.is_empty() && !v.starts_with('$')).map(String::from);
    // Without the header, the working lane whose worktree holds the agent's cwd.
    let in_worktree = |l: &&crate::app::Lane| matches!(l.status.as_str(), "running" | "waiting") && !l.workspace.is_empty() && std::path::Path::new(&cwd).starts_with(&l.workspace);
    let lane = header("x-tirith-lane").and_then(|id| app.lane(&id)).or_else(|| app.lanes.lock().unwrap().values().find(in_worktree).cloned());
    let cfg = app.cfg();
    let project = lane
        .as_ref()
        .and_then(|l| cfg.project(&l.project))
        .or_else(|| cfg.projects.iter().filter(|p| !p.repo.is_empty()).find(|p| std::path::Path::new(&cwd).starts_with(p.repo()) || std::path::Path::new(&cwd).starts_with(p.lanes_dir())))
        .cloned();
    let tool = s(&p, "tool_name");
    let session = s(&p, "session_id");
    let summary = if event == "UserPromptSubmit" { rules::summary("", &json!({"prompt": p["prompt"]})) } else { rules::summary(&tool, &p["tool_input"]) };
    let (lane_id, project_id) = (lane.as_ref().map(|l| l.id.clone()).unwrap_or_default(), project.as_ref().map(|p| p.id.clone()).unwrap_or_default());
    if let Some(l) = &lane {
        drop((!session.is_empty() && l.session != session).then(|| app.update_lane(&l.id, |x| x.session = session.clone())));
        app.lanes.lock().unwrap().get_mut(&l.id).into_iter().for_each(|m| m.last_activity = now_s());
        // in memory only: activity is not worth a disk write per tool call
    }
    sandbox::tags(&tool, &p["tool_input"]).into_iter().for_each(|(tag, target)| _ = app.event(&lane_id, &project_id, "access", tag, &target, &session));
    if event == "PreToolUse" {
        let unsandboxed = lane.is_none() && header("x-tirith-role").is_none();
        let refusal = rules::check(&app, &tool, &p["tool_input"], lane.as_ref(), project.as_ref()).or_else(|| if unsandboxed { sandbox::check_command(&tool, &p["tool_input"]) } else { None });
        if let Some(why) = refusal {
            app.event(&lane_id, &project_id, "blocked", &tool, &format!("{summary} — {why}"), &session);
            return Json(json!({"hookSpecificOutput": {"hookEventName": "PreToolUse",
                "permissionDecision": "deny", "permissionDecisionReason": why}}));
        }
    }
    let _ = (event != "PostToolUse").then(|| app.event(&lane_id, &project_id, &event, &tool, &summary, &session));
    let Some(l) = lane else { return Json(json!({})) };
    if event == "PostToolUse" {
        let lines = lanes::drain(&l.id);
        let Some(lines) = (!lines.is_empty()).then_some(lines) else { return Json(json!({})) };
        let _ = (l.status == "waiting" && !l.resume.is_empty()).then(|| app.update_lane(&l.id, |x| (x.status, x.resume) = ("running".into(), String::new()))); // the answer reached the live agent
        return Json(json!({"hookSpecificOutput": {"hookEventName": "PostToolUse", "additionalContext": lines.join("\n")}}));
    }
    if event == "PreToolUse" {
        // In a lane the rules above (with the network sandbox) are the barrier: an allowed call needs no permission prompt, which a headless agent could never answer.
        return Json(json!({"hookSpecificOutput": {"hookEventName": "PreToolUse", "permissionDecision": "allow",
            "permissionDecisionReason": "allowed by Tirith's rules"}}));
    }
    if event == "Stop" {
        let last = s(&p, "last_assistant_message");
        drop((!last.trim().is_empty()).then(|| app.update_lane(&l.id, |x| x.note = last.trim().chars().take(2000).collect())));
        if l.status == "running" && p["stop_hook_active"].as_bool() != Some(true) && last.trim().ends_with('?') {
            return Json(json!({"decision": "block", "reason":
                "Do not end with a question: decide yourself. If you truly need the manager, run: tirith ask \"<question>\" and end your turn."}));
        }
    }
    Json(json!({}))
}

/// The page: the files of `web/` (`[app] web_dir` of config.toml, else this repository's), read from disk on each request so a change to the page shows on reload. Nothing outside that folder is ever served.
async fn web_file(State(app): State<Shared>, uri: axum::http::Uri) -> Response {
    let rel = Some(uri.path().trim_start_matches('/')).filter(|r| !r.is_empty()).unwrap_or("index.html");
    let Some(()) = (!rel.split('/').any(|p| p.is_empty() || p == ".." || p.starts_with('.'))).then_some(()) else { return (StatusCode::NOT_FOUND, "not found").into_response() };
    let dir = crate::config::get::<String>(&app.cfg().raw, &["app", "web_dir"]).map(|d| crate::config::expand(&d)).unwrap_or_else(|| std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../web")));
    let Ok(body) = tokio::fs::read(dir.join(rel)).await else { return (StatusCode::NOT_FOUND, "not found").into_response() };
    let kind = attach::content_kind(rel, true);
    ([("content-type", kind), ("cache-control", "no-cache")], body).into_response()
}
