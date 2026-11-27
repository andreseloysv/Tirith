//! Which routines an event reaches: its labels against a routine's `when`.
use serde_json::Value;
/// The first label of an event, as a routine's `when` names it: its kind when it says its source (`github:pull_request.opened`), else `<source>:<kind>`.
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
/// Every label of an event: the event's own, then the more specific ones its payload gives (`gitlab:merge_request.assigned:<user>`).
pub(super) fn labels(e: &Value) -> Vec<String> {
    let all: Vec<String> = e["labels"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|l| l.as_str().map(str::to_string))
        .collect();
    if all.is_empty() {
        vec![label(e)]
    } else {
        all
    }
}
/// `when` holds for the event: it lists one or more patterns, separated by commas, and one holds for one of the event's labels. A pattern is `*`, a label itself, or a pattern of it with `*` for any part (as `github:pull_request.*`).
pub(super) fn matches(when: &str, e: &Value) -> bool {
    let labels = labels(e);
    when.split(',')
        .map(str::trim)
        .filter(|w| !w.is_empty())
        .any(|w| w == "*" || labels.iter().any(|l| one(w, l)))
}
fn one(when: &str, label: &str) -> bool {
    let pattern = format!("^{}$", regex::escape(when).replace(r"\*", ".*"));
    regex::Regex::new(&pattern).is_ok_and(|re| re.is_match(label))
}
