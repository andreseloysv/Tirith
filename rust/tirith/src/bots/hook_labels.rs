//! The more specific labels of a GitLab merge request event: who was newly assigned or asked to review, and a push.
use super::hook_gitlab::clean;
use serde_json::Value;
/// The cleaned `username`s of a list of users (an object stands for a list of one).
pub(super) fn names(v: &Value) -> Vec<String> {
    let one = |u: &Value| Some(clean(u["username"].as_str()?)).filter(|n| !n.is_empty());
    match v {
        Value::Array(a) => a.iter().filter_map(one).collect(),
        Value::Object(_) => one(v).into_iter().collect(),
        _ => vec![],
    }
}
/// The people `key` (`assignees` or `reviewers`) gained: on open or reopen every current one, on update the ones in `changes.<key>.current` and not in `.previous`.
fn added(body: &Value, key: &str, action: &str) -> Vec<String> {
    let change = &body["changes"][key];
    match action {
        "open" | "reopen" => {
            let now = names(&body[key]);
            let one = (key == "assignees").then(|| names(&body["assignee"]));
            let now = if now.is_empty() {
                one.unwrap_or_default()
            } else {
                now
            };
            if now.is_empty() {
                names(&change["current"])
            } else {
                now
            }
        }
        "update" => {
            let before = names(&change["previous"]);
            let mut now = names(&change["current"]);
            now.retain(|n| !before.contains(n));
            now
        }
        _ => vec![],
    }
}
/// The labels a merge request event (its first label is `gitlab:merge_request.<action>`) has besides that one: `gitlab:merge_request.assigned:<user>`, `.review_requested:<user>` and, when the call has `object_attributes.oldrev`, `gitlab:merge_request.update.push`.
pub(super) fn extra(first: &str, body: &Value) -> Vec<String> {
    let Some(action) = first.strip_prefix("gitlab:merge_request.") else {
        return vec![];
    };
    let people = |key: &str, word: &str| -> Vec<String> {
        added(body, key, action)
            .into_iter()
            .map(|n| format!("gitlab:merge_request.{word}:{n}"))
            .collect()
    };
    let push = body["object_attributes"]["oldrev"]
        .as_str()
        .is_some_and(|r| !r.is_empty());
    let mut all = people("assignees", "assigned");
    all.extend(people("reviewers", "review_requested"));
    all.extend(push.then(|| "gitlab:merge_request.update.push".to_string()));
    all
}
