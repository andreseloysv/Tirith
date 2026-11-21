//! Adding a folder as a project, step by step.
use super::add_plan::{self, Plan};
use super::register::{of_folder, register};
use super::{block::block, folder::folder, project::project, repo};
use crate::app::{git, Shared};
use crate::config::Project;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

/// Adds the folder as a project; with `dry`, only says what adding it would do.
pub(super) async fn add(
    app: &Shared,
    raw: &str,
    name: &str,
    color: &str,
    dry: bool,
) -> Result<Value, String> {
    let cfg = app.cfg();
    let dir = folder(raw, &cfg.projects, &app.home)?;
    if let Some(p) = of_folder(&cfg.projects, &dir) {
        return Ok(shown(&dir, p, true, &[]));
    }
    let top = repo::git_top(&dir).await?;
    repo::not_inside(&dir, top.as_ref())?;
    let p = project(&dir, name, color, &cfg.projects)?;
    block(
        &std::fs::read_to_string(app.home.join("config.toml")).unwrap_or_default(),
        &p,
    )?; // config.toml takes it: only then touch the folder
    let plan = add_plan::of(app, &dir);
    if dry {
        return Ok(
            json!({"id": p.id, "name": p.name, "folder": dir, "color": p.color, "git": top.is_some(), "plan": !plan.start, "open": plan.open}),
        );
    }
    let mut notes = prepare(app, &dir, &p, top.as_ref(), &plan).await?;
    add_plan::hold(app, &p.id, plan.open);
    let (p, created) = register(app, p)?;
    add_plan::hold(app, &p.id, plan.open);
    notes.extend((plan.open > 0).then(|| {
        format!(
            "Its plan has {} open items: the autopilot waits until you resume it on the Overview.",
            plan.open
        )
    }));
    app.emit("projects", json!(super::list(app)));
    app.sched_wake.notify_one();
    Ok(shown(&dir, &p, !created, &notes))
}

/// The folder made ready: a git repository, the commit guard and a started plan; the notes on what was done.
async fn prepare(
    app: &Shared,
    dir: &Path,
    p: &Project,
    top: Option<&PathBuf>,
    plan: &Plan,
) -> Result<Vec<String>, String> {
    let mut notes = vec![];
    if top.is_none() {
        git(dir, &["init", "-q"]).await?;
        notes.push("Created a git repository in the folder.".to_string());
    }
    notes.extend(crate::guard::install(app, p).err());
    if plan.start {
        notes.push(add_plan::start(dir, &plan.file).await);
    }
    Ok(notes)
}

/// The project as the page and the command line show it.
fn shown(dir: &Path, p: &Project, exists: bool, notes: &[String]) -> Value {
    json!({"id": p.id, "name": p.name, "folder": dir, "color": p.color, "exists": exists, "notes": notes})
}
