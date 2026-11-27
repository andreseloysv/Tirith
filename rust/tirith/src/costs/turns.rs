//! The bots' and judges' turns next to the lanes' runs, so the cost of an issue is complete.
use super::{rows::turn_runs, sum::Run};
use crate::app::Shared;
use std::collections::HashMap;

/// `lanes` and the turns that ended since `since`. A turn takes the project of the lanes that worked on its issue (none when no lane did); with a `project` only those of that project stay.
pub fn with_turns(app: &Shared, lanes: Vec<Run>, project: &str, since: &str) -> Vec<Run> {
    let of: HashMap<String, String> = lanes
        .iter()
        .filter(|r| !r.issue.is_empty())
        .map(|r| (r.issue.clone(), r.project.clone()))
        .collect();
    let turns = turn_runs(app, since).into_iter().map(|mut r| {
        r.project = of.get(&r.issue).cloned().unwrap_or_default();
        r
    });
    let wanted = |r: &Run| project.is_empty() || r.project == project;
    lanes.into_iter().chain(turns.filter(wanted)).collect()
}
