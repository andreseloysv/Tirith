//! Launching a lane's agent in its slot: a new slot for a lane without one, the brief or a resume of its session.
use super::brief::brief;
use super::spawn::spawn_agent;
use super::worktree::take_slot;
use crate::app::{Lane, Shared};
use crate::config::Project;
use crate::models;
use std::path::{Path, PathBuf};

/// A lane without a live worktree gets a fresh slot on `lane/<id>`.
async fn ensure_slot(app: &Shared, project: &Project, lane: &mut Lane) -> Result<(), String> {
    if lane.workspace.is_empty() || !Path::new(&lane.workspace).exists() {
        lane.branch = format!("lane/{}", lane.id);
        lane.workspace = take_slot(app, project, lane).await?.display().to_string();
        lane.session.clear(); // an agent session lives in its folder: in another slot the lane starts from the brief
        app.save_lane(lane);
    }
    Ok(())
}
/// The brief, with NeoBot's pending message after it.
fn write_brief(app: &Shared, lane: &Lane, msg: &str, path: &Path) -> Result<(), String> {
    std::fs::write(
        path,
        brief(app, lane)
            + &if msg.is_empty() {
                String::new()
            } else {
                format!("\n## Message from NeoBot\n{msg}\n")
            },
    )
    .map_err(|e| e.to_string())
}
/// The model the lane runs, joined to its tried list: the fallback chain skips what this lane already used.
fn note_model(
    lane: &mut Lane,
    role_agent: &str,
    role_model: Option<String>,
    overridden: bool,
    alias: &str,
) {
    lane.model = lane
        .model
        .take()
        .or_else(|| role_model.clone().filter(|_| lane.agent == role_agent));
    let used = lane
        .model
        .clone()
        .or(role_model.filter(|_| !overridden))
        .map_or_else(|| alias.to_string(), |m| format!("{alias}:{m}"));
    let _ = (!lane.tried.contains(&used)).then(|| lane.tried.push(used));
}
/// Runs the lane's agent in its slot (a new slot for a lane without one): the brief, or a resume of its session with the pending message.
pub(super) async fn launch(app: &Shared, mut lane: Lane) -> Result<(), String> {
    let cfg = app.cfg();
    let project = cfg.project(&lane.project).ok_or("unknown project")?.clone();
    ensure_slot(app, &project, &mut lane).await?;
    let ws = PathBuf::from(&lane.workspace);
    let msg = std::mem::take(&mut lane.resume);
    let brief_path = app.home.join(format!("briefs/{}.md", lane.id));
    let resume = (!lane.session.is_empty() && !msg.is_empty())
        .then_some((lane.session.as_str(), msg.as_str()));
    if resume.is_none() {
        write_brief(app, &lane, &msg, &brief_path)?;
    }
    let role = models::role(app, &models::lane_role(&cfg));
    let over = (lane.agent != role.agent || lane.model.is_some()).then_some(lane.agent.as_str());
    let cmd = models::lane_argv(app, over, lane.model.as_deref(), &brief_path, &ws, resume)?;
    let overridden = over.is_some();
    note_model(&mut lane, &role.agent, role.model, overridden, &cmd.alias);
    spawn_agent(app, lane, &ws, cmd.argv)
}
