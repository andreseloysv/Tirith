//! Starting a lane: an open plan item no lane works on becomes a queued lane with an id of its own.
use crate::app::{now_iso, Lane, Shared};
use crate::{models, plan};

pub fn slug(text: &str) -> String {
    let s: String = text
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    let s = s
        .split('-')
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    s.chars()
        .take(32)
        .collect::<String>()
        .trim_end_matches('-')
        .to_string()
}
/// The item must be open in that step of the project's plan.
fn ensure_open(app: &Shared, project: &str, step: &str, item: &str) -> Result<(), String> {
    let steps = plan::read(app, project)?;
    let st = steps
        .iter()
        .find(|s| s.num == step)
        .ok_or(format!("the plan of {project} has no step {step}"))?;
    st.items.iter().any(|i| !i.done && plan::same(&i.text, item)).then_some(()).ok_or_else(|| format!("\"{item}\" is not an open item of step {step} of {project}. Add it first: tirith plan add {project} {step} \"<text>\""))
}
/// No active lane may already work on the item.
fn ensure_free(app: &Shared, project: &str, step: &str, item: &str) -> Result<(), String> {
    app.lanes
        .lock()
        .unwrap()
        .values()
        .find(|l| l.active() && l.project == project && l.step == step && plan::same(&l.item, item))
        .map_or(Ok(()), |l| {
            Err(format!(
                "lane {} already works on that item ({})",
                l.id, l.status
            ))
        })
}
/// The item's words before its check, and a few random hex digits.
fn new_id(item: &str) -> String {
    format!(
        "{}-{:04x}",
        slug(item.split(" (check:").next().unwrap_or("")),
        chrono::Utc::now().timestamp_subsec_nanos() & 0xffff
    )
}
pub fn start(
    app: &Shared,
    project: &str,
    step: &str,
    item: &str,
    agent: Option<String>,
) -> Result<Lane, String> {
    let item = plan::clean(item);
    ensure_open(app, project, step, &item)?;
    ensure_free(app, project, step, &item)?;
    let (factory, role_hash) = app.stamp_for(&models::lane_role(&app.cfg()));
    let lane = Lane {
        id: new_id(&item),
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
