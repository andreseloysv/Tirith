//! The owner's shell environment, taken once instead of at every agent session: what their login shell exports
//! after its login profile ran (a slow thing: one nvm line costs seconds), kept in NeoBot's home as a file of
//! `export` lines that a shell reads in no time. What the profile puts in front of PATH goes in front of whatever
//! PATH the reader has, as it does when the profile itself runs. Functions and aliases are not carried.
//! `login_run.rs` takes it.
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub(super) const MARK: &str = "neobot-login-env";
/// The files a login shell reads: one changed after the capture makes it old.
const PROFILES: [&str; 8] = [
    ".zshenv",
    ".zprofile",
    ".zshrc",
    ".zlogin",
    ".profile",
    ".bash_profile",
    ".bash_login",
    ".bashrc",
];
/// The reading shell's own, never the profile's to hand over.
const OWN: [&str; 7] = ["PWD", "OLDPWD", "SHLVL", "_", "TERM", "COLUMNS", "LINES"];
pub(super) type Vars = BTreeMap<String, String>;

pub fn file(home: &Path) -> PathBuf {
    home.join("shell-env.sh")
}

/// The variables in what the login shell printed after the mark: `NAME=value` pieces between NULs.
pub fn parse(out: &[u8]) -> Option<Vars> {
    let text = String::from_utf8_lossy(out);
    let (_, vars) = text.split_once(&format!("{MARK}\0"))?;
    let pairs = vars.split('\0').filter_map(|kv| kv.split_once('='));
    Some(pairs.map(|(k, v)| (k.into(), v.into())).collect())
}

/// The export lines for what `shell` has and `base` (this process) has not, or has otherwise.
pub fn lines(shell: &Vars, base: &Vars) -> String {
    let named = |k: &str| k.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
    let first = |k: &str| k.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_');
    let quote = |v: &str| format!("'{}'", v.replace('\'', r"'\''"));
    let mut out = String::new();
    for (k, v) in shell {
        if !named(k) || !first(k) || OWN.contains(&k.as_str()) || base.get(k) == Some(v) {
            continue;
        }
        if k != "PATH" {
            out += &format!("export {k}={}\n", quote(v));
            continue;
        }
        let had: Vec<&str> = base.get(k).map_or(vec![], |p| p.split(':').collect());
        let front: Vec<&str> = v
            .split(':')
            .filter(|d| !d.is_empty() && !had.contains(d))
            .collect();
        if !front.is_empty() {
            out += &format!("export PATH={}:\"$PATH\"\n", quote(&front.join(":")));
        }
    }
    out
}

/// Whether the capture is there and no profile file changed after it.
pub fn current(home: &Path) -> bool {
    let at = |p: PathBuf| std::fs::metadata(p).and_then(|m| m.modified()).ok();
    let user = crate::config::home_dir();
    at(file(home)).is_some_and(|taken| {
        let files = PROFILES.iter().filter_map(|f| at(user.join(f)));
        files.into_iter().all(|changed| changed <= taken)
    })
}
