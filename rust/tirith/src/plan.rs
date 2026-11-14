//! The plan: the project's markdown checklist, written by the daemon only; an item may carry its own `(check: `cmd`)` that the merge queue runs.

use crate::app::{git, Shared};
use regex::Regex;
use serde::Serialize;
use serde_json::json;
use std::{path::PathBuf, sync::LazyLock};

static CHECK: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\(check:\s*`([^`]+)`\s*\)").unwrap());
static STATUS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\s*\((en curso|hecho|en revisión|falló|bloqueado|parcial|in progress|done)\b[^)]*\)\s*$").unwrap());

#[derive(Serialize, Clone)]
pub struct Item {
    pub text: String,
    pub done: bool,
    /// The command that proves the item is done, from `(check: `cmd`)` in its text.
    pub check: Option<String>,
    #[serde(skip)]
    line: usize,
}

#[derive(Serialize, Clone)]
pub struct Step {
    pub num: String,
    pub title: String,
    pub items: Vec<Item>,
    #[serde(skip)]
    line: usize,
}

/// An item's text without Tirith's old annotations (status notes, `<!-- tirith:… -->`).
pub fn clean(text: &str) -> String {
    let text = text.split("<!--").next().unwrap_or("").trim();
    STATUS.replace(text, "").trim().to_string()
}

pub fn parse(text: &str) -> Vec<Step> {
    let mut steps: Vec<Step> = vec![];
    for (i, line) in text.lines().enumerate() {
        if let Some(head) = line.strip_prefix("## ") {
            let (num, title) = head.split_once(". ").filter(|(n, _)| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit())).unwrap_or(("", head));
            steps.push(Step { num: num.into(), title: title.trim().into(), items: vec![], line: i });
        } else if let Some(step) = steps.last_mut() {
            let (done, rest) = match line.get(..6) {
                Some("- [ ] ") => (false, &line[6..]),
                Some("- [x] ") | Some("- [X] ") => (true, &line[6..]),
                _ => continue,
            };
            let check = CHECK.captures(rest).map(|c| c[1].to_string());
            step.items.push(Item { text: clean(rest), done, check, line: i });
        }
    }
    steps.retain(|s| !s.num.is_empty());
    steps
}

pub fn path(app: &Shared, project: &str) -> Result<PathBuf, String> {
    let repo = app.cfg().project(project).ok_or(format!("unknown project '{project}'"))?.repo();
    Ok(repo.join(app.manifest(&repo).roadmap))
}

pub fn read(app: &Shared, project: &str) -> Result<Vec<Step>, String> {
    let file = path(app, project)?;
    std::fs::read_to_string(&file).map(|text| parse(&text)).map_err(|_| format!("{} has no plan file ({})", project, file.display()))
}

pub fn is_open(app: &Shared, project: &str, step: &str, item: &str) -> Result<bool, String> {
    let steps = read(app, project)?;
    let s = steps.iter().find(|s| s.num == step).ok_or(format!("the plan of {project} has no step {step}"))?;
    Ok(s.items.iter().any(|i| !i.done && same(&i.text, &clean(item))))
}

/// Two item texts that name the same item: equal once their `(check: ...)` is left out. A lane carries the text its item had when it started; when only the check is rewritten meanwhile, the lane still belongs to that item (it is
/// checked with the check the plan has now, and ticked when it is merged).
pub fn same(a: &str, b: &str) -> bool {
    a == b || CHECK.replace(a, "").trim() == CHECK.replace(b, "").trim()
}

pub fn find(app: &Shared, project: &str, step: &str, item: &str) -> Option<Item> {
    read(app, project).ok()?.into_iter().find(|s| s.num == step)?.items.into_iter().find(|i| same(&i.text, item))
}

/// Ticks the open `(step, item)` pairs in a plan text: the new text and how many were ticked.
pub fn tick_text(text: &str, items: &[(String, String)]) -> (String, usize) {
    let mut lines: Vec<String> = text.lines().map(String::from).collect();
    let mut n = 0;
    for s in parse(text) {
        for it in s.items.iter().filter(|it| !it.done && items.iter().any(|(st, i)| *st == s.num && same(i, &it.text))) {
            lines[it.line] = lines[it.line].replacen("- [ ] ", "- [x] ", 1);
            n += 1;
        }
    }
    (lines.join("\n") + "\n", n)
}

/// Applies `edit` to the plan's lines under the writer lock, then queues the commit.
async fn change(app: &Shared, project: &str, edit: impl FnOnce(&mut Vec<String>, &[Step]) -> Result<(), String>) -> Result<(), String> {
    let _guard = app.plan_lock.lock().await;
    let file = path(app, project)?;
    let text = std::fs::read_to_string(&file).map_err(|e| e.to_string())?;
    let mut lines: Vec<String> = text.lines().map(String::from).collect();
    edit(&mut lines, &parse(&text))?;
    std::fs::write(&file, lines.join("\n") + "\n").map_err(|e| e.to_string())?;
    app.plan_dirty.lock().unwrap().insert(project.to_string());
    app.emit("plan", json!({ "project": project }));
    Ok(())
}

/// Why an item's check can never pass: it calls a tirith subcommand that does not exist, or the removed `collector/`.
pub fn check_problem(check: &str) -> Option<String> {
    static CALL: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?:^|[\s;&|(])(?:(cd\s+(?:\./)?collector(?:[/\s;&]|$))|tirith\s+([A-Za-z-][\w-]*))").unwrap());
    const OK: [&str; 18] = ["serve", "hook", "hooks", "status", "plan", "lane", "ask", "answer", "ask-owner", "card", "autopilot", "say", "room", "net", "models", "guard", "project", "help"];
    CALL.captures_iter(check).find_map(|c| {
        c.get(2).map_or(Some("the check does `cd collector`, a removed folder (the daemon is Rust: use cargo test -q <name>)".to_string()), |s| {
            (!OK.contains(&s.as_str())).then(|| format!("the check calls `tirith {}`, which is not a tirith command", s.as_str()))
        })
    })
}

pub async fn add(app: &Shared, project: &str, step: &str, text: &str) -> Result<(), String> {
    let text = clean(text);
    if text.is_empty() {
        return Err("the item needs a text".into());
    }
    CHECK.captures(&text).and_then(|c| check_problem(&c[1])).map_or(Ok(()), Err)?;
    // A forbidden word of the project's commit guard would make the plan uncommittable and block every merge.
    if let Some(p) = app.cfg().project(project).cloned() {
        let words = crate::guard::words(app, &p);
        if let Some(m) = regex::Regex::new(&format!(r"(?i)\b({})\b", words.join("|"))).ok().filter(|_| !words.is_empty()).and_then(|r| r.find(&text)) {
            return Err(format!("\"{}\" is a forbidden word of this project's commit guard: say it another way", m.as_str()));
        }
    }
    change(app, project, |lines, steps| {
        let i = steps.iter().position(|s| s.num == step).ok_or(format!("the plan has no step {step}"))?;
        (!(steps[i].items.iter().any(|it| same(&it.text, &text)))).then_some(()).ok_or_else(|| format!("step {step} already has that item"))?;
        let end = steps.get(i + 1).map(|s| s.line).unwrap_or(lines.len());
        let at = (steps[i].line + 1..end).rev().find(|&k| !lines[k].trim().is_empty()).map_or(steps[i].line + 1, |k| k + 1);
        lines.insert(at, format!("- [ ] {text}"));
        Ok(())
    })
    .await
}

/// Takes an open item out of the plan (its lane was deleted).
pub async fn drop_item(app: &Shared, project: &str, step: &str, item: &str) -> Result<(), String> {
    change(app, project, |lines, steps| {
        let it = steps.iter().filter(|s| s.num == step).flat_map(|s| &s.items).find(|i| !i.done && same(&i.text, item)).ok_or("no such open item")?;
        lines.remove(it.line);
        Ok(())
    })
    .await
}

/// Commits the plan files changed since the last commit: one commit per project, whatever the number of changes.
pub async fn flush(app: &Shared, only: Option<&str>) {
    let projects: Vec<String> = app.plan_dirty.lock().unwrap().iter().filter(|p| only.is_none_or(|o| o == *p)).cloned().collect();
    for project in projects {
        let _guard = app.plan_lock.lock().await;
        flush_locked(app, &project).await;
    }
}

/// `flush` of one project for a caller that already holds the plan lock.
pub async fn flush_locked(app: &Shared, project: &str) {
    if !app.plan_dirty.lock().unwrap().contains(project) {
        return;
    }
    let (Ok(file), Some(repo)) = (path(app, project), app.cfg().project(project).map(|p| p.repo())) else { return };
    let rel = file.strip_prefix(&repo).unwrap_or(&file).to_string_lossy().to_string();
    let pending = git(&repo, &["status", "--porcelain", "--", &rel]).await.unwrap_or_default();
    let result = if pending.is_empty() { Ok(String::new()) } else { git(&repo, &["commit", "-q", "-m", "Plan: update from Tirith", "--", &rel]).await };
    match result {
        Ok(_) => _ = app.plan_dirty.lock().unwrap().remove(project),
        Err(e) => app.event("", project, "error", "plan", &format!("plan commit failed: {e}"), ""),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn plan_add_rejects_invalid_checks() {
        let b = super::check_problem;
        assert!(["tirith run -s test", "tirith check", "cd collector && ls", "cargo test && tirith run"].iter().all(|c| b(c).is_some()) && ["cargo test -q x", "tirith plan show p", "cd collectors"].iter().all(|c| b(c).is_none()));
    }
    #[test]
    fn a_rewritten_check_keeps_the_lane_on_its_item() {
        let (old, now) = ("Make the chat a PWA (check: `npm test`)", "Make the chat a PWA (check: `export PATH=/opt/homebrew/bin:$PATH; npm test`)");
        assert!(super::same(old, now) && super::same(old, "Make the chat a PWA") && !super::same(old, "Make the chat a PWA, with icons (check: `npm test`)"));
        // The lane started with the old text: merged, it ticks the item as the plan has it now, and no other.
        let (text, n) = super::tick_text(&format!("## 1. Step\n- [ ] {now}\n- [ ] Another\n"), &[("1".into(), old.into())]);
        assert_eq!((n, text.contains(&format!("- [x] {now}")), text.contains("- [ ] Another")), (1, true, true));
    }
}
