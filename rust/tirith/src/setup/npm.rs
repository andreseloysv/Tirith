//! A maker's npm installer, where its script's build cannot run: on Windows (the script is for Mac and Linux), on musl (Alpine) Copilot's script downloads the glibc one, and its npm package brings the musl one.
use super::{ask::Ask, catalog::Known, find, packages, run};
use std::path::Path;

/// The package to install with npm here; none where the maker's script serves.
pub(super) fn package(known: &Known) -> Option<&str> {
    (!known.npm_on_musl.is_empty() && (cfg!(windows) || musl()))
        .then_some(known.npm_on_musl.as_str())
}

/// True where the system's C library is musl's.
fn musl() -> bool {
    let found = |dir: &str, start: &str| {
        let names = std::fs::read_dir(dir).into_iter().flatten().flatten();
        names
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .any(|name| name.starts_with(start))
    };
    cfg!(target_os = "linux")
        && found("/lib", "ld-musl-")
        && !found("/lib", "ld-linux")
        && !found("/lib64", "ld-linux")
}

/// Installs it in the owner's ~/.local, like the script would.
pub(super) fn install(ask: &Ask, package: &str) -> Result<(), String> {
    if find::program("npm").is_none() {
        packages::provide(ask, &["npm"])?;
    }
    let npm = find::program("npm").ok_or("npm is missing")?;
    node_new_enough()?;
    let prefix = crate::config::home_dir().join(".local");
    std::fs::create_dir_all(prefix.join("bin"))
        .map_err(|e| format!("{}: {e}", prefix.display()))?;
    let to = prefix.display().to_string();
    let args = [
        "install",
        "-g",
        "--prefix",
        &to,
        "--no-fund",
        "--no-audit",
        package,
    ];
    run::shown(ask, Path::new(&npm), &args, &[])
        .then_some(())
        .ok_or(format!("npm could not install {package}"))
}

/// The makers' npm agents (Copilot) run on Node.js 22 or newer: an older one is said before anything installs.
fn node_new_enough() -> Result<(), String> {
    const NEEDED: u32 = 22;
    let said =
        find::program("node").and_then(|n| run::output(&n.display().to_string(), &["--version"]));
    let major = said.as_deref().and_then(|v| {
        v.trim_start_matches('v')
            .split('.')
            .next()?
            .parse::<u32>()
            .ok()
    });
    match major {
        Some(m) if m < NEEDED => Err(format!(
            "it needs Node.js {NEEDED} or newer, and this computer has Node.js {m}: update Node.js, then press Try again"
        )),
        _ => Ok(()),
    }
}
