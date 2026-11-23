//! OpenBot's own files and the agents' settings: no agent edits them, except in the parts of OpenBot's home agents write in (and the folders a confined role's `writes` names).
use super::call::Call;
use crate::app::Shared;
use crate::config::{expand, Config};
use std::path::{Component, Path};

/// The lanes' worktrees live inside OpenBot's home: with the bots' folders they are the parts of it agents write in. A path with `..` in it can end anywhere, so it is never taken for one of them.
pub(super) fn in_lanes(app: &Shared, cfg: &Config, path: &Path) -> bool {
    !path.components().any(|c| c == Component::ParentDir)
        && (cfg.projects.iter().any(|p| path.starts_with(p.lanes_dir()))
            || crate::bots::writable(&app.home, path))
}

/// An edit of a file under OpenBot's home or of an agent's settings file.
pub(super) fn guarded(
    app: &Shared,
    cfg: &Config,
    call: &Call,
    role: Option<&str>,
) -> Option<String> {
    let mut protected: Vec<String> = cfg
        .factory
        .commands()
        .iter()
        .filter_map(|(alias, _)| cfg.factory.agent::<String>(alias, "settings"))
        .map(|s| expand(&s).display().to_string())
        .collect();
    protected.push(format!("{}/", app.home.display()));
    let guarded = call.files.iter().find(|f| {
        !(in_lanes(app, cfg, Path::new(f)) || super::confined::named(app, role, Path::new(f)))
            && protected.iter().any(|p| f.starts_with(p.as_str()))
    });
    guarded.filter(|_| call.edit).map(|g| format!("{g} belongs to OpenBot or to an agent's settings: agents never change it. Ask the owner if it must change."))
}
