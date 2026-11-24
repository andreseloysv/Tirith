//! The command's two names: `neobot`, and `tirith` as its alias. Agents' prompts may still say `tirith ask`, so a folder on the PATH of
//! lanes and agents holds both, each a link to this binary (a hard link, or a copy, on Windows).
use std::path::Path;

/// The names the binary answers to.
const NAMES: [&str; 2] = ["neobot", "tirith"];

/// Makes `dir` hold both names, each pointing at the running binary and nothing else; a failure leaves the others working.
pub fn link_names(dir: &Path) {
    let Some(exe) = super::path::exe() else {
        return;
    };
    let _ = std::fs::create_dir_all(dir);
    let ext = exe.extension().map(|e| format!(".{}", e.to_string_lossy()));
    for name in NAMES {
        let link = dir.join(format!("{name}{}", ext.as_deref().unwrap_or("")));
        if link != exe {
            point(&exe, &link);
        }
    }
}

#[cfg(unix)]
fn point(exe: &Path, link: &Path) {
    if std::fs::read_link(link).is_ok_and(|t| t == exe) {
        return;
    }
    let _ = std::fs::remove_file(link);
    let _ = crate::os::link_any(exe, link);
}

#[cfg(windows)]
fn point(exe: &Path, link: &Path) {
    let same = |a: &Path, b: &Path| match (std::fs::metadata(a), std::fs::metadata(b)) {
        (Ok(a), Ok(b)) => a.len() == b.len() && a.modified().ok() == b.modified().ok(),
        _ => false,
    };
    if same(exe, link) {
        return;
    }
    let _ = std::fs::remove_file(link);
    let _ = std::fs::hard_link(exe, link).or_else(|_| std::fs::copy(exe, link).map(drop));
}
