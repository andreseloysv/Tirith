//! Which program a name on the command line is, and Codex's own binary in it: the profile's rules name that one file.
use std::io::Read;
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

/// The binary itself: npm's `codex` is a script beside the one it starts.
pub(super) fn native(found: &Path) -> Option<PathBuf> {
    let real = found.canonicalize().ok()?;
    if mach_o(&real) {
        return Some(real);
    }
    let package = real.parent()?.parent()?;
    let (arch, short) = if cfg!(target_arch = "aarch64") {
        ("aarch64", "arm64")
    } else {
        ("x86_64", "x64")
    };
    let mut vendors = vec![package
        .join(format!("node_modules/@openai/codex-darwin-{short}"))
        .join("vendor")];
    if let Some(beside) = package.parent() {
        vendors.push(beside.join(format!("codex-darwin-{short}/vendor")));
    }
    vendors.push(package.join("vendor"));
    vendors
        .iter()
        .flat_map(|vendor| {
            let built = vendor.join(format!("{arch}-apple-darwin"));
            ["bin/codex", "codex/codex"].map(|file| built.join(file))
        })
        .find_map(|file| file.canonicalize().ok().filter(|real| mach_o(real)))
}

fn mach_o(file: &Path) -> bool {
    let mut magic = [0u8; 4];
    let read = std::fs::File::open(file).and_then(|mut f| f.read_exact(&mut magic));
    read.is_ok() && matches!(magic, [0xcf, 0xfa, 0xed, 0xfe] | [0xca, 0xfe, 0xba, 0xbe])
}
