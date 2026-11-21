//! The environment an item's check borrows from the pipeline stage it belongs to.
use crate::config::Stage;
/// An item's check runs with the `VAR=value` words (JAVA_HOME, ANDROID_HOME) of the pipeline stage it belongs to: the stage named in it, or one running the same program.
pub(super) fn stage_env(stages: &[Stage], check: &str) -> Vec<(String, String)> {
    let said: Vec<String> = check
        .split(|c: char| !c.is_alphanumeric() && c != '_')
        .map(str::to_lowercase)
        .collect();
    let same = |s: &Stage| {
        said.contains(&s.name.to_lowercase())
            || s.run.split_whitespace().any(|w| {
                w.len() > 4
                    && assign(w).is_none()
                    && said.contains(&w.rsplit('/').next().unwrap_or("").to_lowercase())
            })
    };
    stages
        .iter()
        .filter(|s| same(s))
        .flat_map(|s| s.run.split_whitespace().filter_map(assign))
        .collect()
}
/// A `VAR=value` word (an upper-case name), its value without quotes.
fn assign(w: &str) -> Option<(String, String)> {
    w.split_once('=')
        .filter(|(k, _)| {
            !k.is_empty()
                && k.chars()
                    .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
        })
        .map(|(k, v)| (k.to_string(), v.trim_matches(['"', '\'']).to_string()))
}
