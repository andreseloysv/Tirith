//! Which lanes ride the next train.
use crate::app::{git, Lane, Shared};
use std::collections::BTreeSet;
/// The oldest merge-queued lane alone when it is solo, else up to five non-solo lanes of its project. A project whose main is red rides only its oldest lane, alone, as a trial: the lane that fixes main gets through, one that fails goes back.
pub(super) async fn next_batch(app: &Shared) -> Vec<Lane> {
    let mut queued = queued(app);
    let projects: BTreeSet<String> = queued.iter().map(|l| l.project.clone()).collect();
    for p in projects {
        if let Some(head8) = red_head(app, &p).await {
            queued = red_trial(queued, &p, &head8);
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
/// With main red at `head8`, a project keeps only its oldest lane that has not already failed on that red main, marked solo so it rides alone; an unknown head ("") holds every lane back.
pub(super) fn red_trial(queued: Vec<Lane>, p: &str, head8: &str) -> Vec<Lane> {
    let mut trial = false;
    queued
        .into_iter()
        .filter_map(|mut l| {
            if l.project != p {
                return Some(l);
            }
            let pick = !trial && !head8.is_empty() && !waits_at(&l.note, head8);
            trial |= pick;
            l.solo |= pick;
            pick.then_some(l)
        })
        .collect()
}
/// The note of a lane that failed alone where main itself is red at `sha8`.
pub(super) fn waits_note(sha8: &str) -> String {
    format!("main is red at {sha8}: waits for a green main")
}
fn waits_at(note: &str, head8: &str) -> bool {
    note == waits_note(head8)
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
/// Main's head (8 chars) when the project's main is red at it, "" when red and the head is unknown, None when not red at its head.
async fn red_head(app: &Shared, p: &str) -> Option<String> {
    let why = app.kv(&format!("main_red:{p}"))?;
    let head = if let Some(r) = app.cfg().project(p).map(|p| p.repo()) {
        git(&r, &["rev-parse", "HEAD"]).await.unwrap_or_default()
    } else {
        String::new()
    };
    if head.len() < 8 {
        return Some(String::new());
    }
    why.ends_with(&head[..8]).then(|| head[..8].to_string())
}
