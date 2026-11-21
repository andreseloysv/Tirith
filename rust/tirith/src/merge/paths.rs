//! Which stages a change asks for: `paths` globs, and deploy stages that publish main instead of checking a lane.
use crate::config::Stage;
use regex::Regex;
/// A stage with `paths` runs only when a changed file matches them (`!pattern` excludes).
pub(super) fn wanted(stage: &Stage, changed: &[String]) -> bool {
    let Some(()) = (!stage.paths.is_empty()).then_some(()) else {
        return true;
    };
    let (neg, pos): (Vec<&String>, Vec<&String>) =
        stage.paths.iter().partition(|p| p.starts_with('!'));
    let neg: Vec<Regex> = neg.iter().map(|p| glob(&p[1..])).collect();
    let pos: Vec<Regex> = pos.iter().map(|p| glob(p)).collect();
    changed.iter().any(|f| {
        (pos.is_empty() || pos.iter().any(|r| r.is_match(f))) && !neg.iter().any(|r| r.is_match(f))
    })
}
/// A glob as a regex: `**/` any folders, `**` anything, `*` within a folder, `?` one char.
fn glob(pattern: &str) -> Regex {
    Regex::new(&format!(
        "^{}$",
        regex::escape(pattern)
            .replace(r"\*\*/", "(.*/)?")
            .replace(r"\*\*", ".*")
            .replace(r"\*", "[^/]*")
            .replace(r"\?", "[^/]")
    ))
    .unwrap_or_else(|_| Regex::new("^$").unwrap())
}
/// A deploy stage (its `kind`, else a name that says so) is no check of a lane: it publishes main.
pub(super) fn deploys(s: &Stage) -> bool {
    let kind = s.kind.trim().to_ascii_lowercase();
    kind == "deploy"
        || (kind.is_empty() && ["deploy", "release", "publish", "ship"].contains(&s.name.as_str()))
}
