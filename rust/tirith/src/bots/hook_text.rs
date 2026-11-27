//! The text of a GitLab merge request, pipeline or job event for the bot: the essentials on a few short lines, since the raw call is longer than the text a task may hold.
use super::hook_labels::names;
use serde_json::Value;
/// The longest value shown.
const VALUE: usize = 300;
/// The first non-empty value among `paths` (JSON pointers joined by `|`), on one line.
fn pick(b: &Value, paths: &str) -> String {
    paths
        .split('|')
        .filter_map(|p| match b.pointer(p)? {
            Value::String(s) => Some(s.clone()),
            Value::Number(n) => Some(n.to_string()),
            _ => None,
        })
        .map(|s| {
            let line: String = s
                .chars()
                .map(|c| if c.is_control() { ' ' } else { c })
                .collect();
            line.trim().chars().take(VALUE).collect::<String>()
        })
        .find(|s| !s.is_empty())
        .unwrap_or_default()
}
const WHO: &str = "/user/username|/user/name|/user_username";
const MR: [(&str, &str); 8] = [
    ("project", "/project/path_with_namespace"),
    ("mr iid", "/object_attributes/iid"),
    ("title", "/object_attributes/title"),
    ("url", "/object_attributes/url"),
    ("source branch", "/object_attributes/source_branch"),
    ("state", "/object_attributes/state"),
    ("merge status", "/object_attributes/detailed_merge_status"),
    ("pipeline id", "/object_attributes/head_pipeline_id"),
];
const PIPELINE: [(&str, &str); 11] = [
    ("project", "/project/path_with_namespace"),
    ("pipeline id", "/object_attributes/id"),
    ("pipeline status", "/object_attributes/status"),
    ("ref", "/object_attributes/ref"),
    ("pipeline url", "/object_attributes/url"),
    ("mr iid", "/merge_request/iid"),
    ("title", "/merge_request/title"),
    ("url", "/merge_request/url"),
    ("source branch", "/merge_request/source_branch"),
    ("state", "/merge_request/state"),
    ("failed jobs", "/failed_jobs"),
];
const JOB: [(&str, &str); 7] = [
    ("project", "/project/path_with_namespace|/project_name"),
    ("job", "/build_name"),
    ("stage", "/build_stage"),
    ("job status", "/build_status"),
    ("ref", "/ref"),
    ("pipeline id", "/pipeline_id"),
    ("failure", "/build_failure_reason"),
];
/// The essentials of a GitLab event whose labels are `labels` (the first is its own): labels, project, merge request iid, title and link, source branch, state, pipeline id and status, assignees and reviewers, and who triggered it. None for any other kind.
pub(super) fn compact(labels: &[String], body: &Value) -> Option<String> {
    let kind = labels.first()?.strip_prefix("gitlab:")?.split('.').next()?;
    let mut b = body.clone();
    let failed: Vec<&str> = body["builds"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|j| j["status"] == "failed")
        .filter_map(|j| j["name"].as_str())
        .collect();
    b["failed_jobs"] = Value::from(failed.join(", "));
    let rows: &[(&str, &str)] = match kind {
        "merge_request" => &MR,
        "pipeline" => &PIPELINE,
        "build" | "job" => &JOB,
        _ => return None,
    };
    let mut lines = vec![format!("gitlab event: {}", labels.join(" "))];
    lines.extend(rows.iter().map(|(k, p)| format!("{k}: {}", pick(&b, p))));
    let (assignees, reviewers) = (names(&body["assignees"]), names(&body["reviewers"]));
    lines.push(format!("assignees: {}", assignees.join(", ")));
    lines.push(format!("reviewers: {}", reviewers.join(", ")));
    lines.push(format!("triggered by: {}", pick(&b, WHO)));
    lines.retain(|l| !l.ends_with(": "));
    Some(lines.join("\n"))
}
