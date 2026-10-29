//! A project's plan, read from its markdown checklist (`roadmap.py`, `docs/CONTRACT-roadmap.md`).
//!
//! Each `## <n>. <title>` heading is a step; `- [ ]` / `- [x]` lines are its items. Parsing never fails:
//! a missing or unreadable file gives `None`.
//!
//! Not ported yet (plan lane): `tick`, `edit`, `owner_parts`, `agent_items`, `read_intro`.

use crate::manifest::{self, Manifest};
use crate::projects::real;
use crate::pytext::{expanduser, norm, split_words, splitlines, strip, universal_newlines};
use crate::state::{Plan, PlanItem, PlanStep, Roadmap, RoadmapItem, RoadmapStep};
use chrono::NaiveDate;
use regex::Regex;
use serde_json::Value;
use sha1::{Digest, Sha1};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

pub const MAX_BYTES: u64 = 256 * 1024;
pub const DEFAULT_FILES: [&str; 2] = ["docs/TODO.md", "TODO.md"];

fn re(pattern: &str) -> Regex {
    Regex::new(pattern).expect("static regex")
}

static STEP_RE: LazyLock<Regex> = LazyLock::new(|| re(r"^##\s+(\d+)\.\s*(.+?)\s*#*\s*$"));
static HEADING_RE: LazyLock<Regex> = LazyLock::new(|| re(r"^#{1,6}\s"));
static ITEM_RE: LazyLock<Regex> = LazyLock::new(|| re(r"^\s*[-*+]\s+\[([ xX])\]\s*(.*)$"));
static BULLET_RE: LazyLock<Regex> = LazyLock::new(|| re(r"^\s*[-*+]\s"));
static DEADLINE_RE: LazyLock<Regex> = LazyLock::new(|| re(r"(?i)^\s*deadline\s*:\s*(\S+)\s*$"));
static OUT_RE: LazyLock<Regex> = LazyLock::new(|| re(r"(?i)\((?:fuera de este plan|out of this plan)"));
static OWNER_RE: LazyLock<Regex> = LazyLock::new(|| re(r"(?i)\((?:dueño|owner)"));
static FROM_RE: LazyLock<Regex> = LazyLock::new(|| re(r"(?i)\((?:desde|from)\s+(\d{4}-\d{2}-\d{2})"));
static TITLE_LABEL_RE: LazyLock<Regex> = LazyLock::new(|| re(r"(?i)^(?:todo|plan)\s*:\s*"));

/// A parsed checklist item.
#[derive(Debug, Clone, PartialEq)]
pub struct Item {
    pub text: String,
    pub done: bool,
}

/// A parsed `## <n>. <title>` step.
#[derive(Debug, Clone, PartialEq)]
pub struct Step {
    pub id: String,
    pub title: String,
    pub items: Vec<Item>,
}

/// `(who, date)` from the tags in an item's text: `out` (not in this plan), `owner`, `date` (blocked
/// until `YYYY-MM-DD`) or `agent`.
pub fn tag(text: &str) -> (&'static str, Option<String>) {
    if OUT_RE.is_match(text) {
        return ("out", None);
    }
    if OWNER_RE.is_match(text) {
        return ("owner", None);
    }
    if let Some(found) = FROM_RE.captures(text) {
        let day = &found[1];
        if NaiveDate::parse_from_str(day, "%Y-%m-%d").is_ok() {
            return ("date", Some(day.to_string()));
        }
    }
    ("agent", None)
}

/// `agent|owner|date|out` for an item today: a `date` item whose day has come is an agent's.
pub fn who_on(text: &str, today: NaiveDate) -> (&'static str, Option<String>) {
    match tag(text) {
        ("date", Some(until)) if NaiveDate::parse_from_str(&until, "%Y-%m-%d").is_ok_and(|d| d <= today) => {
            ("agent", None)
        }
        other => other,
    }
}

/// The stable id of the owner item raised for an owner-only roadmap item.
pub fn owner_item_id(project: &str, step_id: &str, text: &str) -> String {
    let digest = Sha1::digest(split_words(text).join(" ").as_bytes());
    let hex: String = digest.iter().map(|b| format!("{b:02x}")).collect();
    format!("owner-{project}-{step_id}-{}", &hex[..8])
}

