//! A role confined by its `writes` key (the Coach): its edits land only in the bots' folders and in the folders of NeoBot's home `writes` names (`bots`, `lab`), never in a hidden folder there but a skill's own (an agent's settings could turn NeoBot's hooks off).
use super::call::Call;
use crate::app::Shared;
use std::path::{Component, Path, PathBuf};

/// The folders of NeoBot's home that `role`'s `writes` names, if it names any.
fn writes(app: &Shared, role: Option<&str>) -> Option<Vec<PathBuf>> {
    let names = app.cfg().factory.role::<Vec<String>>(role?, "writes")?;
    Some(names.iter().map(|n| app.home.join(n)).collect())
}

/// No `..` (it can end anywhere) and no hidden folder but a skill's (`.agents/skills/…`, in a draft worktree of the lab as in a bot's folder).
fn plain(rel: &Path) -> bool {
    let parts: Vec<Component> = rel.components().collect();
    parts.iter().enumerate().all(|(i, c)| match c {
        Component::Normal(n) if n.to_string_lossy().starts_with('.') => {
            let skills = Component::Normal("skills".as_ref());
            *n == ".agents" && parts.get(i + 1) == Some(&skills) && parts.len() > i + 2
        }
        Component::Normal(_) => true,
        _ => false,
    })
}

/// `path` is in one of the folders `role`'s `writes` names.
pub(super) fn named(app: &Shared, role: Option<&str>, path: &Path) -> bool {
    writes(app, role).is_some_and(|w| {
        w.iter().any(|d| {
            path.strip_prefix(d)
                .is_ok_and(|r| !r.as_os_str().is_empty() && plain(r))
        })
    })
}

/// An edit of a confined role outside the bots' folders and its `writes` folders.
pub(super) fn refused(app: &Shared, call: &Call, role: Option<&str>) -> Option<String> {
    let folders = writes(app, role)?;
    call.edit.then_some(())?;
    let inside = |f: &str| {
        let p = Path::new(f);
        named(app, role, p)
            || (!p.components().any(|c| c == Component::ParentDir)
                && crate::bots::writable(&app.home, p))
    };
    let out = call.files.iter().find(|f| !inside(f))?;
    let list: Vec<String> = folders.iter().map(|d| d.display().to_string()).collect();
    Some(format!(
        "{out}: {} writes only in the bots' folders ({}/manager/<bot>) and in {}.",
        role.unwrap_or_default(),
        app.home.display(),
        list.join(", ")
    ))
}
