//! Which program a name on the command line is.
use std::path::{Path, PathBuf};

/// The program by its whole path.
pub(super) fn whole(program: &str, cwd: &Path) -> Option<PathBuf> {
    if program.contains('/') {
        return Some(cwd.join(program));
    }
    let own = crate::config::tirith_home().join("net/bin");
    std::env::split_paths(&std::env::var_os("PATH")?)
        .filter(|dir| *dir != own)
        .map(|dir| cwd.join(dir).join(program))
        .find(|file| file.is_file())
}
