//! The queue's loop: it rechecks a red main, rides the next train, or deploys what is owed.
use super::{batch::next_batch, deploy::deploy, locks::BUSY, red::recheck_red, train::train};
use crate::app::{now_iso, now_s, Shared};
use crate::config::Project;
use rusqlite::params;
use std::time::Duration;
pub async fn worker(app: Shared) {
    restart_cut_short(&app);
    loop {
        recheck_red(&app).await;
        let batch = next_batch(&app).await;
        if batch.is_empty() {
            idle(&app).await;
            continue;
        }
        let _busy = BUSY.lock().await;
        train(&app, batch).await;
    }
}
/// A merge cut short by a restart starts over, and its pipeline and runs, left "running" by the restart, end as interrupted.
fn restart_cut_short(app: &Shared) {
    let cut: Vec<String> = app
        .lanes
        .lock()
        .unwrap()
        .values()
        .filter(|l| l.status == "merging")
        .map(|l| l.id.clone())
        .collect();
    cut.iter()
        .for_each(|id| drop(app.update_lane(id, |l| l.status = "merge-queued".into())));
    ["pipelines", "runs"].into_iter().for_each(|t| {
        _ = app.insert(
            &format!(
                "UPDATE {t} SET status = 'interrupted', ended_at = ?1 WHERE status = 'running'"
            ),
            params![now_iso()],
        )
    });
}
/// Nothing queued: wait for a lane (lanes ending together ride one train), then deploy what is due.
async fn idle(app: &Shared) {
    if tokio::time::timeout(Duration::from_secs(10), app.merge_wake.notified())
        .await
        .is_ok()
    {
        tokio::time::sleep(Duration::from_secs(1)).await; // lanes ending together ride one train
    }
    // A deploy that failed, or that a restart cut short, is tried again on main as it is now. The lock comes first: a deploy that the page asked for has settled what is due by the time the lock is had.
    let _busy = BUSY.lock().await;
    let due = |p: &&Project| {
        app.kv(&format!("deploy_due:{}", p.id))
            .and_then(|v| v.split('@').next()?.parse().ok())
            .is_some_and(|t: u64| now_s() >= t)
    };
    if let Some(project) = app.cfg().projects.iter().find(due).cloned() {
        deploy(app, &project, &[]).await;
    }
}
