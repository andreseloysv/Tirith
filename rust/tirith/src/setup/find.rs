//! Where an agent's program is on this machine.
use super::catalog::Known;
use std::path::{Path, PathBuf};

/// Where installers put programs, besides PATH (npm's global folder is APPDATA's on Windows).
fn usual() -> Vec<PathBuf> {
    let home = crate::config::home_dir();
    let mut all: Vec<PathBuf> = [".local/bin", ".npm-global/bin", ".bun/bin", "bin"]
        .iter()
        .map(|d| home.join(d))
        .collect();
    all.extend(["/opt/homebrew/bin", "/usr/local/bin"].map(PathBuf::from));
    all.extend(shim_dirs(&home));
    if cfg!(windows) {
        // Where winget puts Git and Node: the PATH of this process does not see them until a new terminal.
        let files = ["ProgramFiles", "ProgramW6432"].map(std::env::var_os);
        let kept = files.into_iter().flatten().map(PathBuf::from);
        all.extend(kept.flat_map(|d| [d.join("Git/cmd"), d.join("nodejs")]));
    }
    all.extend(std::env::var_os("APPDATA").map(|d| PathBuf::from(d).join("npm")));
    // Muse's and Codex's installers put them here and tell only new terminals (the user's PATH in the registry).
    let programs = std::env::var_os("LOCALAPPDATA").map(|d| PathBuf::from(d).join("Programs"));
    all.extend(
        programs
            .into_iter()
            .flat_map(|p| [p.join("muse"), p.join("OpenAI/Codex/bin")]),
    );
    all
}

/// Where `npm install -g --prefix ~/.local` puts its programs: `bin` on Unix, the prefix itself on Windows.
pub(super) fn shim_dirs(home: &Path) -> Vec<PathBuf> {
    if cfg!(windows) {
        vec![home.join(".local")]
    } else {
        vec![]
    }
}

/// `path` when it runs.
#[cfg(not(windows))]
fn named(path: &Path) -> Option<PathBuf> {
    crate::os::runs(path).then(|| path.to_path_buf())
}

/// `path` when it runs, else with one of PATHEXT's extensions after it (`codex` is `codex.cmd`).
#[cfg(windows)]
fn named(path: &Path) -> Option<PathBuf> {
    let with = |ext: String| {
        let mut name = path.as_os_str().to_owned();
        name.push(format!(".{ext}"));
        PathBuf::from(name)
    };
    let mut all =
        std::iter::once(path.to_path_buf()).chain(crate::os::exts().into_iter().map(with));
    all.find(|p| crate::os::runs(p))
}

/// The program called `name` (a path, or a name looked up), as an absolute path.
pub fn program(name: &str) -> Option<PathBuf> {
    program_if(name, |_| true)
}

/// The first program called `name` that `keep` takes: a later one when an earlier one is not.
fn program_if(name: &str, keep: impl Fn(&Path) -> bool) -> Option<PathBuf> {
    if name.is_empty() {
        return None;
    }
    if name.contains('/') || (cfg!(windows) && name.contains('\\')) {
        let path = crate::config::expand(name);
        let found = named(&path).map(|p| std::path::absolute(&p).unwrap_or(p));
        return found.filter(|p| keep(p));
    }
    let path = std::env::var("PATH").unwrap_or_default();
    std::env::split_paths(&path)
        .filter(|d| d.is_absolute())
        .chain(usual())
        .filter_map(|d| named(&d.join(name)))
        .find(|p| keep(p))
}

/// An agent's program: config.toml's command when it runs, else the catalog's name. Never a stand-in that only says the agent is missing (the real one after it on PATH is found).
pub fn agent(known: &Known, configured: Option<&str>) -> Option<PathBuf> {
    let real = |p: &Path| !stand_in(p);
    let set = configured.and_then(|c| program_if(c, real));
    set.or_else(|| program_if(&known.bin, real))
}

/// VS Code's `copilot-shim\copilot.exe`, on PATH for every terminal it opens: it exits 127 with "GitHub Copilot CLI was not found".
pub(super) fn stand_in(path: &Path) -> bool {
    path.components().any(|c| c.as_os_str() == "copilot-shim")
}
