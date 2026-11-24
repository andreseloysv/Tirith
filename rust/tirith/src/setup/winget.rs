//! What NeoBot needs of the system on Windows, installed with winget when the owner says so.
use super::{ask::Ask, find, run};

/// winget's package for a program NeoBot needs.
pub(super) fn id(program: &str) -> Option<&'static str> {
    match program {
        "git" => Some("Git.Git"),
        "npm" => Some("OpenJS.NodeJS.LTS"),
        _ => None,
    }
}

/// Makes the programs `missing` be there with winget; else what to do.
pub(super) fn provide(ask: &Ask, missing: &[&str]) -> Result<(), String> {
    let names = missing.join(" ");
    let ids: Option<Vec<&str>> = missing.iter().map(|m| id(m)).collect();
    let (Some(ids), Some(winget)) = (ids, find::program("winget")) else {
        return Err(format!(
            "{names} missing: install them (Git for Windows, Node.js)"
        ));
    };
    let failed = || format!("{names} missing: winget install {}", ids.join(" "));
    let page = ask.on_page().is_some(); // the owner's tap on the page was the yes
    (page
        || ask.person()
            && ask.yes_no(&format!("  {names} missing. Install it with winget?"), true))
    .then_some(())
    .ok_or_else(failed)?;
    for id in &ids {
        let mut args = vec![
            "install",
            "--id",
            id,
            "-e",
            "--accept-source-agreements",
            "--accept-package-agreements",
        ];
        if page {
            args.extend(["--silent", "--disable-interactivity"]); // no installer window, only Windows' own yes
        }
        run::shown(ask, &winget, &args, &[])
            .then_some(())
            .ok_or_else(failed)?;
    }
    let found: Option<Vec<_>> = missing.iter().map(|m| find::program(m)).collect();
    found.ok_or_else(failed)?.iter().for_each(|p| on_path(p));
    Ok(())
}

/// The folder of a program just installed, on this process's PATH: what it starts (an agent that needs Git's bash) finds it without a restart.
fn on_path(program: &std::path::Path) {
    let (Some(dir), path) = (
        program.parent(),
        std::env::var_os("PATH").unwrap_or_default(),
    ) else {
        return;
    };
    if std::env::split_paths(&path).any(|d| d == dir) {
        return;
    }
    let all = std::iter::once(dir.to_path_buf()).chain(std::env::split_paths(&path));
    if let Ok(joined) = std::env::join_paths(all) {
        std::env::set_var("PATH", joined); // Windows: the environment is behind std's own lock
    }
}
