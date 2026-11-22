//! A judge's verdict: the last line it wrote in its rubric's shape (a small model sometimes writes one, thinks, and corrects it below).
use regex::Regex;
use std::sync::LazyLock;

/// A label as a rubric lists it.
static LABEL: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?m)^- `([a-z][a-z_-]*)`").unwrap());

/// `text` from its last verdict, one of `rubric`'s labels and a dash; all of it when the rubric lists none (a score) or none is there.
pub(super) fn last<'a>(text: &'a str, rubric: &str) -> &'a str {
    let labels: Vec<String> = LABEL
        .captures_iter(rubric)
        .map(|c| regex::escape(&c[1]))
        .collect();
    if labels.is_empty() {
        return text;
    }
    let shape = format!(r"(?m)(?:^\s*|[`*])({})[`*]*\s+[—–]", labels.join("|"));
    let Ok(shape) = Regex::new(&shape) else {
        return text;
    };
    let found = shape.captures_iter(text).filter_map(|c| c.get(1)).last();
    found.map_or(text, |m| &text[m.start()..])
}
