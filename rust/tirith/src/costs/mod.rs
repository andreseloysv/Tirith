//! What the lanes spent: every run of every lane (each attempt, a sent-back continuation too) leaves a row in `lane_runs` with its model, tokens and cost, so a plan item's cost is the sum of its lanes' runs and the cost per finished item is that for the items merged. `neobot costs` and `GET /api/costs` export it (CSV or JSON) grouped by item, lane, run, issue key, model, day or project; team and bot level only, never a person (docs/DAEMON.md).
mod api;
mod cli;
mod export;
mod group;
mod issue;
mod keys;
mod read;
mod record;
mod rows;
mod sum;

pub use api::routes;
pub use cli::cli;
pub use record::{counted, fresh, run_ended};

use crate::app::Shared;
use serde_json::{json, Value};
use std::collections::HashMap;

/// What each lane spent so far, by lane id, as the page and `neobot status` show it.
pub fn by_lane(app: &Shared) -> HashMap<String, Value> {
    let mut sums: HashMap<String, sum::Sum> = HashMap::new();
    rows::runs(app, "", "")
        .iter()
        .for_each(|r| sums.entry(r.lane.clone()).or_default().add(r));
    sums.into_iter().map(|(lane, s)| (lane, s.json())).collect()
}

/// What each item of `project`'s plan spent, by step and plain text of the item: the sum of all its lanes' runs.
pub fn by_item(app: &Shared, project: &str) -> HashMap<(String, String), Value> {
    let mut sums: HashMap<(String, String), sum::Sum> = HashMap::new();
    for r in rows::runs(app, project, "") {
        sums.entry((r.step.clone(), crate::plan::plain(&r.item)))
            .or_default()
            .add(&r);
    }
    sums.into_iter().map(|(k, s)| (k, s.json())).collect()
}

/// The cost of one plan item as the page and `plan show` print it.
pub fn of_item(spent: &HashMap<(String, String), Value>, step: &str, text: &str) -> Value {
    spent
        .get(&(step.to_string(), crate::plan::plain(text)))
        .cloned()
        .unwrap_or(json!(null))
}

#[cfg(test)]
#[path = "../../checks/costs.rs"]
mod tests;
