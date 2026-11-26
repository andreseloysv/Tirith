//! macOS: the rules that keep the files proving who calls (`app/keys.rs`) from every confined process.
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// A path as a string of a profile.
fn quoted(path: &Path) -> String {
    let text = path.display().to_string();
    let text = text.replace('\\', "\\\\").replace('"', "\\\"");
    format!("\"{}\"", text.replace('\n', "\\n"))
}

/// The profile's lines for `files`: neither read, written, linked nor cloned, as named and as the kernel names them. A folder above one keeps its name, or the rule would stay behind.
pub(super) fn rules(files: &[PathBuf]) -> String {
    let (mut lines, mut above) = (String::new(), BTreeSet::new());
    for file in files {
        let real = file
            .parent()
            .and_then(|dir| dir.canonicalize().ok())
            .zip(file.file_name())
            .map(|(dir, name)| dir.join(name));
        for path in BTreeSet::from([file.clone(), real.unwrap_or_else(|| file.clone())]) {
            lines += &format!(
                "(deny file-read* file-write* file-link file-clone (literal {}))\n",
                quoted(&path)
            );
            let dirs = path.ancestors().skip(1).filter(|d| d.parent().is_some());
            above.extend(dirs.map(Path::to_path_buf));
        }
    }
    for dir in above {
        lines += &format!("(deny file-write-unlink (literal {}))\n", quoted(&dir));
    }
    lines
}

/// The profile's line for a folder and all under it, as named and as the kernel names it: files made in it later are covered too (`auth/`, where an agent's provider key is kept).
pub(super) fn folder(dir: &Path) -> String {
    let real = dir
        .parent()
        .and_then(|up| up.canonicalize().ok())
        .zip(dir.file_name())
        .map(|(up, name)| up.join(name));
    BTreeSet::from([dir.to_path_buf(), real.unwrap_or_else(|| dir.to_path_buf())])
        .iter()
        .map(|p| {
            format!(
                "(deny file-read* file-write* file-link file-clone (subpath {}))\n",
                quoted(p)
            )
        })
        .collect()
}
