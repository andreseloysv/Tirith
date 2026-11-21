//! Adding an item to a step of the plan.
use super::{change::change, check::check_problem, item::Step, lookup::same, parse::CHECK};
use crate::app::Shared;

pub async fn add(app: &Shared, project: &str, step: &str, text: &str) -> Result<(), String> {
    let text = super::clean(text);
    refused(app, project, &text).map_or(Ok(()), Err)?;
    change(app, project, |lines, steps| {
        insert(lines, steps, step, &text)
    })
    .await
}

/// Why the item cannot be added: no text, a check that can never pass, or a word the commit guard forbids.
fn refused(app: &Shared, project: &str, text: &str) -> Option<String> {
    if text.is_empty() {
        return Some("the item needs a text".to_string());
    }
    CHECK
        .captures(text)
        .and_then(|c| check_problem(&c[1]))
        .or_else(|| forbidden(app, project, text))
}

/// A forbidden word of the project's commit guard would make the plan uncommittable and block every merge.
fn forbidden(app: &Shared, project: &str, text: &str) -> Option<String> {
    let hit = app
        .cfg()
        .project(project)
        .map(|p| crate::guard::words(app, p))
        .filter(|w| !w.is_empty())
        .and_then(|w| regex::Regex::new(&format!(r"(?i)\b({})\b", w.join("|"))).ok())
        .and_then(|r| r.find(text).map(|m| m.as_str().to_string()));
    hit.map(|word| {
        format!("\"{word}\" is a forbidden word of this project's commit guard: say it another way")
    })
}

/// Puts the item after the last line of its step that is not blank.
fn insert(lines: &mut Vec<String>, steps: &[Step], step: &str, text: &str) -> Result<(), String> {
    let i = steps
        .iter()
        .position(|s| s.num == step)
        .ok_or(format!("the plan has no step {step}"))?;
    (!(steps[i].items.iter().any(|it| same(&it.text, text))))
        .then_some(())
        .ok_or_else(|| format!("step {step} already has that item"))?;
    let end = steps.get(i + 1).map(|s| s.line).unwrap_or(lines.len());
    let at = (steps[i].line + 1..end)
        .rev()
        .find(|&k| !lines[k].trim().is_empty())
        .map_or(steps[i].line + 1, |k| k + 1);
    lines.insert(at, format!("- [ ] {text}"));
    Ok(())
}
