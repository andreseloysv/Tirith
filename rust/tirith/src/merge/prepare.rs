//! Readying the merge slot on main's head for a train, and what happens to its lanes when that fails.
use super::slot::{disk_full, merge_slot, reset};
use crate::app::{git, Lane, Shared};
use crate::config::{Manifest, Project};
use crate::{lanes, manager};
use std::path::PathBuf;
/// The merge slot reset to main's head, and that head; `None` once the lanes were dealt with.
pub(super) async fn ready(
    app: &Shared,
    project: &Project,
    manifest: &Manifest,
    batch: &[Lane],
) -> Option<(PathBuf, String)> {
    let ready = async {
        let slot = merge_slot(project, manifest).await?;
        let base = git(&project.repo(), &["rev-parse", "HEAD"]).await?;
        reset(&slot, &base).await?;
        Ok::<_, String>((slot, base))
    };
    match ready.await {
        Ok(x) => Some(x),
        Err(e) if disk_full(&e) => {
            wait_for_disk(app, &project.id, batch, &e);
            None
        }
        Err(e) => {
            broken_slot(app, &project.id, batch, &e);
            None
        }
    }
}
/// Out of disk is not the lanes' fault: they leave the queue (no slot held) and wait for space.
fn wait_for_disk(app: &Shared, pid: &str, batch: &[Lane], e: &str) {
    batch.iter().for_each(|l| {
        drop(app.update_lane(&l.id, |x| {
            (x.status, x.note) = (
                "waiting-disk".into(),
                "the merge worktree could not be made: no space left on device".into(),
            )
        }))
    });
    manager::emit_event(
        app,
        pid,
        "merge.blocked",
        &format!("the merge of {pid} waits for disk: {e}"),
        None,
    )
}
/// Any other failure: the lanes fail (their worktrees go later) and the manager hears the slot is broken.
fn broken_slot(app: &Shared, pid: &str, batch: &[Lane], e: &str) {
    batch.iter().for_each(|l| {
        drop(
            app.update_lane(&l.id, |x| x.status = "failed".into())
                .inspect(|l| lanes::cleanup_later(app, l.clone(), false, 0)),
        )
    });
    manager::emit_event(
        app,
        pid,
        "merge.blocked",
        &format!("the merge slot of {pid} is broken: {e}"),
        None,
    )
}
