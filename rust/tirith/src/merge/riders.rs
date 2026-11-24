//! Who rides a train: lanes with commits are merged, lanes without are ticked if their check passes on main.
use super::stage::run_stage;
use super::train::{item_check, Train};
use crate::app::{git, Lane, Shared};
use crate::lanes;
/// The lanes with commits ahead of main (to merge), and those without whose item's check passes on main (to tick); the rest go back.
pub(super) async fn sort_out(app: &Shared, t: &Train, batch: Vec<Lane>) -> (Vec<Lane>, Vec<Lane>) {
    let (repo, pid) = (t.project.repo(), &t.project.id);
    let (mut todo, mut ticks) = (vec![], vec![]);
    for lane in batch {
        let ahead = git(
            &repo,
            &[
                "rev-list",
                "--count",
                &format!("{}..{}", t.base, lane.branch),
            ],
        )
        .await
        .unwrap_or_default();
        if !matches!(ahead.as_str(), "0" | "") {
            todo.push(lane);
        } else if let Some(c) = item_check(app, pid, &lane) {
            match run_stage(app, &t.pipe, &t.slot, &format!("check {}", lane.id), &c).await {
                Ok(()) => ticks.push(lane),
                Err(e) => lanes::send_back(app, &lane, &format!("nothing to merge, and the item's check fails on main: commit your work, or ask the manager with neobot ask.\n{e}")),
            }
        } else {
            lanes::send_back(
                app,
                &lane,
                "nothing to merge: commit your work, or ask the manager with neobot ask",
            );
        }
    }
    (todo, ticks)
}
/// Each branch merged into the slot; one that conflicts is aborted and its lane goes back.
pub(super) async fn merge_branches(app: &Shared, t: &Train, todo: Vec<Lane>) -> Vec<Lane> {
    let mut merged = vec![];
    for lane in todo {
        let msg = format!(
            "Merge {}: {}",
            lane.branch,
            lane.item.chars().take(120).collect::<String>()
        );
        match git(
            &t.slot,
            &["merge", "--no-ff", "-q", "-m", &msg, &lane.branch],
        )
        .await
        {
            Ok(_) => merged.push(lane),
            Err(e) => {
                let _ = git(&t.slot, &["merge", "--abort"]).await;
                lanes::send_back(
                    app,
                    &lane,
                    &format!(
                        "conflict with main: merge main into your branch and resolve it.\n{e}"
                    ),
                );
            }
        }
    }
    merged
}
