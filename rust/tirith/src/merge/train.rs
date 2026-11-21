//! A train: up to five lanes of one project merged together in the merge slot, checked once, then landed on main.
use super::pipeline::{close_pipeline, open_pipeline, Pipe};
use super::riders::{merge_branches, sort_out};
use super::train_checks::{changed_files, stages_to_run};
use super::{land::land, prepare::ready, red_train::red_train, slot::requeue, stage::run_stage};
use crate::app::{Lane, Shared};
use crate::{config::Project, plan};
use std::path::PathBuf;
/// Where a train runs: its project, its pipeline, the merge slot and the main commit it starts from.
pub(super) struct Train {
    pub(super) project: Project,
    pub(super) pipe: Pipe,
    pub(super) slot: PathBuf,
    pub(super) base: String,
}
/// The lanes still on a train: merged branches, and lanes with nothing to merge whose check passed on main.
pub(super) struct Riders {
    pub(super) merged: Vec<Lane>,
    pub(super) ticks: Vec<Lane>,
}
impl Riders {
    pub(super) fn done(&self) -> Vec<Lane> {
        self.merged.iter().chain(&self.ticks).cloned().collect()
    }
}
/// The item's check of a lane, from the plan.
pub(super) fn item_check(app: &Shared, project: &str, l: &Lane) -> Option<String> {
    plan::find(app, project, &l.step, &l.item).and_then(|i| i.check)
}
pub(super) async fn train(app: &Shared, batch: Vec<Lane>) {
    let pid = batch[0].project.clone();
    batch
        .iter()
        .for_each(|l| drop(app.update_lane(&l.id, |x| x.status = "merging".into())));
    let Some(project) = app.cfg().project(&pid).cloned() else {
        return requeue(app, &batch, false, "unknown project");
    };
    let manifest = app.manifest(&project.repo());
    plan::flush(app, Some(&pid)).await;
    let Some((slot, base)) = ready(app, &project, &manifest, &batch).await else {
        return;
    };
    let ids: Vec<&str> = batch.iter().map(|l| l.id.as_str()).collect();
    let pipe = open_pipeline(app, &pid, "merge", &ids.join(","), &base);
    let t = Train {
        project,
        pipe,
        slot,
        base,
    };
    let (todo, ticks) = sort_out(app, &t, batch).await;
    let merged = merge_branches(app, &t, todo).await;
    let riders = Riders { merged, ticks };
    let Some(()) = (!riders.merged.is_empty() || !riders.ticks.is_empty()).then_some(()) else {
        return close_pipeline(app, &t.pipe, "failed");
    };
    let changed = changed_files(&t).await;
    for (name, run) in &stages_to_run(app, &t, &manifest, &riders, &changed) {
        if let Err(why) = run_stage(app, &t.pipe, &t.slot, name, run).await {
            return red_train(app, &t, &manifest, riders, name, &why).await;
        }
    }
    land(app, &t, riders, &changed).await;
}
