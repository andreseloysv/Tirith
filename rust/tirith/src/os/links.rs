//! Programs and links: what runs, and links to folders and files. Windows lets a user make a symlink only in Developer Mode, so there a folder gets a junction and a file a hard link when a symlink cannot be made.
use std::path::Path;

/// Whether `path` is a program that runs: a file with an execute bit on Unix, a file whose extension is one of PATHEXT's on Windows.
#[cfg(unix)]
pub fn runs(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(path).is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
}

#[cfg(windows)]
pub fn runs(path: &Path) -> bool {
    let ext = path.extension().map(|e| e.to_string_lossy().to_lowercase());
    std::fs::metadata(path).is_ok_and(|m| m.is_file()) && ext.is_some_and(|e| exts().contains(&e))
}

/// Windows's extensions of programs, from PATHEXT, lower case and without their dots, in its order.
#[cfg(windows)]
pub fn exts() -> Vec<String> {
    let set = std::env::var("PATHEXT").unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD".into());
    set.split(';')
        .filter_map(|e| e.strip_prefix('.'))
        .map(str::to_lowercase)
        .collect()
}

/// A link at `link` to the folder `target`, which may be relative to the link's folder.
#[cfg(unix)]
pub fn link_dir(target: &Path, link: &Path) -> std::io::Result<()> {
    std::os::unix::fs::symlink(target, link)
}

#[cfg(windows)]
pub fn link_dir(target: &Path, link: &Path) -> std::io::Result<()> {
    let full = beside(target, link);
    std::os::windows::fs::symlink_dir(&full, link).or_else(|_| junction(&full, link))
}

/// A link at `link` to `target`, a file or a folder.
#[cfg(unix)]
pub fn link_any(target: &Path, link: &Path) -> std::io::Result<()> {
    std::os::unix::fs::symlink(target, link)
}

#[cfg(windows)]
pub fn link_any(target: &Path, link: &Path) -> std::io::Result<()> {
    let full = beside(target, link);
    if full.is_dir() {
        return link_dir(&full, link);
    }
    std::os::windows::fs::symlink_file(&full, link).or_else(|_| std::fs::hard_link(&full, link))
}

/// Removes a link, to a file or to a folder (Windows removes a folder's link as a folder, never what it points to).
pub fn unlink(path: &Path) -> std::io::Result<()> {
    std::fs::remove_file(path).or_else(|e| match cfg!(windows) {
        true => std::fs::remove_dir(path),
        false => Err(e),
    })
}

/// `target` as seen from the link's folder.
#[cfg(windows)]
fn beside(target: &Path, link: &Path) -> std::path::PathBuf {
    link.parent()
        .map(|dir| dir.join(target))
        .unwrap_or_else(|| target.to_path_buf())
}

/// A junction: a link to a folder that needs no privilege (`mklink /J`).
#[cfg(windows)]
fn junction(target: &Path, link: &Path) -> std::io::Result<()> {
    let made = std::process::Command::new("cmd")
        .args(["/C", "mklink", "/J"])
        .arg(link)
        .arg(target)
        .stdout(std::process::Stdio::null())
        .status()?;
    made.success()
        .then_some(())
        .ok_or_else(|| std::io::Error::other(format!("mklink /J {}", link.display())))
}
