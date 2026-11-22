//! A project's plan as the page shows it (each item with its newest lane), and the step a command adds to it.
use crate::app::{bad, Shared, R};
use crate::{lanes, plan};
use axum::extract::{Path, State};
use axum::Json;
use serde_json::{json, Value};

pub(super) async fn plan_show(State(app): State<Shared>, Path(project): Path<String>) -> R {
    let steps = plan::read(&app, &project).map_err(bad)?;
    let lanes = app.lanes.lock().unwrap().clone();
    let item = |num: &str, it: &plan::Item| {
        let lane = lanes
            .values()
            .filter(|l| l.project == project && l.step == num && plan::same(&l.item, &it.text))
            .max_by(|a, b| a.created_at.cmp(&b.created_at));
        json!({"text": it.text, "done": it.done, "check": it.check, "lane": lane.map(|l| json!({"id": l.id, "status": l.status}))})
    };
    let steps: Vec<Value> = steps.iter().map(|st| json!({"num": st.num, "title": st.title, "items": st.items.iter().map(|it| item(&st.num, it)).collect::<Vec<_>>()})).collect();
    Ok(Json(
        json!({"project": project, "steps": steps, "autopilot": lanes::autopilot_on(&app, &project), "main_red": app.kv(&format!("main_red:{project}"))}),
    ))
}

pub(super) async fn plan_step(
    State(app): State<Shared>,
    Path(project): Path<String>,
    Json(b): Json<Value>,
) -> R {
    let field = |key: &str| super::s(&b, key);
    let (num, title, intro) = (field("step"), field("title"), field("intro"));
    plan::add_step(&app, &project, &num, &title, &intro)
        .await
        .map_err(bad)?;
    Ok(Json(json!({"ok": true})))
}
