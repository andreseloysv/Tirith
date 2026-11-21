//! What a webhook call brought, as a bot's `webhook` event: where from, what kind, its title, link and text, and the body.
use serde_json::{json, Value};
/// The longest `text` a webhook event carries.
const TEXT: usize = 4000;
/// The event of one `tirith-hook {"v":1,"at":…,"headers":{…},"body":…}` line (the part after the mark): `source` (github when GitHub sent it, else the user-agent), `kind` (`github:<event>.<action>`, or `hook`), `title`, `url`, `text` (the body as text, 4000 characters at most) and `data` (the body).
pub(super) fn event_of(payload: &str) -> Option<Value> {
    let v: Value = serde_json::from_str(payload).ok()?;
    let (h, body) = (&v["headers"], v["body"].clone());
    let github = h["x-github-event"].as_str().filter(|e| !e.is_empty());
    let source = github.map_or_else(|| agent(h), |_| "github".to_string());
    let kind = github.map_or("hook".to_string(), |e| match body["action"].as_str() {
        Some(a) => format!("github:{e}.{a}"),
        None => format!("github:{e}"),
    });
    let (title, url) = title_and_url(&body);
    let text = match &body {
        Value::String(s) => s.clone(),
        b => serde_json::to_string_pretty(b).unwrap_or_default(),
    };
    Some(
        json!({"source": source, "kind": kind, "title": title, "url": url, "text": text.chars().take(TEXT).collect::<String>(), "data": body, "at": v["at"]}),
    )
}
/// The sender's user-agent, cleaned of controls, 100 characters at most; `hook` without one.
fn agent(h: &Value) -> String {
    let ua: String = h["user-agent"]
        .as_str()
        .unwrap_or("")
        .chars()
        .filter(|c| !c.is_control())
        .take(100)
        .collect();
    Some(ua.trim().to_string())
        .filter(|u| !u.is_empty())
        .unwrap_or_else(|| "hook".into())
}
/// The title and link of GitHub's pull request, issue, release or comment (a comment's link with its issue's title), else the body's own `title` and `url`.
fn title_and_url(b: &Value) -> (String, String) {
    let s = |v: &Value| v.as_str().unwrap_or("").to_string();
    let item = ["pull_request", "issue", "release"]
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
        .map(|v| s(&v["html_url"]))
        .find(|u| !u.is_empty())
        .unwrap_or_else(|| s(&b["url"]));
    (title, url)
}
/// The match key of an event, as a routine's `when` names it: its kind when it says its source (`github:pull_request.opened`), else `<source>:<kind>`.
pub(super) fn label(e: &Value) -> String {
    let (source, kind) = (
        e["source"].as_str().unwrap_or(""),
        e["kind"].as_str().unwrap_or(""),
    );
    if kind.starts_with(&format!("{source}:")) {
        kind.to_string()
    } else {
        format!("{source}:{kind}")
    }
}
/// `when` holds for the event: `*`, its label itself, or a pattern of it with `*` for any part (as `github:pull_request.*`).
pub(super) fn matches(when: &str, e: &Value) -> bool {
    let (when, label) = (when.trim(), label(e));
    let pattern = format!("^{}$", regex::escape(when).replace(r"\*", ".*"));
    when == "*" || regex::Regex::new(&pattern).is_ok_and(|re| re.is_match(&label))
}
/// GitHub's call when a pull request is opened, as the page's «Probar» sends it: no platform in between.
pub(super) fn sample() -> String {
    json!({"v": 1, "at": crate::app::now_iso(), "headers": {"content-type": "application/json", "user-agent": "GitHub-Hookshot/tirith-test", "x-github-event": "pull_request"},
        "body": {"action": "opened", "number": 1, "pull_request": {"title": "Example pull request", "html_url": "https://github.com/example/repo/pull/1", "body": "An example sent by «Probar» on the page.", "user": {"login": "example"}}, "repository": {"full_name": "example/repo"}}}).to_string()
}
