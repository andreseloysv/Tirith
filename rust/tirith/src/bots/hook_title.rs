//! The title and link of a webhook call's body.
use serde_json::Value;

/// The title and link of GitHub's pull request, issue, release or comment (a comment's link with its issue's title), else the body's own `title` and `url`.
pub(super) fn title_and_url(b: &Value) -> (String, String) {
    let s = |v: &Value| v.as_str().unwrap_or("").to_string();
    let item = ["pull_request", "issue", "release", "object_attributes"]
        .iter()
        .map(|k| &b[*k])
        .find(|v| v.is_object());
    let title = item.map_or_else(
        || s(&b["title"]),
        |i| {
            Some(s(&i["title"]))
                .filter(|t| !t.is_empty())
                .unwrap_or_else(|| s(&i["name"]))
        },
    );
    let url = [&b["comment"]]
        .into_iter()
        .chain(item)
        .map(|v| {
            Some(s(&v["html_url"]))
                .filter(|u| !u.is_empty())
                .unwrap_or_else(|| s(&v["url"]))
        })
        .find(|u| !u.is_empty())
        .unwrap_or_else(|| s(&b["url"]));
    (title, url)
}
