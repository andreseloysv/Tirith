//! GitLab's call as «Probar» sends it, shaped as a relay passes it (a `GitLab/` user-agent, no `X-Gitlab-Event`).
use serde_json::{json, Value};

const LINK: &str = "https://gitlab.example.com/example/repo/-/merge_requests/1";

/// A merge request's attributes: its action (none when empty) and an oldrev for a push.
fn merge_request(body: &mut Value, action: &str) {
    let (verb, who) = action.split_once(':').unwrap_or((action, ""));
    let mut attrs = json!({"iid": 1, "title": "Example merge request", "url": LINK, "source_branch": "feature", "target_branch": "main", "state": "opened", "description": "An example sent by «Probar» on the page."});
    let (act, key) = match verb {
        "assigned" => ("update", "assignees"),
        "review_requested" => ("update", "reviewers"),
        "update.push" => ("update", ""),
        v => (v, ""),
    };
    let _ = (!act.is_empty()).then(|| attrs["action"] = json!(act));
    let _ = (verb == "update.push").then(|| attrs["oldrev"] = json!("a1b2c3d"));
    let _ = (!key.is_empty()).then(|| {
        let user = json!([{"username": who}]);
        body["changes"] = json!({key: {"previous": [], "current": user}});
        body[key] = user;
    });
    body["object_attributes"] = attrs;
}

/// GitLab's call for `gitlab:<object_kind>.<action>` (`pipeline.success`, `build.failed`, `merge_request.assigned:<user>`, `merge_request.update.push`; a merge request opened by default).
pub(super) fn sample(when: &str) -> String {
    let (object, action) = when.split_once('.').unwrap_or((when, ""));
    let (object, action) = match object {
        "" => ("merge_request", "open"),
        o => (o, action),
    };
    let mut body = json!({"object_kind": object, "user": {"username": "example-user"}, "project": {"path_with_namespace": "example/repo"}});
    let status = if action.is_empty() { "success" } else { action };
    match object {
        "pipeline" => {
            body["object_attributes"] = json!({"id": 31, "status": status, "ref": "feature"});
            body["merge_request"] = json!({"iid": 1, "title": "Example merge request", "url": LINK, "source_branch": "feature", "state": "opened"});
        }
        "build" | "job" => {
            body["build_status"] = json!(status);
            body["build_name"] = json!("test");
            body["pipeline_id"] = json!(31);
            body["ref"] = json!("feature");
        }
        _ => merge_request(&mut body, action),
    }
    json!({"v": 1, "at": crate::app::now_iso(), "headers": {"content-type": "application/json", "user-agent": "GitLab/tirith-test"}, "body": body}).to_string()
}
