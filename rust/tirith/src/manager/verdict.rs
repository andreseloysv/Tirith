//! A judge's verdict: the last line it wrote in its rubric's shape (it thinks first, and a small model that began with one corrects it below).
use regex::Regex;
use std::sync::LazyLock;

/// A label as a rubric lists it.
static LABEL: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?m)^- `([a-z][a-z_-]*)`").unwrap());

/// A score, where the rubric lists no labels.
const SCORE: &str = r"0(?:\.\d+)?|1(?:\.0+)?";

/// What a judge that gave none is told, once.
pub(super) const AGAIN: &str =
    "\n\nYour answer had no verdict in that shape. Your last line must be the verdict itself.";

/// Where `text`'s last verdict starts: one of `rubric`'s labels (a score when it lists none) and a dash.
fn find(text: &str, rubric: &str) -> Option<usize> {
    let labels: Vec<String> = LABEL
        .captures_iter(rubric)
        .map(|c| regex::escape(&c[1]))
        .collect();
    let verdicts = match labels.is_empty() {
        true => SCORE.to_string(),
        false => labels.join("|"),
    };
    let start = r"(?:^[ \t]*(?:[—–-][ \t]*)?|[`*])";
    let shape = Regex::new(&format!(r"(?m){start}({verdicts})[`*]*\s+[—–]")).ok()?;
    let found = shape.captures_iter(text).filter_map(|c| c.get(1)).last();
    found.map(|m| m.start())
}

/// True when `text` gives a verdict of `rubric`'s.
pub(super) fn given(text: &str, rubric: &str) -> bool {
    find(text, rubric).is_some()
}

/// `text` from its last verdict; all of it when none is there.
pub(super) fn last<'a>(text: &'a str, rubric: &str) -> &'a str {
    find(text, rubric).map_or(text, |at| &text[at..])
}
