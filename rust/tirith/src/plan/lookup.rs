//! Finding a project's plan and its items.
use super::item::{Item, Step};
use super::parse::{parse, CHECK};
use crate::app::Shared;
use std::path::PathBuf;

pub(super) fn path(app: &Shared, project: &str) -> Result<PathBuf, String> {
    let repo = app
        .cfg()
        .project(project)
        .ok_or(format!("unknown project '{project}'"))?
        .repo();
    Ok(repo.join(app.manifest(&repo).roadmap))
}

pub fn read(app: &Shared, project: &str) -> Result<Vec<Step>, String> {
    let file = path(app, project)?;
    std::fs::read_to_string(&file)
        .map(|text| parse(&text))
        .map_err(|_| format!("{} has no plan file ({})", project, file.display()))
}

/// Two item texts that name the same item: equal once their `(check: ...)` is left out. A lane carries the text its item had when it started; when only the check is rewritten meanwhile, the lane still belongs to that item (it is checked with the check the plan has now, and ticked when it is merged).
pub fn same(a: &str, b: &str) -> bool {
    a == b || CHECK.replace(a, "").trim() == CHECK.replace(b, "").trim()
}

/// The item's text without its `(check: ...)`: what names it for as long as it lives.
pub fn plain(text: &str) -> String {
    CHECK.replace(text, "").trim().to_string()
}

pub fn find(app: &Shared, project: &str, step: &str, item: &str) -> Option<Item> {
    read(app, project)
        .ok()?
        .into_iter()
        .find(|s| s.num == step)?
        .items
        .into_iter()
        .find(|i| same(&i.text, item))
}

/// Ticks the open `(step, item)` pairs in a plan text: the new text and how many were ticked.
pub fn tick_text(text: &str, items: &[(String, String)]) -> (String, usize) {
    let mut lines: Vec<String> = text.lines().map(String::from).collect();
    let tick: Vec<usize> = parse(text)
        .iter()
        .flat_map(|s| {
            s.items
                .iter()
                .filter(move |it| {
                    !it.done
                        && items
                            .iter()
                            .any(|(st, i)| *st == s.num && same(i, &it.text))
                })
                .map(|it| it.line)
        })
        .collect();
    tick.iter()
        .for_each(|&l| lines[l] = lines[l].replacen("- [ ] ", "- [x] ", 1));
    (lines.join("\n") + "\n", tick.len())
}
