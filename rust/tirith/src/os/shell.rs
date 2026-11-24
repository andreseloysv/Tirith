//! The POSIX shell that stages, checks and hooks run in.

/// `sh` on Unix. On Windows the one Git for Windows carries (NeoBot needs Git there anyway): on PATH, else in Git's folder above `git.exe` (`cmd\git.exe` or `mingw64\bin\git.exe` beside `bin\sh.exe`).
#[cfg(unix)]
pub fn sh() -> String {
    "sh".into()
}

#[cfg(windows)]
pub fn sh() -> String {
    use crate::setup::find::program;
    let beside_git = || {
        let git = program("git")?;
        let mut roots = git.ancestors().skip(1).take(3);
        roots.find_map(|root| {
            ["bin/sh.exe", "usr/bin/sh.exe"]
                .map(|p| root.join(p))
                .into_iter()
                .find(|p| p.is_file())
        })
    };
    program("sh")
        .or_else(beside_git)
        .map_or_else(|| "sh".into(), |p| p.display().to_string())
}
