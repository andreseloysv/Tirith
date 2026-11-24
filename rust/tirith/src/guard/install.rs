//! Installing a project's guard hooks.
use super::render::{HOOKS, MARKER};
use super::{git_dir::git_dir, immutable::immutable, intact, render::render, words::words};
use crate::app::Shared;
use crate::config::Project;
use std::path::Path;

/// Writes both hooks (mode 755) where they are missing or ours (an older guard is replaced); never over a hook that is not ours.
pub fn install(app: &Shared, project: &Project) -> Result<(), String> {
    let words = words(app, project);
    let Some(words) = (!words.is_empty()).then_some(words) else {
        return Ok(());
    };
    let dir = git_dir(&project.repo())
        .ok_or(format!("{} is not a git repo", project.repo().display()))?
        .join("hooks");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    for hook in HOOKS {
        keep_local(&dir, hook)?;
    }
    for hook in HOOKS {
        write(&dir.join(hook), &render(hook, &words))?;
    }
    intact(app, project)
}

/// The repository's own hook keeps running after ours, as <hook>.local.
fn keep_local(dir: &Path, hook: &str) -> Result<(), String> {
    let ours = |p: &Path| std::fs::read_to_string(p).is_ok_and(|t| t.contains(MARKER));
    let path = dir.join(hook);
    if path.exists() && !ours(&path) {
        let local = dir.join(format!("{hook}.local"));
        (!(local.exists())).then_some(()).ok_or_else(|| {
            format!(
                "{} is not OpenBot's hook and {} exists: merge them by hand",
                path.display(),
                local.display()
            )
        })?;
        immutable(&path, false);
        std::fs::rename(&path, &local).map_err(|e| format!("{}: {e}", path.display()))?;
    }
    Ok(())
}

/// Writes one hook, executable and immutable.
fn write(path: &Path, text: &str) -> Result<(), String> {
    immutable(path, false);
    std::fs::write(path, text).map_err(|e| format!("{}: {e}", path.display()))?;
    crate::os::set_mode(path, 0o755).map_err(|e| e.to_string())?;
    immutable(path, true);
    Ok(())
}
