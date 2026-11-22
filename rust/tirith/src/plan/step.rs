//! Creating the plan's next step.
use super::{add::forbidden, change::change, item::Step, parse::is_structure};
use crate::app::Shared;

/// Appends `## num. title`, and `intro` below it when there is one, as the step after the plan's last.
pub async fn add_step(
    app: &Shared,
    project: &str,
    num: &str,
    title: &str,
    intro: &str,
) -> Result<(), String> {
    let (title, intro) = (title.trim(), intro.trim());
    refused(app, project, title, intro).map_or(Ok(()), Err)?;
    change(app, project, |lines, steps| {
        append(lines, steps, num, title, intro)
    })
    .await
}

/// Why the words cannot be a step: no title, one that breaks the plan's structure, or a word the commit guard forbids.
fn refused(app: &Shared, project: &str, title: &str, intro: &str) -> Option<String> {
    if title.is_empty() {
        return Some("the step needs a title".to_string());
    }
    if title.contains(['\n', '\r']) {
        return Some("the step's title is one line".to_string());
    }
    if intro.lines().any(is_structure) {
        return Some("the step's paragraph cannot hold a heading or an item".to_string());
    }
    forbidden(app, project, &format!("{title}\n{intro}"))
}

/// Puts the step at the end, when `num` follows the highest number the plan has (1 in a plan without steps).
fn append(
    lines: &mut Vec<String>,
    steps: &[Step],
    num: &str,
    title: &str,
    intro: &str,
) -> Result<(), String> {
    if steps.iter().any(|s| s.num == num) {
        return Err(format!("the plan already has step {num}"));
    }
    let last = steps.iter().filter_map(|s| s.num.parse::<u32>().ok()).max();
    let next = last.map_or(1, |n| n.saturating_add(1)).to_string();
    if num != next {
        return Err(format!("the plan's next step is {next}, not \"{num}\""));
    }
    while lines.last().is_some_and(|l| l.trim().is_empty()) {
        lines.pop();
    }
    if !lines.is_empty() {
        lines.push(String::new());
    }
    lines.push(format!("## {num}. {title}"));
    if !intro.is_empty() {
        lines.push(String::new());
        lines.extend(intro.lines().map(String::from));
    }
    Ok(())
}