/// Steps in file order.
pub fn parse(text: &str) -> Vec<Step> {
    let mut steps: Vec<Step> = Vec::new();
    let mut in_step = false;
    let mut in_item = false;
    for line in splitlines(text) {
        if let Some(found) = STEP_RE.captures(line) {
            steps.push(Step { id: found[1].to_string(), title: found[2].to_string(), items: Vec::new() });
            in_step = true;
            in_item = false;
            continue;
        }
        if HEADING_RE.is_match(line) {
            in_step = false;
            in_item = false;
            continue;
        }
        if !in_step {
            continue;
        }
        let step = steps.last_mut().expect("in_step implies a step");
        if let Some(found) = ITEM_RE.captures(line) {
            step.items.push(Item { text: strip(&found[2]).to_string(), done: &found[1] != " " });
            in_item = true;
        } else if strip(line).is_empty() || BULLET_RE.is_match(line) {
            in_item = false;
        } else if in_item {
            let item = step.items.last_mut().expect("in_item implies an item");
            item.text = format!("{} {}", item.text, strip(line));
        }
    }
    steps
}

/// `empty|done|active|ready|blocked-owner|blocked-date|todo` (see `docs/CONTRACT-roadmap.md`).
pub fn step_status(done: usize, total: usize, working: bool, agent_open: usize, owner_open: usize, date_open: usize) -> &'static str {
    if total == 0 {
        "empty"
    } else if done == total {
        "done"
    } else if working {
        "active"
    } else if agent_open > 0 {
        "ready"
    } else if owner_open > 0 {
        "blocked-owner"
    } else if date_open > 0 {
        "blocked-date"
    } else {
        "todo"
    }
}

/// The roadmap file: the configured path, else `docs/TODO.md`, else `TODO.md`.
pub fn find_file(folder: &Path, configured: Option<&str>) -> Option<PathBuf> {
    let names: Vec<&str> = match configured.filter(|c| !c.is_empty()) {
        Some(name) => vec![name],
        None => DEFAULT_FILES.to_vec(),
    };
    names.into_iter().find_map(|name| {
        let path = norm(&expanduser(name));
        let path = if path.is_absolute() { path } else { folder.join(path) };
        path.is_file().then_some(path)
    })
}

/// Parse the checklist at `path`; `None` when it is too big or unreadable.
fn parsed(path: &Path) -> Option<Vec<Step>> {
    if fs::metadata(path).ok()?.len() > MAX_BYTES {
        return None;
    }
    Some(parse(&read_text(path)?))
}

fn read_text(path: &Path) -> Option<String> {
    String::from_utf8(fs::read(path).ok()?).ok().map(|t| universal_newlines(&t))
}

/// The first `# ` heading (a leading `TODO:` / `Plan:` label is dropped); may be empty.
pub fn title_of(text: &str) -> String {
    for line in splitlines(text) {
        if let Some(rest) = line.strip_prefix("# ") {
            return TITLE_LABEL_RE.replace(strip(rest), "").into_owned();
        }
    }
    String::new()
}

/// The value of a `Deadline: <iso>` line before the first step (may be `None`).
pub fn deadline_of(text: &str) -> Option<String> {
    for line in splitlines(text) {
        if line.starts_with("##") {
            break;
        }
        if let Some(found) = DEADLINE_RE.captures(line) {
            return Some(found[1].to_string());
        }
    }
    None
}

/// The current plan first (`roadmap`), then the files of `plans` (each once, existing only).
pub fn plan_paths(folder: &Path, manifest: &Manifest) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = Vec::new();
    let wanted = std::iter::once(manifest.roadmap.as_deref()).chain(manifest.plans.iter().map(|p| Some(p.as_str())));
    for name in wanted {
        let Some(path) = find_file(folder, name) else { continue };
        if !out.iter().any(|p| real(p) == real(&path)) {
            out.push(path);
        }
    }
    out
}

/// agent rows working on a step: `step id -> agent ids`.
fn working(agents: &[Value], project: &str) -> HashMap<String, Vec<String>> {
    let mut out: HashMap<String, Vec<String>> = HashMap::new();
    for agent in agents {
        let step = agent.get("step").filter(|s| json_truthy(s));
        if agent.get("project").and_then(Value::as_str) == Some(project)
            && agent.get("status").and_then(Value::as_str) == Some("working")
        {
            if let Some(step) = step {
                out.entry(json_str(step)).or_default().push(agent.get("id").map(json_str).unwrap_or_default());
            }
        }
    }
    out
}

