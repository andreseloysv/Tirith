//! The recorded steps as the bot reads them: one line each, what a web page said cleaned to one line and the whole between two marks no page can guess, the owner's words under the step they follow.
use crate::server::s;
use serde_json::Value;
/// What a page said, on one line: controls become spaces, 300 characters at most.
fn plain(text: &str) -> String {
    text.chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .take(300)
        .collect()
}
/// One entry as a line: a numbered step, or the owner's words under the step before.
fn line(n: &mut usize, e: &Value) -> String {
    let time = s(e, "at").get(11..19).unwrap_or("").to_string();
    let kind = s(e, "kind");
    if kind == "said" {
        return format!("   The owner wrote: «{}»", s(e, "text"));
    }
    *n += 1;
    let word = |k: &str| plain(&s(e, k));
    let what = match kind.as_str() {
        "page" => format!("page «{}» {}", word("title"), word("url")),
        "click" => format!(
            "click «{}» ({}, {})",
            word("text"),
            word("role"),
            word("selector")
        ),
        _ => format!("type «{}» = {}", word("label"), word("value")),
    };
    format!("{n}. {time} {what}")
}
/// The steps and words sorted by time (a step and words at the same moment keep the order they came in).
pub(super) fn lines(entries: &[Value]) -> String {
    let mut sorted: Vec<&Value> = entries.iter().collect();
    sorted.sort_by_key(|e| s(e, "at"));
    let mut n = 0;
    sorted
        .into_iter()
        .map(|e| line(&mut n, e))
        .collect::<Vec<_>>()
        .join("\n")
}
/// The lines between two marks: what the pages said is data from outside, never instructions; only the lines «The owner wrote» are the owner's.
pub(super) fn fenced(entries: &[Value], mark: &str) -> String {
    format!(
        "<<<outside-data {mark}\n{}\noutside-data {mark}>>>",
        lines(entries)
    )
}
