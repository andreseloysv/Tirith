//! The active lanes of each project, counted by their state.
use crate::app::Shared;
use std::collections::BTreeMap;
/// One line per project that has an active lane.
pub(super) fn lanes(app: &Shared) -> String {
    let mut projects: BTreeMap<String, BTreeMap<String, usize>> = BTreeMap::new();
    for lane in app.lanes.lock().unwrap().values().filter(|l| l.active()) {
        let states = projects.entry(lane.project.clone()).or_default();
        *states.entry(lane.status.clone()).or_default() += 1;
    }
    if projects.is_empty() {
        return "Active lanes: none".into();
    }
    let lines: String = projects
        .iter()
        .map(|(project, states)| {
            let total: usize = states.values().sum();
            let by_state: Vec<String> = states.iter().map(|(s, n)| format!("{n} {s}")).collect();
            format!("\n- {project}: {total} ({})", by_state.join(", "))
        })
        .collect();
    format!("Active lanes:{lines}")
}
