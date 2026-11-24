//! The owner's home, NeoBot's home, and paths written with `~/`.
use std::path::PathBuf;

/// A path from a config file, with a leading `~/` read as the owner's home.
pub fn expand(p: &str) -> PathBuf {
    p.strip_prefix("~/")
        .map_or_else(|| PathBuf::from(p), |rest| home_dir().join(rest))
}

/// HOME; on Windows, where it is seldom set, USERPROFILE.
pub fn home_dir() -> PathBuf {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE").filter(|_| cfg!(windows)))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

/// `TIRITH_HOME`, else `~/.tirith`.
pub fn tirith_home() -> PathBuf {
    std::env::var_os("TIRITH_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home_dir().join(".tirith"))
}
