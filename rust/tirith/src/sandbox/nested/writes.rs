//! What may be written: the folders workspace-write opens, less what it protects, but not by Codex's own process, whose writing is its state's alone (and only what it must). A folder above what stays cannot be renamed, or the rule would stay behind.
use super::limits::Around;
use super::sb::{named, real, Sb};
use super::{state, Request};
use std::collections::BTreeSet;
use std::path::Path;

/// What workspace-write itself protects in each of its folders.
const PROTECTED: [&str; 3] = [".git", ".codex", ".agents"];
const DEVICES: [&str; 4] = [
    "(allow file-write* (literal \"/dev/null\") (literal \"/dev/zero\") (literal \"/dev/dtracehelper\"))",
    "(allow file-write* (subpath \"/dev/fd\"))",
    "(allow file-write* (literal \"/dev/ptmx\"))",
    "(allow file-write* (require-all (regex #\"^/dev/ttys[0-9]+\") (extension \"com.apple.sandbox.pty\")))",
];

pub(super) fn writes(sb: &mut Sb, r: &Request, around: &Around, own: Option<&str>) {
    sb.line("(deny file-write*)");
    DEVICES.iter().for_each(|rule| sb.line(rule));
    let mut held = BTreeSet::new();
    let roots: &[String] = if r.write { &r.roots } else { &[] };
    for root in roots.iter().filter(|p| !p.is_empty()) {
        let root = format!("(subpath {})", sb.path(&real(root)));
        let who = match own {
            Some(own) => format!("(require-all {root} (require-not {own}))"),
            None => root,
        };
        sb.line(&format!("(allow file-write* {who})"));
    }
    for root in roots.iter().filter(|p| !p.is_empty()) {
        for name in PROTECTED {
            stays(sb, &format!("{}/{name}", real(root)), &mut held);
        }
    }
    if !r.state.is_empty() {
        stays(sb, &r.state, &mut held);
        if let Some(own) = own {
            state::own(sb, &real(&r.state), own);
        }
    }
    if let Some(own) = own.filter(|_| !r.out.is_empty()) {
        let out = sb.path(&real(&r.out)); // Codex writes the file `-o` names itself
        sb.line(&format!(
            "(allow file-write* (require-all {own} (literal {out})))"
        ));
    }
    for path in around.kept.iter().chain(Some(&r.program)) {
        stays(sb, path, &mut held);
    }
    for dir in held {
        let dir = sb.path(&dir);
        sb.line(&format!("(deny file-write-unlink (literal {dir}))"));
    }
}

/// `path` is not written, as it is named and as it resolves; what is above it stays where it is.
fn stays(sb: &mut Sb, path: &str, held: &mut BTreeSet<String>) {
    if path.is_empty() {
        return;
    }
    for path in BTreeSet::from([named(path), real(path)]) {
        let above = Path::new(&path).ancestors().skip(1);
        held.extend(above.map(|dir| dir.display().to_string()));
        let name = sb.path(&path);
        sb.line(&format!("(deny file-write* (subpath {name}))"));
    }
}
