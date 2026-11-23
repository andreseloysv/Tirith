//! Whether a project's guard is installed, current and not skipped by git.
use super::{git_dir::git_dir, render::render, render::HOOKS, render::MARKER, words::words};
use crate::app::Shared;
use crate::config::{home_dir, Project};
use std::path::{Path, PathBuf};

/// Ok when the project's guard is installed and intact (or the project opts out); the reason otherwise. The scheduler calls it before it starts a lane of the project: a few small file reads, no git process.
pub fn intact(app: &Shared, project: &Project) -> Result<(), String> {
    let words = words(app, project);
    let Some(words) = (!words.is_empty()).then_some(words) else {
        return Ok(());
    };
    let git = git_dir(&project.repo()).ok_or(format!(
        "commit guard: {} is not a git repo",
        project.repo().display()
    ))?;
    let fix = format!("(tirith guard install {})", project.id);
    for hook in HOOKS {
        hook_intact(&git, hook, &words, &fix)?;
    }
    not_skipped(&git)
}

/// The hook is there, is OpenBot's, and has config.toml's words.
fn hook_intact(git: &Path, hook: &str, words: &[String], fix: &str) -> Result<(), String> {
    let text = std::fs::read_to_string(git.join("hooks").join(hook))
        .map_err(|_| format!("commit guard: the {hook} hook is missing {fix}"))?;
    (text.contains(MARKER)).then_some(()).ok_or_else(|| {
        format!("commit guard: the {hook} hook is not OpenBot's; merge the guard into it by hand")
    })?;
    (text == render(hook, words)).then_some(()).ok_or_else(|| {
        format!(
            "commit guard: the {hook} hook is outdated or has other words than config.toml {fix}"
        )
    })
}

/// A `core.hooksPath` in the repo's or the user's git config makes git skip the guard.
fn not_skipped(git: &Path) -> Result<(), String> {
    let skips = |f: &PathBuf| {
        std::fs::read_to_string(f).is_ok_and(|t| {
            t.lines().any(|l| {
                l.trim()
                    .to_lowercase()
                    .replace(' ', "")
                    .starts_with("hookspath=")
            })
        })
    };
    [git.join("config"), home_dir().join(".gitconfig")]
        .into_iter()
        .find(skips)
        .map_or(Ok(()), |f| {
            Err(format!(
                "commit guard: core.hooksPath in {} makes git skip the guard",
                f.display()
            ))
        })
}
