//! Draft skills (a taught task the owner has not approved): kept in `drafts/` of the bot's folder (or `<home>/bots/drafts` for a shared one), outside every folder an agent loads by itself; one found in a skills folder is moved there.
use super::folder::{folder, SKILLS};
use crate::app::App;
use std::path::{Path, PathBuf};
/// Where a bot's drafts are, beside its skills folder and never inside it.
pub const DRAFTS: &str = "drafts";
/// The folder of drafts of the bot whose conversation is `key`, or the shared one.
pub fn drafts_dir(app: &App, key: &str, shared: bool) -> PathBuf {
    if shared {
        app.home.join("bots").join(DRAFTS)
    } else {
        folder(app, key).join(DRAFTS)
    }
}
/// A bot's own draft `name`: its SKILL.md.
pub fn draft_file(app: &App, key: &str, name: &str) -> PathBuf {
    drafts_dir(app, key, false).join(name).join("SKILL.md")
}
/// Whether the header of a skill has the line `draft: true`.
pub fn is_draft(text: &str) -> bool {
    text.strip_prefix("---\n")
        .and_then(|b| b.find("\n---").map(|end| &b[..end]))
        .is_some_and(|head| {
            head.lines()
                .any(|l| l.replace(' ', "").eq_ignore_ascii_case("draft:true"))
        })
}
/// The skill's text without the `draft: true` line of its header.
pub fn activated(text: &str) -> String {
    let Some(body) = text.strip_prefix("---\n") else {
        return text.to_string();
    };
    let end = body.find("\n---").unwrap_or(0);
    let head: Vec<&str> = body[..end]
        .lines()
        .filter(|l| !l.replace(' ', "").eq_ignore_ascii_case("draft:true"))
        .collect();
    format!("---\n{}{}", head.join("\n"), &body[end..])
}
/// Moves folder `from` to `to`. Over an older draft of the same name the new files win and that draft's others stay: nothing is deleted.
fn into(from: &Path, to: &Path) -> std::io::Result<()> {
    if !to.exists() {
        return std::fs::rename(from, to);
    }
    for e in std::fs::read_dir(from)? {
        let e = e?;
        let there = to.join(e.file_name());
        if e.file_type()?.is_dir() {
            into(&e.path(), &there)?;
        } else {
            std::fs::rename(e.path(), there)?;
        }
    }
    std::fs::remove_dir(from) // empty now
}
/// Moves every skill of `skills` that is a draft into `drafts` (a link, as a shared skill's, stays); the names moved.
fn move_drafts(skills: &Path, drafts: &Path) -> Vec<String> {
    let found = std::fs::read_dir(skills).into_iter().flatten().flatten();
    let found = found.filter(|e| e.file_type().is_ok_and(|t| t.is_dir()));
    let draft = |e: &std::fs::DirEntry| {
        is_draft(&std::fs::read_to_string(e.path().join("SKILL.md")).unwrap_or_default())
    };
    let moved = |e: std::fs::DirEntry| {
        let to = drafts.join(e.file_name());
        std::fs::create_dir_all(drafts).ok()?;
        into(&e.path(), &to).ok()?;
        e.file_name().into_string().ok()
    };
    found.filter(draft).filter_map(moved).collect()
}
/// Every draft found among the bot's skills or the shared ones goes to its drafts folder: whatever its author wrote where, no agent loads it.
pub fn sweep(app: &App, key: &str) -> Vec<String> {
    let mut moved = move_drafts(&folder(app, key).join(SKILLS), &drafts_dir(app, key, false));
    moved.extend(move_drafts(
        &app.home.join("bots/skills"),
        &drafts_dir(app, key, true),
    ));
    moved
}

#[cfg(test)]
#[path = "../../checks/draft_moves.rs"]
mod tests;
