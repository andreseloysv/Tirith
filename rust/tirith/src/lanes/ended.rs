//! The agent's process is gone: where the lane goes next.
use super::failed::{no_commits, out_of_credits, without_commits};
use super::logscan::this_run;
use super::session::codex_session;
use crate::app::{now_iso, tail, Lane, Shared};
use crate::{models, plan};

/// Only what the agent program itself said counts: its plain-text output and its final result event, never the files and command output it read (a lane working on NeoBot reads the very patterns that detect it).
fn agent_said(raw: &str) -> String {
    raw.lines()
        .filter_map(|l| match serde_json::from_str::<serde_json::Value>(l) {
            Ok(v) if l.trim_start().starts_with('{') => match v["type"].as_str() {
                Some("result") if v["is_error"] == true => Some(l.to_string()),
                Some("turn.failed") => v["error"]["message"].as_str().map(String::from),
                Some("error") => v["message"].as_str().map(String::from),
                Some("session.error") => v["data"]["message"].as_str().map(String::from),
                _ => None,
            },
            _ if !l.trim_start().starts_with('{') => Some(l.to_string()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n")
}
/// A working lane goes to the merge queue; a waiting one answered meanwhile is queued again.
fn to_merge_queue(app: &Shared, id: &str, code: Option<i32>) {
    app.update_lane(id, |l| {
        l.pid = None;
        match l.status.as_str() {
            "running" => {
                l.status = "merge-queued".into();
                l.ended_at = Some(now_iso());
                l.note = if l.note.is_empty() {
                    format!(
                        "ended (exit {})",
                        code.map(|c| c.to_string()).unwrap_or("?".into())
                    )
                } else {
                    std::mem::take(&mut l.note)
                };
            }
            "waiting" if !l.resume.is_empty() => l.status = "queued".into(), // answered while it ended its turn
            _ => {}
        }
    });
    app.merge_wake.notify_one();
}
/// Ended without commits and without a check on its item that could still be true.
async fn nothing_to_merge(app: &Shared, lane: &Lane) -> bool {
    lane.status == "running"
        && no_commits(app, lane).await
        && plan::find(app, &lane.project, &lane.step, &lane.item).is_none_or(|i| i.check.is_none())
}
/// The agent's process is gone: a working lane goes to the merge queue (or, out of credits, to the next agent of the fallback chain); a waiting lane waits for its answer.
pub(super) async fn ended(app: &Shared, id: &str, code: Option<i32>) {
    let Some(lane) = app.lane(id) else { return };
    crate::costs::run_ended(app, &lane);
    let text = tail(&app.home.join(format!("logs/lanes/{id}.log")), 200);
    let raw = this_run(&text).to_string();
    let _ = (lane.agent == "codex").then(|| codex_session(app, id, &raw));
    let log = crate::app::redact_secrets(&agent_said(&raw));
    if lane.status == "running" && models::out_of_credits(&log) {
        out_of_credits(app, &lane);
    } else if nothing_to_merge(app, &lane).await {
        without_commits(app, &lane, code);
    } else {
        to_merge_queue(app, id, code);
    }
    app.sched_wake.notify_one();
}
