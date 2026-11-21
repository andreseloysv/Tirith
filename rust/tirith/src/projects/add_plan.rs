//! The plan of a project being added: what it has, and its start when it has none.
use crate::app::{git, Shared};
use std::path::{Component, Path, PathBuf};

pub(super) struct Plan {
    pub file: PathBuf,
    /// Its open items.
    pub open: usize,
    /// There is no plan yet, and it would be inside the folder: adding starts it.
    pub start: bool,
}

/// The plan the manifest of `dir` names.
pub(super) fn of(app: &Shared, dir: &Path) -> Plan {
    let file = dir.join(app.manifest(dir).roadmap);
    let open = std::fs::read_to_string(&file)
        .map(|t| {
            crate::plan::parse(&t)
                .iter()
                .flat_map(|s| &s.items)
                .filter(|i| !i.done)
                .count()
        })
        .unwrap_or(0);
    let start = !file.exists()
        && file.starts_with(dir)
        && !file.components().any(|c| c == Component::ParentDir); // never outside the folder
    Plan { file, open, start }
}

/// Writes and commits a plan with a first step: the note that says how it went.
pub(super) async fn start(dir: &Path, plan: &Path) -> String {
    let rel = plan.strip_prefix(dir).unwrap_or(plan).display().to_string();
    commit_first(dir, plan, &rel).await.map_or_else(
        |e| format!("Its plan {rel} could not be started: {e}"),
        |_| format!("Started its plan in {rel}: add the first items on the Overview."),
    )
}

async fn commit_first(dir: &Path, plan: &Path, rel: &str) -> Result<String, String> {
    plan.parent()
        .map_or(Ok(()), std::fs::create_dir_all)
        .and_then(|_| std::fs::write(plan, "# Plan\n\n## 1. First steps\n"))
        .map_err(|e| e.to_string())?;
    git(dir, &["-c", "core.fsmonitor=false", "add", "--", rel]).await?;
    git(
        dir,
        &[
            "-c",
            "core.fsmonitor=false",
            "commit",
            "-q",
            "-m",
            "Plan: start the plan",
            "--",
            rel,
        ],
    )
    .await
}

/// A plan that already has open items: agents start on it only once the owner resumes the autopilot.
pub(super) fn hold(app: &Shared, id: &str, open: usize) {
    let _ = (open > 0).then(|| app.set_kv(&format!("autopilot:{id}"), Some("off")));
}
