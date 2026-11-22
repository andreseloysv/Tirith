//! A factory version's stamps: its name (commit plus overlay hash) and the newest change time of its files.
use super::lists::SECTIONS;
use super::paths::{dir, text_files};
use std::path::Path;
use std::time::SystemTime;

/// An 8-hex-char hash of a text: short enough for a stamp, different whenever the text is.
pub(super) fn short_hash(text: &str) -> String {
    crate::rooms::sealed::sha_hex(text.as_bytes())[..8].to_string()
}

/// The main commit that last changed the factory folder (its short hash), or `embedded` when no checkout is at hand.
fn commit_of(overlay: &toml::Table) -> String {
    let show = |rev: &str| -> Option<String> {
        let out = crate::sandbox::git() // the CLI runs it unconfined, in a checkout lanes write
            .env("GIT_NO_LAZY_FETCH", "1")
            .args(["-c", "core.fsmonitor=false", "-C"])
            .arg(dir(overlay)?)
            .args([
                "log",
                "--no-show-signature",
                "-1",
                "--format=%h",
                rev,
                "--",
                ".",
            ])
            .output()
            .ok()?;
        let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
        (out.status.success() && !text.is_empty()).then_some(text)
    };
    show("main")
        .or_else(|| show("HEAD"))
        .unwrap_or_else(|| "embedded".into())
}

/// `<commit>+<hash>`: the factory's commit and a short hash of the sections config.toml lays over it.
pub(super) fn version(overlay: &toml::Table) -> String {
    let over: toml::Table = SECTIONS
        .into_iter()
        .filter_map(|s| overlay.get(s).map(|v| (s.to_string(), v.clone())))
        .collect();
    format!(
        "{}+{}",
        commit_of(overlay),
        short_hash(&serde_json::to_string(&over).unwrap_or_default())
    )
}

/// The newest change time of the factory's files, and of main itself: when it moves, the factory reloads with no restart.
pub fn stamp(overlay: &toml::Table) -> Option<SystemTime> {
    let dir = dir(overlay)?;
    let m = |p: &Path| std::fs::metadata(p).and_then(|x| x.modified()).ok();
    let files = [
        dir.join("factory.toml"),
        dir.join("../.git/logs/HEAD"),
        dir.join("../.git/refs/heads/main"),
    ];
    text_files(&dir)
        .filter_map(|e| m(&e.path()))
        .chain(files.iter().filter_map(|p| m(p)))
        .max()
}
