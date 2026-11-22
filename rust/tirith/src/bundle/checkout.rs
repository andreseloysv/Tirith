//! The checkout this binary was built from, for a development build that still has it.
use std::path::PathBuf;

/// `path` in that checkout, when it exists. A build to hand out (`TIRITH_DIST`) never has one.
#[cfg(not(tirith_dist))]
pub fn checkout(path: &str) -> Option<PathBuf> {
    Some(PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../..")).join(path))
        .filter(|p| p.exists())
}

#[cfg(tirith_dist)]
pub fn checkout(_path: &str) -> Option<PathBuf> {
    None
}
