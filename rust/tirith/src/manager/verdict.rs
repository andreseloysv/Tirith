//! A judge's verdict: the last line it wrote in its rubric's shape (it thinks first, and a small model that began with one corrects it below).
use regex::Regex;
use std::sync::LazyLock;

/// A label as a rubric lists it.
static LABEL: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?m)^- `([a-z][a-z_-]*)`").unwrap());

/// A score, where the rubric lists no labels.
const SCORE: &str = r"0(?:\.\d+)?|1(?:\.0+)?";

/// `text` from its last verdict, one of `rubric`'s labels (a score when it lists none) and a dash; all of it when none is there.
pub(super) fn last<'a>(text: &'a str, rubric: &str) -> &'a str {
    let labels: Vec<String> = LABEL
        .captures_iter(rubric)
        .map(|c| regex::escape(&c[1]))
        .collect();
    let verdicts = match labels.is_empty() {
        true => SCORE.to_string(),
        false => labels.join("|"),
    };
    let shape = format!(r"(?m)(?:^[ \t]*|[`*])({verdicts})[`*]*\s+[—–]");
    let Ok(shape) = Regex::new(&shape) else {
        return text;
    };
    let found = shape.captures_iter(text).filter_map(|c| c.get(1)).last();
    found.map_or(text, |m| &text[m.start()..])
}
