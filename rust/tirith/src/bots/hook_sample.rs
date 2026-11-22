//! The sample events «Probar» sends a routine, with no platform in between: GitHub's, the owner's mail script's, each shaped as the real one so the routine's `when` can hold for it.
use serde_json::{json, Value};
/// The item GitHub's event `event` is about, and its key in the body.
fn item(event: &str) -> (&'static str, Value) {
    let (key, title) = match event {
        "pull_request" => ("pull_request", "Example pull request"),
        "release" => ("release", "Example release"),
        _ => ("issue", "Example issue"),
    };
    (
        key,
        json!({"title": title, "html_url": "https://github.com/example/repo/pull/1", "body": "An example sent by «Probar» on the page.", "user": {"login": "example"}}),
    )
}
/// GitHub's call for `event` and `action` (a pull request opened by default), as «Probar» sends it.
fn github(event: &str, action: &str) -> String {
    let (key, it) = item(event);
    let mut body = json!({"number": 1, "repository": {"full_name": "example/repo"}});
    body[key] = it;
    let _ = (!action.is_empty()).then(|| body["action"] = json!(action));
    json!({"v": 1, "at": crate::app::now_iso(), "headers": {"content-type": "application/json", "user-agent": "GitHub-Hookshot/tirith-test", "x-github-event": event}, "body": body}).to_string()
}
/// The owner's mail script's call: one new e-mail.
fn mail() -> String {
    json!({"v": 1, "at": crate::app::now_iso(), "headers": {"content-type": "application/json", "user-agent": "Mozilla/5.0 (compatible; Google-Apps-Script)"},
        "body": {"title": "Example e-mail", "url": "https://mail.google.com/mail/u/0/#all/example", "from": "Example <example@example.com>", "subject": "Example e-mail", "date": crate::app::now_iso(), "text": "An example sent by «Probar» on the page."}}).to_string()
}
/// The sample that fits a routine's `when`: an e-mail for `mail:…`, the named GitHub event and action for `github:<event>.<action>` (no `*`), else a pull request opened.
pub(super) fn for_when(when: &str) -> String {
    let when = when.trim();
    let github_event = when.strip_prefix("github:").filter(|w| !w.contains('*'));
    match github_event {
        _ if when.starts_with("mail") => mail(),
        Some(w) => {
            let (event, action) = w.split_once('.').unwrap_or((w, ""));
            github(event, action)
        }
        None => github("pull_request", "opened"),
    }
}
