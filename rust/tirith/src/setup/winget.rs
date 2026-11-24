//! What OpenBot needs of the system on Windows, installed with winget when the owner says so.
use super::{ask::Ask, find, run};

/// winget's package for a program OpenBot needs.
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
    ask.person()
        .then_some(())
        .filter(|_| ask.yes_no(&format!("  {names} missing. Install it with winget?"), true))
        .ok_or_else(failed)?;
    for id in &ids {
        let args = [
            "install",
            "--id",
            id,
            "-e",
            "--accept-source-agreements",
            "--accept-package-agreements",
        ];
        run::shown(ask, &winget, &args, &[])
            .then_some(())
            .ok_or_else(failed)?;
    }
    missing
        .iter()
        .all(|m| find::program(m).is_some())
        .then_some(())
        .ok_or_else(failed)
}
