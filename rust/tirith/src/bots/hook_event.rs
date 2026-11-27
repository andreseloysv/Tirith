//! What a webhook call brought, as a bot's `webhook` event: where from, what kind, its title, link and text, and the body.
use serde_json::{json, Value};
/// The longest `text` a webhook event carries.
const TEXT: usize = 4000;
/// The event of one `tirith-hook {"v":1,"at":…,"headers":{…},"body":…}` line (the part after the mark): `source` (github when GitHub sent it, mail for the owner's mail script, else the user-agent), `kind` (`github:<event>.<action>`, `mail:received`, or `hook`), `title`, `url`, `text` (the body as text, 4000 characters at most; a mail's sender, subject, date and body) and `data` (the body).
pub(super) fn event_of(payload: &str) -> Option<Value> {
    let v: Value = serde_json::from_str(payload).ok()?;
    let (h, body) = (&v["headers"], v["body"].clone());
    let github = h["x-github-event"].as_str().filter(|e| !e.is_empty());
    let gitlab = super::hook_gitlab::kind(h, &body).filter(|_| github.is_none());
    let mail = github.is_none()
        && gitlab.is_none()
        && body["from"].is_string()
        && body["subject"].is_string();
    let source = match (github, mail) {
        _ if gitlab.is_some() => "gitlab".to_string(),
        (Some(_), _) => "github".to_string(),
        (_, true) => "mail".to_string(),
        _ => agent(h),
    };
    let kind = match (github, mail) {
        _ if gitlab.is_some() => gitlab.clone().unwrap_or_default(),
        (Some(e), _) => match body["action"].as_str() {
            Some(a) => format!("github:{e}.{a}"),
            None => format!("github:{e}"),
        },
        (_, true) => "mail:received".to_string(),
        _ => "hook".to_string(),
    };
    let (title, url) = super::hook_title::title_and_url(&body);
    let text = match (&body, mail) {
        (_, true) => format!(
            "From: {}\nSubject: {}\nDate: {}\n\n{}",
            body["from"].as_str().unwrap_or(""),
            body["subject"].as_str().unwrap_or(""),
            body["date"].as_str().unwrap_or(""),
            body["text"].as_str().unwrap_or("")
        ),
        (Value::String(s), _) => s.clone(),
        (b, _) => serde_json::to_string_pretty(b).unwrap_or_default(),
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
