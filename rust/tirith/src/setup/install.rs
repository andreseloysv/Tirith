//! Installing an agent with its maker's own installer, as the owner, in their own folder.
use super::{ask::Ask, catalog::Known, find, run};
use std::path::Path;

/// Downloads the installer over HTTPS into OpenBot's home, then runs it: never a pipe into a shell, which would run half a script of a cut download.
pub fn install(ask: &Ask, known: &Known, home: &Path) -> Result<(), String> {
    if let Some(package) = super::npm::package(known) {
        return super::npm::install(ask, package);
    }
    let (bash, curl) = (
        find::program("bash").ok_or("bash is missing")?,
        find::program("curl").ok_or("curl is missing")?,
    );
    if !known.install.starts_with("https://") {
        return Err(format!("no installer is known for {}", known.name));
    }
    let dir = home.join("setup");
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let file = dir.join(format!("install-{}.sh", known.alias));
    let to = file.display().to_string();
    let get = [
        "-fsSL",
        "--proto",
        "=https",
        "--proto-redir",
        "=https",
        "-m",
        "120",
    ];
    let got = run::shown(
        ask,
        &curl,
        &[&get[..], &["-o", &to, &known.install]].concat(),
        &[],
    );
    got.then_some(())
        .ok_or(format!("cannot download {}", known.install))?;
    let mut env: Vec<(String, String)> = known
        .installer_env
        .iter()
        .filter_map(|(k, v)| Some((k.clone(), v.as_str()?.to_string())))
        .collect();
    // The folder installers put programs in is on PATH for them: none of them stops to ask about PATH then.
    let bin = crate::config::home_dir().join(".local/bin");
    let _ = std::fs::create_dir_all(&bin);
    let path = std::env::var("PATH").unwrap_or_default();
    env.push(("PATH".into(), format!("{}:{path}", bin.display())));
    let done = run::shown(ask, &bash, &[&to], &env);
    let _ = std::fs::remove_file(&file);
    done.then_some(())
        .ok_or(format!("{}'s installer failed", known.name))
}
