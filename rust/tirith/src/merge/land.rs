//! A green train: under the plan lock, pending plan edits are committed on main, the items are ticked in the merge slot, and main fast-forwards to the result.
use super::train::{Riders, Train};
use super::{landed::landed, pipeline::close_pipeline, slot::requeue};
use crate::app::{git, now_iso, Lane, Shared};
use crate::{manager, plan};
pub(super) async fn land(app: &Shared, t: &Train, riders: Riders, changed: &[String]) {
    let done = riders.done();
    let blocked = format!("main_blocked:{}", t.project.id);
    match move_main(app, t, &done).await {
        Ok(sha) => {
            app.set_kv(&blocked, None);
            landed(app, t, riders, &sha, changed).await
        }
        Err(e) => cannot_move(app, t, &done, &blocked, &e),
    }
}
/// Main's head with the train on it and the items ticked; main may only have moved in the plan meanwhile.
async fn move_main(app: &Shared, t: &Train, done: &[Lane]) -> Result<String, String> {
    let (repo, slot) = (t.project.repo(), &t.slot);
    let roadmap = app.manifest(&repo).roadmap;
    let roadmap = roadmap.trim_start_matches("./").to_string();
    let _plan = app.plan_lock.lock().await;
    plan::flush_locked(app, &t.project.id).await;
    let head = git(&repo, &["rev-parse", "HEAD"]).await?;
    if head != t.base {
        (!git(&repo, &["diff", "--name-only", &t.base, &head])
            .await?
            .lines()
            .any(|f| f != roadmap))
        .then_some(())
        .ok_or("main moved while the checks ran".to_string())?;
        git(slot, &["merge", "-q", "--no-edit", &head]).await?; // only plan edits: no new check needed
    }
    tick_items(slot, &roadmap, done).await?;
    let sha = git(slot, &["rev-parse", "HEAD"]).await?;
    git(&repo, &["merge", "--ff-only", "-q", &sha]).await?;
    Ok(sha)
}
/// The lanes' items ticked in the slot's plan, in one commit when any changed.
async fn tick_items(slot: &std::path::Path, roadmap: &str, done: &[Lane]) -> Result<(), String> {
    let file = slot.join(roadmap);
    let items: Vec<(String, String)> = done
        .iter()
        .map(|l| (l.step.clone(), l.item.clone()))
        .collect();
    let (text, n) = plan::tick_text(
        &std::fs::read_to_string(&file).map_err(|e| e.to_string())?,
        &items,
    );
    if n > 0 {
        std::fs::write(&file, text).map_err(|e| e.to_string())?;
        let msg = format!("Plan: done {n} item(s)");
        git(slot, &["commit", "-q", "-m", &msg, "--", roadmap]).await?;
    }
    Ok(())
}
/// Main that cannot move (local changes in the checkout are in the way, say) is retried quietly, but its manager hears it once, with git's words, until main moves again.
fn cannot_move(app: &Shared, t: &Train, done: &[Lane], blocked: &str, e: &str) {
    let project = &t.project;
    close_pipeline(app, &t.pipe, "failed");
    if app.kv(blocked).is_none() {
        app.set_kv(blocked, Some(&now_iso()));
        let ids = done
            .iter()
            .map(|l| l.id.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        let why = format!("main of {} cannot move, so lanes that passed their checks wait ({ids}). Git says: {e}\nTirith retries by itself. If files in {} are in the way, commit, move or discard them (ask the owner first when they may be the owner's work).", project.id, project.repo().display());
        manager::emit_event(app, &project.id, "merge.blocked", &why, None);
    }
    requeue(app, done, false, &format!("main could not move: {e}"))
}
