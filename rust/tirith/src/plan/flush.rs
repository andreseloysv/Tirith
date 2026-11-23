//! Committing the plan files OpenBot changed.
use super::lookup::path;
use crate::app::{git, Shared};

/// Commits the plan files changed since the last commit: one commit per project, whatever the number of changes.
pub async fn flush(app: &Shared, only: Option<&str>) {
    let projects: Vec<String> = app
        .plan_dirty
        .lock()
        .unwrap()
        .iter()
        .filter(|p| only.is_none_or(|o| o == *p))
        .cloned()
        .collect();
    for project in projects {
        let _guard = app.plan_lock.lock().await;
        flush_locked(app, &project).await;
    }
}

/// `flush` of one project for a caller that already holds the plan lock.
pub async fn flush_locked(app: &Shared, project: &str) {
    let Some(()) = app
        .plan_dirty
        .lock()
        .unwrap()
        .contains(project)
        .then_some(())
    else {
        return;
    };
    let (Ok(file), Some(repo)) = (
        path(app, project),
        app.cfg().project(project).map(|p| p.repo()),
    ) else {
        return;
    };
    let rel = file
        .strip_prefix(&repo)
        .unwrap_or(&file)
        .to_string_lossy()
        .to_string();
    let _ = commit(&repo, &rel)
        .await
        .map(|_| app.plan_dirty.lock().unwrap().remove(project))
        .map_err(|e| {
            app.event(
                "",
                project,
                "error",
                "plan",
                &format!("plan commit failed: {e}"),
                "",
            )
        });
}

/// Commits the plan file `rel` of `repo` when it has changes.
async fn commit(repo: &std::path::Path, rel: &str) -> Result<String, String> {
    let pending = git(repo, &["status", "--porcelain", "--", rel])
        .await
        .unwrap_or_default();
    if pending.is_empty() {
        return Ok(String::new());
    }
    git(
        repo,
        &["commit", "-q", "-m", "Plan: update from OpenBot", "--", rel],
    )
    .await
}
