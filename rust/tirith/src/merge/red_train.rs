//! A red train: several lanes are merged alone next; one lane goes back, unless main itself is red.
use super::train::{Riders, Train};
use super::{batch::waits_note, main_check::main_check, pipeline::close_pipeline, slot::requeue};
use crate::app::{Lane, Shared};
use crate::{config::Manifest, lanes, manager};
use serde_json::json;
pub(super) async fn red_train(
    app: &Shared,
    t: &Train,
    manifest: &Manifest,
    mut riders: Riders,
    name: &str,
    why: &str,
) {
    close_pipeline(app, &t.pipe, "failed");
    requeue(
        app,
        &riders.ticks,
        false,
        "its train was red: checked again",
    );
    if riders.merged.len() > 1 {
        requeue(
            app,
            &riders.merged,
            true,
            "its train was red: merged alone next",
        );
        app.merge_wake.notify_one();
        return;
    }
    let lane = riders.merged.remove(0);
    if let Some(st) = manifest.stages.iter().find(|s| s.name == *name) {
        if !main_check(app, &t.project, &t.slot, &t.base, st, false).await {
            return main_is_red(app, t, lane, name, why);
        }
    }
    lanes::send_back(app, &lane, why)
}
/// The stage fails on main too: main is marked red, the lane waits for a green main, and the manager hears it once. A lane that rode alone on a main already red at this commit (a trial) did not fix it: it goes back.
fn main_is_red(app: &Shared, t: &Train, lane: Lane, name: &str, why: &str) {
    let (pid, base) = (&t.project.id, &t.base);
    let key = format!("main_red:{pid}");
    let (was, sha) = (app.kv(&key), &base[..8.min(base.len())]);
    if was.as_deref().is_some_and(|w| w.ends_with(sha)) {
        let why = format!("main is red ('{name}' fails on main at {sha}) and your branch does not make it pass: fix it on your branch if it is yours to fix, else ask the manager with neobot ask.\n{why}");
        return lanes::send_back(app, &lane, &why);
    }
    app.set_kv(&key, Some(&format!("{name} at {sha}")));
    app.emit("plan", json!({"project": pid}));
    requeue(app, &[lane], true, &waits_note(sha));
    let _ = was.is_none().then(|| {
        manager::emit_event(app, pid, "main.red", &format!("main of {pid} is red: check '{name}' fails on main itself ({sha}). The autopilot pauses {pid} until main is green; fix main or retry the main pipeline."), None)
    });
}
