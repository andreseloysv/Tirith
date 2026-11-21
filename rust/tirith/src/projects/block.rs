//! The `[[projects]]` block appended to config.toml.
use crate::config::Project;

/// What to append to config.toml (`text`) for the project, checked to parse with the project in it.
pub(super) fn block(text: &str, p: &Project) -> Result<String, String> {
    let block = format!("{}{}", separator(text), table(p));
    let t: toml::Table = format!("{text}{block}")
        .parse()
        .map_err(|e| format!("config.toml would not parse with the project: {e}"))?;
    let added = crate::config::get::<Vec<Project>>(&t, &["projects"])
        .is_some_and(|ps| ps.iter().any(|x| x.id == p.id && x.repo == p.repo));
    added.then_some(block).ok_or("config.toml has a `projects` key that is not [[projects]] tables: add the project there by hand".into())
}

/// One blank line before the block, unless the file is empty.
fn separator(text: &str) -> &'static str {
    if text.trim().is_empty() {
        ""
    } else {
        &"\n\n"[usize::from(text.ends_with('\n'))..]
    }
}

/// The project's table, its strings quoted as TOML.
fn table(p: &Project) -> String {
    let q = |s: &str| toml::Value::String(s.into()).to_string();
    format!(
        "[[projects]]\nid = {}\nname = {}\nrepo = {}\ncolor = {}\n",
        q(&p.id),
        q(&p.name),
        q(&p.repo),
        q(p.color.as_deref().unwrap_or(""))
    )
}
