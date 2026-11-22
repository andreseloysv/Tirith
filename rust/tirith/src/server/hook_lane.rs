//! What a hook call is about: its lane and project, its tool call under the rules' names, and its summary.
use super::{
    hook_find::{lane_at, project_of},
    s,
    who::Agent,
};
use crate::app::{now_s, Lane, Shared};
use crate::config::Project;
use crate::{rules, sandbox};
use serde_json::{json, Value};

pub(super) struct Call {
    pub cwd: String,
    pub lane: Option<Lane>,
    pub role: Option<String>,
    pub project: Option<Project>,
    /// The tool's name as the agent used it.
    pub tool: String,
    pub session: String,
    /// The tool and its input under the names the rules and the sandbox audit know.
    pub known: String,
    pub input: Value,
    pub summary: String,
    pub lane_id: String,
    pub project_id: String,
}

impl Call {
    pub fn read(app: &Shared, event: &str, agent: Option<&Agent>, p: &Value) -> Call {
        let cwd = s(p, "cwd");
        // A token alone says who it is; without one the folder only says which refusals hold there.
        let (lane, role) = match agent {
            Some(a) => (
                app.lane(&a.lane),
                Some(a.role.clone()).filter(|r| !r.is_empty()),
            ),
            None => (lane_at(app, &cwd), rules::role_at(app, &cwd)),
        };
        let project = project_of(app, lane.as_ref(), &cwd);
        let tool = s(p, "tool_name");
        let session = s(p, "session_id");
        // The rules and the sandbox audit see every agent's call under the names they know; the events keep `tool`, the name the agent used.
        let (known, input) = rules::normalize(&tool, &p["tool_input"], &cwd);
        let summary = if event == "UserPromptSubmit" {
            rules::summary("", &json!({"prompt": p["prompt"]}))
        } else {
            rules::summary(&tool, &input)
        };
        let lane_id = lane.as_ref().map(|l| l.id.clone()).unwrap_or_default();
        let project_id = project.as_ref().map(|p| p.id.clone()).unwrap_or_default();
        Call {
            cwd,
            lane,
            role,
            project,
            tool,
            session,
            known,
            input,
            summary,
            lane_id,
            project_id,
        }
    }

    /// Keeps the lane's session id and its last activity.
    pub fn note_activity(&self, app: &Shared) {
        let Some(l) = &self.lane else { return };
        drop(
            (!self.session.is_empty() && l.session != self.session)
                .then(|| app.update_lane(&l.id, |x| x.session = self.session.clone())),
        );
        app.lanes
            .lock()
            .unwrap()
            .get_mut(&l.id)
            .into_iter()
            .for_each(|m| m.last_activity = now_s());
        // in memory only: activity is not worth a disk write per tool call
    }

    /// One `access` event per sandbox tag of the call.
    pub fn record_access(&self, app: &Shared) {
        sandbox::tags(&self.known, &self.input)
            .into_iter()
            .for_each(|(tag, target)| {
                _ = app.event(
                    &self.lane_id,
                    &self.project_id,
                    "access",
                    tag,
                    &target,
                    &self.session,
                )
            });
    }
}
