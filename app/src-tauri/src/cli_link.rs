//! `tirith` in the owner's terminal: `~/.local/bin/tirith` links to the binary the bundle carries.

use std::path::Path;

/// Makes the link when there is no `tirith` there. What is there is left alone, except a link into an
/// app bundle that is gone (the app was moved).
pub fn link(bundled: &Path, home: &Path) -> std::io::Result<()> {
    if !bundled.is_file() || passing(bundled) {
        return Ok(());
    }
    let link = home.join(".local/bin/tirith");
    if link.symlink_metadata().is_ok() {
        let into_bundle = std::fs::read_link(&link).is_ok_and(|to| to.ends_with("Contents/MacOS/tirith"));
        if link.exists() || !into_bundle {
            return Ok(());
        }
        std::fs::remove_file(&link)?;
    }
    std::fs::create_dir_all(home.join(".local/bin"))?;
    std::os::unix::fs::symlink(bundled, &link)
}

/// A path that will be gone later: a mounted dmg, macOS's App Translocation or an AppImage's mount.
fn passing(path: &Path) -> bool {
    path.starts_with("/Volumes")
        || path.iter().any(|part| part == "AppTranslocation" || part.to_string_lossy().starts_with(".mount_"))
}
