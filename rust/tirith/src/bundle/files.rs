//! The files built into the binary (build.rs): the page, the teach extension, the browser launcher.

include!(concat!(env!("OUT_DIR"), "/carried.rs"));

/// The carried file at `path`, as the repository names it (`web/index.html`).
pub fn file(path: &str) -> Option<&'static [u8]> {
    FILES.iter().find(|(p, _)| *p == path).map(|(_, b)| *b)
}

/// The carried files below `dir`, named from it, in order.
pub fn below(dir: &str) -> Vec<(&'static str, &'static [u8])> {
    let prefix = format!("{dir}/");
    FILES
        .iter()
        .filter_map(|(p, b)| p.strip_prefix(prefix.as_str()).map(|rest| (rest, *b)))
        .collect()
}
