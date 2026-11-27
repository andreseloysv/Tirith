//! What GitLab's webhook call says of itself: `X-Gitlab-Event` (a name such as "Merge Request Hook") or, when a relay drops that header, a `GitLab/` user-agent with the body's `object_kind`; then `object_attributes.action`, or the status of a pipeline or job.
use serde_json::Value;

/// A name part kept to letters, digits and `_`.
pub(super) fn clean(w: &str) -> String {
    w.chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_')
        .collect()
}

/// `gitlab:<object_kind>.<action>` (`gitlab:merge_request.open`, `gitlab:pipeline.success`: a pipeline's or job's status is its action), or `gitlab:<object_kind>` for one with no action (`push`); none when GitLab did not send it. GitLab is known by its `X-Gitlab-Event` header, or by a user-agent that starts with `GitLab/` and a body with `object_kind` (the agentsmessenger.com relay passes no `X-Gitlab-Event`). The kind is the body's `object_kind`, else its header's name in words (`Merge Request Hook` is `merge_request`).
pub(super) fn kind(h: &Value, body: &Value) -> Option<String> {
    let header = h["x-gitlab-event"].as_str().filter(|e| !e.is_empty());
    let object = body["object_kind"].as_str().filter(|k| !k.is_empty());
    let agent = h["user-agent"]
        .as_str()
        .is_some_and(|u| u.starts_with("GitLab/"));
    let named = header.map(|e| e.trim_end_matches(" Hook").to_lowercase().replace(' ', "_"));
    let known = header.is_some() || (agent && object.is_some());
    let object = object.map(str::to_string).or(named).filter(|_| known)?;
    let object = clean(&object);
    let action = match object.as_str() {
        "pipeline" => &body["object_attributes"]["status"],
        "build" | "job" => &body["build_status"],
        _ => &body["object_attributes"]["action"],
    };
    let action = action.as_str().map(clean).filter(|a| !a.is_empty());
    Some(match action {
        Some(a) => format!("gitlab:{object}.{a}"),
        None => format!("gitlab:{object}"),
    })
}
