//! Which lanes ride the next train.
use crate::app::{git, Lane, Shared};
use std::collections::BTreeSet;
/// The oldest merge-queued lane alone when it is solo, else up to five non-solo lanes of its project. Projects whose main is red wait until main changes (then the main check decides again).
pub(super) async fn next_batch(app: &Shared) -> Vec<Lane> {
    let mut queued = queued(app);
    let projects: BTreeSet<String> = queued.iter().map(|l| l.project.clone()).collect();
    for p in projects {
        if red_on_head(app, &p).await {
            queued.retain(|l| l.project != p);
        }
    }
    let Some(first) = queued.first().cloned() else {
        return vec![];
    };
    let Some(()) = (!first.solo).then_some(()) else {
        return vec![first];
    };
    queued
        .into_iter()
        .filter(|l| l.project == first.project && !l.solo)
        .take(app.cfg().factory.limits.merge_train)
        .collect()
}
/// The merge-queued lanes, oldest first.
fn queued(app: &Shared) -> Vec<Lane> {
    let mut queued: Vec<Lane> = app
        .lanes
        .lock()
        .unwrap()
        .values()
        .filter(|l| l.status == "merge-queued")
        .cloned()
        .collect();
    queued.sort_by(|a, b| a.ended_at.cmp(&b.ended_at));
    queued
}
/// A project whose main is red at its current head (or whose head is unknown) holds its lanes back.
async fn red_on_head(app: &Shared, p: &str) -> bool {
    let Some(why) = app.kv(&format!("main_red:{p}")) else {
        return false;
    };
    let head = if let Some(r) = app.cfg().project(p).map(|p| p.repo()) {
        git(&r, &["rev-parse", "HEAD"]).await.unwrap_or_default()
    } else {
        String::new()
    };
    head.len() < 8 || why.ends_with(&head[..8])
}
