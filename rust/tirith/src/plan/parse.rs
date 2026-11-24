//! Reading a plan's markdown into its steps and items.
use super::item::{Item, Step};
use regex::Regex;
use std::sync::LazyLock;

pub(super) static CHECK: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\(check:\s*`([^`]+)`\s*\)").unwrap());
static STATUS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"\s*\((en curso|hecho|en revisión|falló|bloqueado|parcial|in progress|done)\b[^)]*\)\s*$",
    )
    .unwrap()
});

/// An item's text without NeoBot's old annotations (status notes, `<!-- tirith:… -->`).
pub fn clean(text: &str) -> String {
    let text = text.split("<!--").next().unwrap_or("").trim();
    STATUS.replace(text, "").trim().to_string()
}

pub fn parse(text: &str) -> Vec<Step> {
    let mut steps: Vec<Step> = text
        .lines()
        .enumerate()
        .fold(vec![], |mut steps, (i, line)| {
            read_line(&mut steps, i, line);
            steps
        });
    steps.retain(|s| !s.num.is_empty());
    steps
}

/// Line `i`: a `## N. Title` starts a step, a `- [ ]` or `- [x]` is an item of the last one.
fn read_line(steps: &mut Vec<Step>, i: usize, line: &str) {
    if let Some(head) = line.strip_prefix("## ") {
        steps.push(heading(head, i));
    } else if let Some((done, rest)) = checkbox(line) {
        steps.last_mut().into_iter().for_each(|step| {
            step.items.push(Item {
                text: clean(rest),
                done,
                check: CHECK.captures(rest).map(|c| c[1].to_string()),
                line: i,
            })
        });
    }
}

/// Whether the parser reads `line` as a heading or an item, and not as a step's prose.
pub(super) fn is_structure(line: &str) -> bool {
    line.starts_with("## ") || checkbox(line).is_some()
}

/// The step a heading starts; one without a number is no step (it is dropped).
fn heading(head: &str, i: usize) -> Step {
    let (num, title) = head
        .split_once(". ")
        .filter(|(n, _)| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()))
        .unwrap_or(("", head));
    Step {
        num: num.into(),
        title: title.trim().into(),
        items: vec![],
        line: i,
    }
}

/// Whether an item line is ticked, and its text.
fn checkbox(line: &str) -> Option<(bool, &str)> {
    line.strip_prefix("- [ ] ").map(|r| (false, r)).or_else(|| {
        line.strip_prefix("- [x] ")
            .or_else(|| line.strip_prefix("- [X] "))
            .map(|r| (true, r))
    })
}
