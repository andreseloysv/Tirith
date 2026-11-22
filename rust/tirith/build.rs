//! What the binary carries so that it runs with no checkout beside it: the page, the teach extension and the
//! browser launcher, plus its version and commit. With `TIRITH_DIST` set it is a build to hand out: it also
//! never looks for the checkout it was built from (and does not hold that folder's path).
use std::path::{Path, PathBuf};
use std::process::Command;

const CARRIED: [&str; 3] = ["web", "tools/teach", "tools/browser"];

fn main() {
    let root = PathBuf::from(var("CARGO_MANIFEST_DIR")).join("../..");
    let mut files = vec![];
    for dir in CARRIED {
        walk(&root.join(dir), &mut files);
        println!("cargo::rerun-if-changed={}", root.join(dir).display());
    }
    files.sort();
    let out = PathBuf::from(var("OUT_DIR"));
    let rows: String = files
        .iter()
        .enumerate()
        .map(|(i, f)| row(&root, &out, i, f))
        .collect();
    let code = format!("pub(super) const FILES: &[(&str, &[u8])] = &[\n{rows}];\n");
    std::fs::write(out.join("carried.rs"), code).expect("write carried.rs");
    version(&root);
    println!("cargo::rustc-check-cfg=cfg(tirith_dist)");
    println!("cargo::rerun-if-env-changed=TIRITH_DIST");
    if std::env::var("TIRITH_DIST").is_ok_and(|v| !v.is_empty()) {
        println!("cargo::rustc-cfg=tirith_dist");
    }
}

fn var(key: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| panic!("{key} is not set"))
}

/// Every file below `dir`, but hidden ones.
fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        if e.file_name().to_string_lossy().starts_with('.') {
            continue;
        }
        if e.path().is_dir() {
            walk(&e.path(), out);
        } else {
            out.push(e.path());
        }
    }
}

/// One file, copied into OUT_DIR so the binary names no folder of the machine that built it.
fn row(root: &Path, out: &Path, i: usize, file: &Path) -> String {
    let name = file.strip_prefix(root).expect("a file of the repository");
    let copy = format!("carried-{i}");
    std::fs::copy(file, out.join(&copy)).expect("copy a carried file");
    format!(
        "    ({:?}, include_bytes!(concat!(env!(\"OUT_DIR\"), \"/{copy}\")).as_slice()),\n",
        name.to_string_lossy()
    )
}

/// `TIRITH_VERSION`: the release's (`TIRITH_RELEASE`) or the crate's, then the commit when git knows it.
fn version(root: &Path) {
    println!("cargo::rerun-if-env-changed=TIRITH_RELEASE");
    let release = std::env::var("TIRITH_RELEASE").unwrap_or_else(|_| var("CARGO_PKG_VERSION"));
    let commit = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["rev-parse", "--short=8", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| format!(" ({})", String::from_utf8_lossy(&o.stdout).trim()))
        .unwrap_or_default();
    let target = var("TARGET");
    println!("cargo::rustc-env=TIRITH_VERSION={release}{commit} {target}");
}
