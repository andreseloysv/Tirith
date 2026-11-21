//! A role confined by its `writes` key (the Coach): its edits land only in the bots' folders and in the folders of Tirith's home `writes` names (`bots`, `lab`), never in a hidden folder there (an agent's settings could turn Tirith's hooks off).
use super::call::Call;
use crate::app::Shared;
use std::path::{Component, Path, PathBuf};

/// The folders of Tirith's home that `role`'s `writes` names, if it names any.
fn writes(app: &Shared, role: Option<&str>) -> Option<Vec<PathBuf>> {
    let names = app.cfg().factory.role::<Vec<String>>(role?, "writes")?;
    Some(names.iter().map(|n| app.home.join(n)).collect())
}

/// No `..` (it can end anywhere) and no hidden folder.
fn plain(rel: &Path) -> bool {
    rel.components().all(|c| match c {
        Component::Normal(n) => !n.to_string_lossy().starts_with('.'),
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
