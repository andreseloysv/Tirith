//! The plan: the project's markdown checklist (`## N. Title` steps, `- [ ] item` lines). Its content changes
//! (new item, ticked item) are written to the file at once and committed in batches; which lane works on
//! an item lives in the daemon, not in the file. The daemon is the only writer. An item may carry its own check:
//! `- [ ] Text (check: `cmd`)`; the merge queue runs it, and ticks a merged item in the merge commit itself.

use crate::app::{git, Shared};
use regex::Regex;
use serde::Serialize;
use serde_json::json;
use std::path::PathBuf;
use std::sync::LazyLock;

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
            if let Some((num, title)) = head.split_once(". ") {
                if !num.is_empty() && num.chars().all(|c| c.is_ascii_digit()) {
                    steps.push(Step { num: num.into(), title: title.trim().into(), items: vec![], line: i });
                    continue;
                }
            }
            steps.push(Step { num: String::new(), title: head.trim().into(), items: vec![], line: i });
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
    let cfg = app.cfg();
    let p = cfg.project(project).ok_or(format!("unknown project '{project}'"))?;
    let repo = p.repo();
    Ok(repo.join(app.manifest(&repo).roadmap))
}

pub fn read(app: &Shared, project: &str) -> Result<Vec<Step>, String> {
    let file = path(app, project)?;
    let text = std::fs::read_to_string(&file).map_err(|_| format!("{} has no plan file ({})", project, file.display()))?;
    Ok(parse(&text))
}

pub fn is_open(app: &Shared, project: &str, step: &str, item: &str) -> Result<bool, String> {
    let steps = read(app, project)?;
    let s = steps.iter().find(|s| s.num == step).ok_or(format!("the plan of {project} has no step {step}"))?;
    Ok(s.items.iter().any(|i| !i.done && i.text == clean(item)))
}

/// One item of the plan in the main checkout.
pub fn find(app: &Shared, project: &str, step: &str, item: &str) -> Option<Item> {
    let steps = read(app, project).ok()?;
    steps.into_iter().find(|s| s.num == step)?.items.into_iter().find(|i| i.text == item)
}

/// Ticks the open `(step, item)` pairs in a plan text: the new text and how many were ticked.
pub fn tick_text(text: &str, items: &[(String, String)]) -> (String, usize) {
    let mut lines: Vec<String> = text.lines().map(String::from).collect();
    let mut n = 0;
    for s in parse(text) {
        for it in s.items.iter().filter(|it| !it.done && items.iter().any(|(st, i)| *st == s.num && *i == it.text)) {
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
    const OK: [&str; 16] = ["serve", "hook", "hooks", "status", "plan", "lane", "ask", "answer", "ask-owner", "autopilot", "say", "room", "net", "models", "guard", "help"];
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
    if let Some(p) = CHECK.captures(&text).and_then(|c| check_problem(&c[1])) {
        return Err(p);
    }
    change(app, project, |lines, steps| {
        let i = steps.iter().position(|s| s.num == step).ok_or(format!("the plan has no step {step}"))?;
        if steps[i].items.iter().any(|it| it.text == text) {
            return Err(format!("step {step} already has that item"));
        }
        let end = steps.get(i + 1).map(|s| s.line).unwrap_or(lines.len());
        let mut at = end;
        while at > steps[i].line + 1 && lines[at - 1].trim().is_empty() {
            at -= 1;
        }
        lines.insert(at, format!("- [ ] {text}"));
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
        Ok(_) => {
            app.plan_dirty.lock().unwrap().remove(project);
        }
        Err(e) => app.event("", project, "error", "plan", &format!("plan commit failed: {e}"), ""),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn plan_add_rejects_invalid_checks() {
        let b = super::check_problem;
        assert!(
            ["tirith run -s test", "tirith check", "cd collector && ls", "cargo test && tirith run"].iter().all(|c| b(c).is_some())
                && ["cargo test -q x", "tirith plan show p", "cd collectors"].iter().all(|c| b(c).is_none())
        );
    }
}
