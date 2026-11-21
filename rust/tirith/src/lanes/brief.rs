//! A lane's brief: the factory's `brief` prompt with its placeholders filled from the lane, its step and its project.
use super::brief_parts::{others, project_lanes, rules, step_items};
use crate::app::{now_iso, Lane, Shared};
use crate::plan;
use regex::Regex;
use std::collections::HashMap;
use std::path::Path;
use std::sync::LazyLock;

/// The check of the lane's item, said as the brief says it.
fn check_line(items: Option<&[plan::Item]>, lane: &Lane) -> String {
    items
        .and_then(|s| s.iter().find(|i| plan::same(&i.text, &lane.item)))
        .and_then(|i| i.check.clone())
        .map(|c| format!("\nIts check (must pass on the merged result): `{c}`\n"))
        .unwrap_or_default()
}
/// Whether main is red, and why.
fn main_state(app: &Shared, lane: &Lane) -> String {
    app.kv(&format!("main_red:{}", lane.project))
        .map(|r| format!("red ({r}): not your job unless your item is about it"))
        .unwrap_or("green".into())
}
/// Rust lanes run the queue's clippy gate first, so a warning does not cost a second attempt.
fn clippy(app: &Shared, lane: &Lane) -> String {
    if Path::new(&lane.workspace).join("Cargo.toml").exists() {
        app.cfg().factory.prompt("clippy")
    } else {
        String::new()
    }
}
fn vars(app: &Shared, lane: &Lane) -> HashMap<&'static str, String> {
    let steps = plan::read(app, &lane.project).unwrap_or_default();
    let lanes = project_lanes(app, &lane.project);
    let items = steps
        .iter()
        .find(|s| s.num == lane.step)
        .map(|s| &s.items[..]);
    HashMap::from([
        ("id", lane.id.clone()),
        ("project", lane.project.clone()),
        ("step", lane.step.clone()),
        ("item", lane.item.clone()),
        ("check", check_line(items, lane)),
        ("items", step_items(items, lane, &lanes).join("\n")),
        ("others", others(lane, &lanes)),
        ("red", main_state(app, lane)),
        ("ws", lane.workspace.clone()),
        ("branch", lane.branch.clone()),
        ("clippy", clippy(app, lane)),
        ("rules", rules(app, lane)),
        ("at", now_iso()),
    ])
}
pub(super) fn brief(app: &Shared, lane: &Lane) -> String {
    let vars = vars(app, lane);
    static PLACEHOLDER: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\{(\w+)\}").unwrap());
    PLACEHOLDER
        .replace_all(&app.cfg().factory.prompt("brief"), |c: &regex::Captures| {
            vars[&c[1]].clone()
        })
        .into_owned()
}
