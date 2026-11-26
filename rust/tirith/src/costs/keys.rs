//! The columns of a row of `lane_runs` for each way to group them.
use super::sum::Run;
use std::collections::HashMap;

pub(super) fn of(
    r: &Run,
    by: &str,
    done: &HashMap<(String, String, String), bool>,
) -> Vec<(&'static str, String)> {
    let item = crate::plan::plain(&r.item);
    let at = |k: &'static str, v: &str| (k, v.to_string());
    match by {
        "run" => vec![
            at("lane", &r.lane),
            ("attempt", r.attempt.to_string()),
            at("project", &r.project),
            at("step", &r.step),
            at("item", &item),
            at("issue", &r.issue),
            at("agent", &r.agent),
            at("model", &r.model),
            at("started_at", &r.started_at),
            at("ended_at", &r.ended_at),
            at("source", &r.source),
        ],
        "lane" => vec![
            at("project", &r.project),
            at("lane", &r.lane),
            at("step", &r.step),
            at("item", &item),
            at("issue", &r.issue),
        ],
        "issue" => vec![at("project", &r.project), at("issue", &r.issue)],
        "model" => vec![at("model", &r.model)],
        "day" => vec![at("day", r.ended_at.get(..10).unwrap_or(""))],
        "project" => vec![at("project", &r.project)],
        _ => {
            let finished = done
                .get(&(r.project.clone(), r.step.clone(), item.clone()))
                .copied()
                .unwrap_or(false);
            vec![
                at("project", &r.project),
                at("step", &r.step),
                at("item", &item),
                at("issue", &r.issue),
                ("done", finished.to_string()),
            ]
        }
    }
}
