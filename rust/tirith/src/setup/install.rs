//! Installing an agent with its maker's own installer, as the owner, in their own folder.
use super::{ask::Ask, catalog::Known, find, run};
use std::path::Path;

/// Downloads the installer over HTTPS into OpenBot's home, then runs it: never a pipe into a shell, which would run half a script of a cut download.
pub fn install(ask: &Ask, known: &Known, home: &Path) -> Result<(), String> {
    if let Some(package) = super::npm::package(known) {
        return super::npm::install(ask, package);
    }
    // On Windows the maker's PowerShell script, run by powershell.exe; elsewhere its shell script, run by bash.
    let (shell, url, ext, run_args): (_, _, _, &[&str]) = if cfg!(windows) {
        let flags = &["-NoProfile", "-ExecutionPolicy", "Bypass", "-File"];
        ("powershell", &known.install_ps1, "ps1", flags)
    } else {
        ("bash", &known.install, "sh", &[])
    };
    let (bash, curl) = (
        find::program(shell).ok_or(format!("{shell} is missing"))?,
        find::program("curl").ok_or("curl is missing")?,
    );
    if !url.starts_with("https://") {
        return Err(format!("no installer is known for {}", known.name));
    }
    let dir = home.join("setup");
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let file = dir.join(format!("install-{}.{ext}", known.alias));
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
    let got = run::shown(ask, &curl, &[&get[..], &["-o", &to, url]].concat(), &[]);
    got.then_some(()).ok_or(format!("cannot download {url}"))?;
    let mut env: Vec<(String, String)> = known
        .installer_env
        .iter()
        .filter_map(|(k, v)| Some((k.clone(), v.as_str()?.to_string())))
        .collect();
    // The folder installers put programs in is on PATH for them: none of them stops to ask about PATH then.
    let bin = crate::config::home_dir().join(".local/bin");
    let _ = std::fs::create_dir_all(&bin);
    let path = std::env::var("PATH").unwrap_or_default();
    env.push(("PATH".into(), put_first(&bin, &path)));
    let done = run::shown(ask, &bash, &[run_args, &[&to]].concat(), &env);
    let _ = std::fs::remove_file(&file);
    done.then_some(())
        .ok_or(format!("{}'s installer failed", known.name))
}

/// `path` with `dir` in front, joined with the system's separator.
pub(super) fn put_first(dir: &Path, path: &str) -> String {
    let all = std::iter::once(dir.to_path_buf()).chain(std::env::split_paths(path));
    std::env::join_paths(all).map_or_else(
        |_| path.to_string(),
        |joined| joined.to_string_lossy().into_owned(),
    )
}