fn json_truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|f| f != 0.0),
        Value::String(s) => !s.is_empty(),
        Value::Array(a) => !a.is_empty(),
        Value::Object(o) => !o.is_empty(),
    }
}

fn json_str(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

/// `state.projects[].plans`: every plan of the project, the current one (`roadmap`) first.
pub fn build_plans(folder: Option<&Path>, project: &str, agents: &[Value]) -> Vec<Plan> {
    let Some(folder) = folder else { return Vec::new() };
    let manifest = manifest::load(folder);
    let working = working(agents, project);
    let mut out: Vec<Plan> = Vec::new();
    for path in plan_paths(folder, &manifest) {
        if let Some(steps) = parsed(&path) {
            let current = out.is_empty();
            out.push(plan_row(&path, &steps, &manifest, current, &working));
        }
    }
    out
}

fn plan_row(path: &Path, steps: &[Step], manifest: &Manifest, current: bool, working: &HashMap<String, Vec<String>>) -> Plan {
    let rows: Vec<PlanStep> = steps
        .iter()
        .map(|step| {
            let done = step.items.iter().filter(|i| i.done).count();
            let mine = if current { working.get(&step.id).cloned().unwrap_or_default() } else { Vec::new() };
            PlanStep {
                id: step.id.clone(),
                title: step.title.clone(),
                status: step_status(done, step.items.len(), !mine.is_empty(), 0, 0, 0).to_string(),
                done,
                total: step.items.len(),
                items: step.items.iter().map(|i| PlanItem { text: i.text.clone(), done: i.done }).collect(),
                agents: mine,
            }
        })
        .collect();
    let text = read_text(path).unwrap_or_default();
    let done: usize = rows.iter().map(|r| r.done).sum();
    let total: usize = rows.iter().map(|r| r.total).sum();
    let stem = path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    let title = title_of(&text);
    Plan {
        id: stem.clone(),
        title: if title.is_empty() { stem } else { title },
        current,
        file: path.to_string_lossy().into_owned(),
        deadline: deadline_of(&text).or_else(|| if current { manifest.deadline.clone() } else { None }),
        done,
        total,
        finished: total > 0 && done == total,
        steps: rows,
    }
}

/// `state.projects[].roadmap` for a project folder; `None` when it has no roadmap file.
pub fn build(folder: Option<&Path>, project: &str, agents: &[Value], today: NaiveDate) -> Option<Roadmap> {
    let folder = folder?;
    let manifest = manifest::load(folder);
    let path = plan_paths(folder, &manifest).into_iter().next()?; // the current plan
    let steps = parsed(&path)?;
    let working = working(agents, project);
    let mut rows = Vec::new();
    for step in &steps {
        let mut items = Vec::new();
        for raw in &step.items {
            let (who, until) = who_on(&raw.text, today);
            let mut item = RoadmapItem { text: raw.text.clone(), done: raw.done, who: who.to_string(), until: None, owner_item: None };
            if who == "date" {
                item.until = until;
            } else if who == "owner" && !raw.done {
                item.owner_item = Some(owner_item_id(project, &step.id, &raw.text));
            }
            items.push(item);
        }
        let counted: Vec<&RoadmapItem> = items.iter().filter(|i| i.who != "out").collect();
        let done = counted.iter().filter(|i| i.done).count();
        let mine = working.get(&step.id).cloned().unwrap_or_default();
        let open = |who: &str| -> Vec<&&RoadmapItem> { counted.iter().filter(|i| !i.done && i.who == who).collect() };
        let (agent_open, owner_open, date_open) = (open("agent"), open("owner"), open("date"));
        let status = step_status(done, counted.len(), !mine.is_empty(), agent_open.len(), owner_open.len(), date_open.len());
        let until = (status == "blocked-date").then(|| date_open.iter().filter_map(|i| i.until.clone()).min()).flatten();
        rows.push(RoadmapStep {
            id: step.id.clone(),
            title: step.title.clone(),
            status: status.to_string(),
            done,
            total: counted.len(),
            items,
            agents: mine,
            until,
        });
    }
    Some(Roadmap {
        file: path.to_string_lossy().into_owned(),
        deadline: manifest.deadline,
        done: rows.iter().map(|r| r.done).sum(),
        total: rows.iter().map(|r| r.total).sum(),
        steps: rows,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const SAMPLE: &str = "# TODO: Demo plan\n\nDeadline: 2026-12-01T10:00:00Z\n\nIntro text.\n\n## 1. First step\n\n\
- [x] done thing\n- [ ] open thing that wraps\n      onto a second line\n  - [ ] nested item\n\n\
## Order\n- [ ] not a step item\n\n## 2. Second step\n* [X] star done\n+ [ ] plus open\n";

    fn day(text: &str) -> NaiveDate {
        NaiveDate::parse_from_str(text, "%Y-%m-%d").unwrap()
    }

    fn project(files: &[(&str, &str)]) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        for (name, text) in files {
            let path = dir.path().join(name);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, text).unwrap();
        }
        dir
    }

    #[test]
    fn parses_steps_items_and_wrapped_lines() {
        let steps = parse(SAMPLE);
        assert_eq!(steps.len(), 2);
        assert_eq!((steps[0].id.as_str(), steps[0].title.as_str()), ("1", "First step"));
        let texts: Vec<_> = steps[0].items.iter().map(|i| (i.text.as_str(), i.done)).collect();
        assert_eq!(
            texts,
            [("done thing", true), ("open thing that wraps onto a second line", false), ("nested item", false)]
        );
        assert_eq!(steps[1].items.len(), 2);
        assert!(steps[1].items[0].done);
    }

    #[test]
    fn unnumbered_heading_ends_the_step() {
        let steps = parse("## 1. A\n- [ ] x\n## Notes\n- [ ] y\n");
        assert_eq!(steps.len(), 1);
        assert_eq!(steps[0].items.len(), 1);
    }

    #[test]
    fn trailing_hashes_leave_the_title() {
        assert_eq!(parse("## 3.  Title here ##\n")[0].title, "Title here");
    }

    #[test]
    fn tags_decide_who() {
        assert_eq!(tag("do it (dueño: payment)").0, "owner");
        assert_eq!(tag("x (OWNER thing)").0, "owner");
        assert_eq!(tag("x (fuera de este plan)").0, "out");
        assert_eq!(tag("x (desde 2026-12-01)"), ("date", Some("2026-12-01".into())));
        assert_eq!(tag("x (desde 2026-13-45)").0, "agent");
        assert_eq!(who_on("x (desde 2026-12-01)", day("2026-12-01")).0, "agent");
        assert_eq!(who_on("x (desde 2026-12-01)", day("2026-11-30")).0, "date");
    }

    #[test]
    fn status_rules() {
        assert_eq!(step_status(0, 0, false, 0, 0, 0), "empty");
        assert_eq!(step_status(2, 2, true, 0, 0, 0), "done");
        assert_eq!(step_status(1, 2, true, 1, 0, 0), "active");
        assert_eq!(step_status(1, 2, false, 1, 1, 1), "ready");
        assert_eq!(step_status(1, 3, false, 0, 1, 1), "blocked-owner");
        assert_eq!(step_status(1, 3, false, 0, 0, 1), "blocked-date");
        assert_eq!(step_status(1, 3, false, 0, 0, 0), "todo");
    }

    #[test]
    fn title_and_deadline() {
        assert_eq!(title_of(SAMPLE), "Demo plan");
        assert_eq!(deadline_of(SAMPLE).as_deref(), Some("2026-12-01T10:00:00Z"));
        assert_eq!(deadline_of("## 1. A\nDeadline: 2026-01-01\n"), None);
    }

    #[test]
    fn owner_item_id_matches_python() {
        // python: hashlib.sha1(b"do the thing").hexdigest()[:8] == "9a1b1d6e" is checked in the parity fixtures;
        // here: whitespace is squashed before hashing
        assert_eq!(owner_item_id("p", "3", "a  b\n c"), owner_item_id("p", "3", "a b c"));
        assert!(owner_item_id("p", "3", "x").starts_with("owner-p-3-"));
    }

    #[test]
    fn build_counts_and_statuses() {
        let text = "## 1. Mixed\n- [x] done\n- [ ] agent one\n- [ ] gated (dueño: payment pay)\n\
- [ ] later (desde 2026-12-25)\n- [ ] elsewhere (fuera de este plan)\n\n## 2. Owner only\n- [ ] x (owner)\n\n\
## 3. Waiting\n- [ ] y (desde 2026-12-25)\n- [ ] z (desde 2026-12-20)\n\n## 4. Nothing\n";
        let dir = project(&[("docs/TODO.md", text), ("tirith.toml", "deadline = \"2026-12-31T00:00:00Z\"\n")]);
        let built = build(Some(dir.path()), "p", &[], day("2026-12-01")).unwrap();
        assert_eq!(built.deadline.as_deref(), Some("2026-12-31T00:00:00Z"));
        let statuses: Vec<_> = built.steps.iter().map(|s| s.status.as_str()).collect();
        assert_eq!(statuses, ["ready", "blocked-owner", "blocked-date", "empty"]);
        let first = &built.steps[0];
        assert_eq!((first.done, first.total, first.items.len()), (1, 4, 5));
        assert!(first.items[2].owner_item.is_some() && first.items[3].until.as_deref() == Some("2026-12-25"));
        assert_eq!(built.steps[2].until.as_deref(), Some("2026-12-20"));
        assert_eq!((built.done, built.total), (1, 4 + 1 + 2));
        assert!(built.file.ends_with("docs/TODO.md"));
    }

    #[test]
    fn working_agent_makes_the_step_active() {
        let dir = project(&[("TODO.md", "## 1. A\n- [ ] x\n")]);
        let agents = [json!({"id": "w1", "project": "p", "status": "working", "step": 1}),
                      json!({"id": "w2", "project": "other", "status": "working", "step": "1"})];
        let built = build(Some(dir.path()), "p", &agents, day("2026-01-01")).unwrap();
        assert_eq!(built.steps[0].status, "active");
        assert_eq!(built.steps[0].agents, ["w1"]);
    }

    #[test]
    fn default_files_configured_file_and_missing() {
        assert!(build(Some(project(&[]).path()), "p", &[], day("2026-01-01")).is_none());
        assert!(build(None, "p", &[], day("2026-01-01")).is_none());
        let both = project(&[("TODO.md", "## 1. root\n- [ ] a\n"), ("docs/TODO.md", "## 1. docs\n- [ ] a\n")]);
        assert_eq!(build(Some(both.path()), "p", &[], day("2026-01-01")).unwrap().steps[0].title, "docs");
        let configured = project(&[("TODO.md", "## 1. root\n- [ ] a\n"), ("tirith.toml", "roadmap = \"plan/none.md\"\n")]);
        assert!(build(Some(configured.path()), "p", &[], day("2026-01-01")).is_none(), "a configured file does not fall back");
    }

    #[test]
    fn oversized_file_is_ignored() {
        let big = format!("## 1. A\n- [ ] {}\n", "x".repeat(MAX_BYTES as usize));
        let dir = project(&[("TODO.md", &big)]);
        assert!(build(Some(dir.path()), "p", &[], day("2026-01-01")).is_none());
    }

    #[test]
    fn plans_list_the_current_one_first_without_repeats() {
        let dir = project(&[
            ("docs/TODO.md", "# Plan: Main\nDeadline: 2026-05-05\n## 1. A\n- [x] a\n"),
            ("docs/plans/beta.md", "## 1. B\n- [ ] b\n"),
            ("tirith.toml", "deadline = \"2026-06-06\"\nplans = [\"docs/TODO.md\", \"docs/plans/beta.md\", \"docs/plans/none.md\"]\n"),
        ]);
        let plans = build_plans(Some(dir.path()), "p", &[]);
        assert_eq!(plans.len(), 2);
        assert!(plans[0].current && plans[0].finished);
        assert_eq!((plans[0].id.as_str(), plans[0].title.as_str()), ("TODO", "Main"));
        assert_eq!(plans[0].deadline.as_deref(), Some("2026-05-05"));
        assert!(!plans[1].current);
        assert_eq!((plans[1].title.as_str(), plans[1].deadline.as_deref()), ("beta", None));
    }
}
