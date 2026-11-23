//! A bot's memory citations (`[m<N>]`, `[s<N>]`) stay off the phones and the rooms: they are for OpenBot's count of the notes that serve (`turns::cited`), noise for the owner. The chat and the turns keep them.
use regex::Regex;
use std::sync::LazyLock;
/// One mark (`[m3]`, `[s12]`) with the spaces before it, or a run of them at the start of a line with the spaces after it (its indent stays).
static MARK: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)^([ \t]*)(?:\[[ms][0-9]+\][ \t]*)+|[ \t]*\[[ms][0-9]+\]").unwrap()
});
/// `text` without its citation marks, with no double space and no space before punctuation left behind: «Hecho [m1].» is «Hecho.».
pub fn uncited(text: &str) -> String {
    MARK.replace_all(text, "${1}").into_owned()
}
