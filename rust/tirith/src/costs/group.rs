//! The runs added up by what a BI tool groups on: run, lane, item, issue key, model, day or project. Team and bot level: no person is ever a column.
use super::sum::{Run, Sum};
use crate::app::Shared;
use std::collections::{BTreeMap, HashMap};

pub const BY: [&str; 8] = [
    "item", "lane", "run", "issue", "model", "day", "project", "bot",
];

/// One row: its grouping columns and what its runs add up to.
pub struct Row {
    pub keys: Vec<(&'static str, String)>,
    pub sum: Sum,
}

/// Which `(project, step, plain text)` items the plans say are done.
pub fn done_items(app: &Shared, runs: &[Run]) -> HashMap<(String, String, String), bool> {
    let mut done = HashMap::new();
    let projects: std::collections::BTreeSet<&str> =
        runs.iter().map(|r| r.project.as_str()).collect();
    for project in projects {
        for step in crate::plan::read(app, project).unwrap_or_default() {
            for item in step.items {
                done.insert(
                    (
                        project.to_string(),
                        step.num.clone(),
                        crate::plan::plain(&item.text),
                    ),
                    item.done,
                );
            }
        }
    }
    done
}

/// The rows of `runs` grouped `by` one of `BY`, in the order of their keys; a run's own row for `run`, the runs with no issue key left out of `issue`.
pub fn group(runs: &[Run], by: &str, done: &HashMap<(String, String, String), bool>) -> Vec<Row> {
    let mut rows: BTreeMap<Vec<String>, Row> = BTreeMap::new();
    for (n, r) in runs.iter().enumerate() {
        let k = super::keys::of(r, by, done);
        let plan = by == "item" || by == "lane"; // a plan item or a lane: only lanes have one
        if (by == "issue" && r.issue.is_empty()) || (plan && r.kind != "lane") {
            continue;
        }
        let id = if by == "run" {
            vec![n.to_string()]
        } else {
            k.iter().map(|(_, v)| v.clone()).collect()
        };
        rows.entry(id)
            .or_insert_with(|| Row {
                keys: k,
                sum: Sum::default(),
            })
            .sum
            .add(r);
    }
    rows.into_values().collect()
}
