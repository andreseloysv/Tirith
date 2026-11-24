//! Writing a new project into config.toml.
use super::block::block;
use super::config_file::{read, WRITING};
use super::project::unique_id;
use crate::app::Shared;
use crate::config::Project;
use crate::os::Mode;
use std::io::Write;
use std::path::Path;

/// The project whose folder is `dir`, if there is one.
pub(super) fn of_folder<'a>(projects: &'a [Project], dir: &Path) -> Option<&'a Project> {
    projects
        .iter()
        .find(|p| !p.repo.is_empty() && p.repo() == dir)
}

/// Appends the project to config.toml and reloads it. The project that has the folder (another add may have won).
pub(super) fn register(app: &Shared, mut p: Project) -> Result<(Project, bool), String> {
    let _one = WRITING.lock().unwrap();
    app.reload_config(true);
    let cfg = app.cfg();
    if let Some(old) = of_folder(&cfg.projects, &p.repo()) {
        return Ok((old.clone(), false));
    }
    p.id = unique_id(&p.name, &cfg.projects);
    append(&app.home.join("config.toml"), &p)?;
    app.reload_config(true);
    Ok((p, true))
}

/// Appends the project's block to the file, created readable by the owner only when it is not there.
fn append(file: &Path, p: &Project) -> Result<(), String> {
    let more = block(&read(file)?, p)?;
    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .with_mode(0o600)
        .open(file)
        .and_then(|mut f| f.write_all(more.as_bytes()))
        .map_err(|e| format!("{}: {e}", file.display()))
}
