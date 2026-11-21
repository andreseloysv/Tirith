//! The project as it will be written: its name, its colour and its id.
use super::colors::COLORS;
use crate::config::Project;
use std::path::Path;

/// The project as it will be written: `name` (else the folder's), `color` (else the first one no project has), an id.
pub(super) fn project(
    dir: &Path,
    name: &str,
    color: &str,
    projects: &[Project],
) -> Result<Project, String> {
    let name = name_of(dir, name)?;
    let color = color_of(color, projects)?;
    Ok(Project {
        id: unique_id(&name, projects),
        name,
        repo: dir.display().to_string(),
        color: Some(color),
        chat_keep: String::new(),
    })
}

/// `name`, else the folder's, when it is one line of 1 to 80 characters.
fn name_of(dir: &Path, name: &str) -> Result<String, String> {
    let name = Some(name.trim().to_string())
        .filter(|n| !n.is_empty())
        .or_else(|| Some(dir.file_name()?.to_string_lossy().into_owned()))
        .unwrap_or_default();
    (!name.is_empty() && name.chars().count() <= 80 && !name.chars().any(char::is_control))
        .then_some(name)
        .ok_or("the name is one line of 1 to 80 characters".to_string())
}

/// `color` in lowercase when it is `#rrggbb`, the first free one when it is empty.
fn color_of(color: &str, projects: &[Project]) -> Result<String, String> {
    let c = color.trim().to_lowercase();
    match c.as_str() {
        "" => Ok(free(projects).to_string()),
        _ if c.len() == 7
            && c.starts_with('#')
            && c[1..].chars().all(|h| h.is_ascii_hexdigit()) =>
        {
            Ok(c)
        }
        _ => Err(format!("the color must look like #2f7d5b, not {color:?}")),
    }
}

/// The first colour no project has, else one by the number of projects.
fn free(projects: &[Project]) -> &'static str {
    COLORS
        .iter()
        .find(|c| {
            !projects.iter().any(|p| {
                p.color
                    .as_deref()
                    .is_some_and(|x| x.eq_ignore_ascii_case(c))
            })
        })
        .unwrap_or(&COLORS[projects.len() % COLORS.len()])
}

/// `name` in lowercase letters, digits and dashes, with -2, -3… when that id is taken.
pub(super) fn unique_id(name: &str, projects: &[Project]) -> String {
    let base = Some(crate::lanes::slug(name))
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "project".into());
    (1..)
        .map(|n| {
            if n == 1 {
                base.clone()
            } else {
                format!("{base}-{n}")
            }
        })
        .find(|id| !projects.iter().any(|p| p.id == *id))
        .unwrap_or(base)
}
