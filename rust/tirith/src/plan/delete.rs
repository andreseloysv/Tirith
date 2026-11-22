//! Removing an old step from the plan: its heading, prose and items, never renumbering the others.
use super::{change::change, item::Step};
use crate::app::Shared;

/// Takes step `num` out of the plan, when it exists and none of its lanes is still unmerged.
pub async fn delete_step(app: &Shared, project: &str, num: &str) -> Result<(), String> {
    if num.is_empty() || !num.chars().all(|c| c.is_ascii_digit()) {
        return Err(format!("\"{num}\" is not a step number"));
    }
    change(app, project, |lines, steps| {
        live_lanes(app, project, num)?;
        remove(lines, steps, num)
    })
    .await
}

/// Refuses while a lane of the step is not merged, naming those lanes.
fn live_lanes(app: &Shared, project: &str, num: &str) -> Result<(), String> {
    let mut live: Vec<String> = app
        .lanes
        .lock()
        .unwrap()
        .values()
        .filter(|l| l.project == project && l.step == num && l.status != "merged")
        .map(|l| format!("{} ({})", l.id, l.status))
        .collect();
    live.sort();
    match live.is_empty() {
        true => Ok(()),
        false => Err(format!(
            "step {num} still has lanes that are not merged: {}",
            live.join(", ")
        )),
    }
}

/// Removes the lines from the step's heading to the next numbered step (or the end), then the blank lines that leaves doubled at the seam.
fn remove(lines: &mut Vec<String>, steps: &[Step], num: &str) -> Result<(), String> {
    let i = steps
        .iter()
        .position(|s| s.num == num)
        .ok_or(format!("the plan has no step {num}"))?;
    let start = steps[i].line;
    let end = steps.get(i + 1).map_or(lines.len(), |s| s.line);
    lines.drain(start..end);
    let blank = |l: &String| l.trim().is_empty();
    let before = lines[..start].iter().rev().take_while(|l| blank(l)).count();
    let after = lines[start..].iter().take_while(|l| blank(l)).count();
    if start == lines.len() {
        lines.truncate(start - before);
    } else if start == 0 || before > 0 {
        lines.drain(start..start + after);
    }
    Ok(())
}
