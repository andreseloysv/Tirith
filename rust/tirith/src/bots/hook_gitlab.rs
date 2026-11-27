//! What GitLab's webhook call says of itself: `X-Gitlab-Event` (a name such as "Merge Request Hook"), the body's `object_kind` and, for most, `object_attributes.action`.
use serde_json::{json, Value};

/// `gitlab:<object_kind>.<action>` (`gitlab:merge_request.open`, `gitlab:issue.close`), or `gitlab:<object_kind>` for one with no action (`push`, `pipeline`); none when GitLab did not send it. The kind is the body's `object_kind`, else its header's name in words (`Merge Request Hook` is `merge_request`).
pub(super) fn kind(h: &Value, body: &Value) -> Option<String> {
    let header = h["x-gitlab-event"].as_str().filter(|e| !e.is_empty())?;
    let named = header
        .trim_end_matches(" Hook")
        .to_lowercase()
        .replace(' ', "_");
    let object = body["object_kind"].as_str().filter(|k| !k.is_empty());
    let object = object.map_or(named, str::to_string);
    let clean = |w: &str| {
        w.chars()
            .filter(|c| c.is_ascii_alphanumeric() || *c == '_')
            .collect::<String>()
    };
    let action = body["object_attributes"]["action"].as_str().map(clean);
    Some(match action.filter(|a| !a.is_empty()) {
        Some(a) => format!("gitlab:{}.{a}", clean(&object)),
        None => format!("gitlab:{}", clean(&object)),
    })
}

/// GitLab's call as «Probar» sends it for `gitlab:<object_kind>.<action>` (a merge request opened by default).
pub(super) fn sample(when: &str) -> String {
    let (object, action) = when.split_once('.').unwrap_or((when, ""));
    let (object, action) = match object {
        "" => ("merge_request", "open"),
        o => (o, action),
    };
    let header = format!(
        "{} Hook",
        object.split('_').map(cap).collect::<Vec<_>>().join(" ")
    );
    let mut attrs = json!({"title": "Example merge request", "url": "https://gitlab.example.com/example/repo/-/merge_requests/1", "description": "An example sent by «Probar» on the page."});
    let _ = (!action.is_empty()).then(|| attrs["action"] = json!(action));
    json!({"v": 1, "at": crate::app::now_iso(), "headers": {"content-type": "application/json", "user-agent": "GitLab/tirith-test", "x-gitlab-event": header},
        "body": {"object_kind": object, "project": {"path_with_namespace": "example/repo"}, "object_attributes": attrs}}).to_string()
}
fn cap(w: &str) -> String {
    let mut c = w.chars();
    c.next()
        .map_or(String::new(), |f| f.to_uppercase().chain(c).collect())
}
