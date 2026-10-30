//! What the app needs to work on a clean Mac: find `uv`, and keep a writable copy of the bundled collector
//! (uv creates its virtualenv next to `pyproject.toml`, which must not be inside the signed `.app`).

use std::ffi::OsStr;
use std::io;
use std::path::{Path, PathBuf};

/// Written in the copied collector; the copy is refreshed when it differs from the bundle's stamp.
const STAMP_FILE: &str = ".tirith-bundle-stamp";

/// The first `uv` found in `path`, then /opt/homebrew/bin, /usr/local/bin, ~/.local/bin, ~/.cargo/bin
/// (apps opened from Finder only get `/usr/bin:/bin:/usr/sbin:/sbin`).
pub fn find_uv(path: Option<&OsStr>, home: &Path) -> Option<PathBuf> {
    let mut dirs: Vec<PathBuf> = path.map(|p| std::env::split_paths(p).collect()).unwrap_or_default();
    dirs.push("/opt/homebrew/bin".into());
    dirs.push("/usr/local/bin".into());
    dirs.push(home.join(".local/bin"));
    dirs.push(home.join(".cargo/bin"));
    dirs.into_iter().map(|d| d.join("uv")).find(|c| c.is_file())
}

/// Where the collector runs from. In order: `$TIRITH_COLLECTOR_DIR`, `[app] collector_dir` (developer mode),
/// the writable copy of the bundled collector, the source tree's `collector/` (cargo runs).
pub fn pick_collector(env: Option<PathBuf>, setting: Option<PathBuf>, bundled_copy: Option<PathBuf>, source_tree: PathBuf) -> PathBuf {
    env.or(setting).or(bundled_copy).unwrap_or(source_tree)
}

/// A stamp of the folder's contents (names and bytes), so a changed bundle is noticed even at the same version.
pub fn stamp_of(dir: &Path, version: &str) -> io::Result<String> {
    fn walk(dir: &Path, h: &mut u64) -> io::Result<()> {
        let mut entries: Vec<_> = std::fs::read_dir(dir)?.flatten().collect();
        entries.sort_by_key(|e| e.file_name());
        for e in entries {
            let name = e.file_name();
            for b in name.to_string_lossy().bytes() {
                *h = (*h ^ b as u64).wrapping_mul(0x100000001b3);
            }
            let p = e.path();
            if p.is_dir() {
                walk(&p, h)?;
            } else {
                for b in std::fs::read(&p)? {
                    *h = (*h ^ b as u64).wrapping_mul(0x100000001b3);
                }
            }
        }
        Ok(())
    }
    let mut h = 0xcbf29ce484222325u64;
    walk(dir, &mut h)?;
    Ok(format!("{version}:{h:016x}"))
}

fn copy_tree(src: &Path, dst: &Path) -> io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for e in std::fs::read_dir(src)?.flatten() {
        let to = dst.join(e.file_name());
        if e.path().is_dir() {
            copy_tree(&e.path(), &to)?;
        } else {
            std::fs::copy(e.path(), &to)?;
        }
    }
    Ok(())
}

/// Makes `dest` a copy of `bundled` unless it already carries the same stamp. The virtualenv (`.venv`) in
/// `dest` is kept, so uv does not rebuild it for every release. Returns whether a copy was made.
pub fn sync_collector(bundled: &Path, dest: &Path, version: &str) -> io::Result<bool> {
    let stamp = stamp_of(bundled, version)?;
    if std::fs::read_to_string(dest.join(STAMP_FILE)).ok().as_deref() == Some(stamp.as_str()) {
        return Ok(false);
    }
    if dest.is_dir() {
        for e in std::fs::read_dir(dest)?.flatten() {
            if e.file_name() == ".venv" {
                continue;
            }
            if e.path().is_dir() {
                std::fs::remove_dir_all(e.path())?;
            } else {
                std::fs::remove_file(e.path())?;
            }
        }
    }
    copy_tree(bundled, dest)?;
    std::fs::write(dest.join(STAMP_FILE), stamp)?;
    Ok(true)
}

/// The Terminal command that runs the official uv installer.
pub const UV_INSTALL_COMMAND: &str = "curl -LsSf https://astral.sh/uv/install.sh | sh";
/// The official install page, for people who prefer to read first.
pub const UV_INSTALL_PAGE: &str = "https://docs.astral.sh/uv/getting-started/installation/";

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("tirith-standalone-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn dev_settings_win_over_the_bundle_and_the_bundle_over_the_source_tree() {
        let p = |s: &str| Some(PathBuf::from(s));
        let src = PathBuf::from("/src/collector");
        assert_eq!(pick_collector(p("/env"), p("/cfg"), p("/support"), src.clone()), PathBuf::from("/env"));
        assert_eq!(pick_collector(None, p("/cfg"), p("/support"), src.clone()), PathBuf::from("/cfg"));
        assert_eq!(pick_collector(None, None, p("/support"), src.clone()), PathBuf::from("/support"));
        assert_eq!(pick_collector(None, None, None, src.clone()), src);
    }

    #[test]
    fn uv_is_found_in_path_then_the_usual_folders() {
        let home = scratch("uv-home");
        let bin = scratch("uv-bin");
        std::fs::create_dir_all(home.join(".local/bin")).unwrap();
        std::fs::write(home.join(".local/bin/uv"), "").unwrap();
        let got = find_uv(Some(OsStr::new("/nonexistent")), &home).unwrap();
        assert!(got == home.join(".local/bin/uv") || got.starts_with("/opt/homebrew") || got.starts_with("/usr/local"));
        std::fs::write(bin.join("uv"), "").unwrap();
        assert_eq!(find_uv(Some(bin.as_os_str()), &home).unwrap(), bin.join("uv"));
    }

    #[test]
    fn the_collector_copy_is_made_once_refreshed_on_change_and_keeps_its_venv() {
        let root = scratch("sync");
        let bundled = root.join("bundled");
        let dest = root.join("support/collector");
        std::fs::create_dir_all(bundled.join("tirith")).unwrap();
        std::fs::write(bundled.join("pyproject.toml"), "a").unwrap();
        std::fs::write(bundled.join("tirith/cli.py"), "v1").unwrap();
        assert!(sync_collector(&bundled, &dest, "0.1.0").unwrap());
        assert_eq!(std::fs::read_to_string(dest.join("tirith/cli.py")).unwrap(), "v1");
        assert!(!sync_collector(&bundled, &dest, "0.1.0").unwrap(), "same bundle: no copy");

        std::fs::create_dir_all(dest.join(".venv")).unwrap();
        std::fs::write(dest.join(".venv/keep"), "").unwrap();
        std::fs::write(dest.join("stale.py"), "").unwrap();
        std::fs::write(bundled.join("tirith/cli.py"), "v2").unwrap();
        assert!(sync_collector(&bundled, &dest, "0.1.0").unwrap(), "changed bundle: copy again");
        assert_eq!(std::fs::read_to_string(dest.join("tirith/cli.py")).unwrap(), "v2");
        assert!(dest.join(".venv/keep").exists());
        assert!(!dest.join("stale.py").exists());
        assert!(sync_collector(&bundled, &dest, "0.2.0").unwrap(), "new app version: copy again");
        let _ = std::fs::remove_dir_all(&root);
    }
}
