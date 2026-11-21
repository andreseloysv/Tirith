//! The "bots" brief a pass reads (the Coach's): why it runs, its own last pass, and per bot its numbers (`GET /api/bots/<id>/numbers`), its routines, its skills with the sections of the factory's skill template they lack, and the size of its memories.
use crate::app::Shared;
use crate::server::s;
use serde_json::Value;

/// The `## ` titles of the factory's skill template.
fn titles(app: &Shared) -> Vec<String> {
    let template = app.cfg().factory.prompt("skill");
    template
        .lines()
        .filter_map(|l| l.strip_prefix("## ").map(|t| t.trim().to_string()))
        .collect()
}

/// One skill's line: its name, where it lives, and the template's sections its SKILL.md lacks.
fn skill_line((name, text): &(String, String), whose: &str, titles: &[String]) -> String {
    let has = |t: &String| text.lines().any(|l| l.trim() == format!("## {t}"));
    let lacks: Vec<&str> = titles
        .iter()
        .filter(|t| !has(t))
        .map(|t| t.as_str())
        .collect();
    let lacks = if lacks.is_empty() {
        "none".into()
    } else {
        lacks.join(", ")
    };
    format!(
        "- {name} ({whose}, {} chars): lacks {lacks}\n",
        text.chars().count()
    )
}

/// One routine's line: its id, title, trigger and task.
fn routine_line(r: &Value) -> String {
    let when = if s(r, "trigger") == "schedule" {
        format!("{} days {}", s(r, "at"), r["days"])
    } else {
        s(r, "when")
    };
    let off = if r["on"] == false { " (off)" } else { "" };
    format!(
        "- {} «{}» on {} {when}{off}: {}\n",
        s(r, "id"),
        s(r, "title"),
        s(r, "trigger"),
        s(r, "prompt")
    )
}

/// One bot's part of the brief.
fn bot_part(app: &Shared, b: &Value, titles: &[String]) -> String {
    let (key, id) = (s(b, "key"), s(b, "id"));
    let numbers = crate::numbers::numbers(app, &id)
        .map(|n| n.to_string())
        .unwrap_or_else(|e| e);
    let routines: String = b["routines"]
        .as_array()
        .into_iter()
        .flatten()
        .map(routine_line)
        .collect();
    let dir = crate::bots::folder(app, &key);
    let skills: String = crate::bots::skills(&dir.join(crate::bots::SKILLS), false)
        .iter()
        .map(|sk| skill_line(sk, "own", titles))
        .collect();
    let memory = std::fs::read_to_string(dir.join("memory.md")).unwrap_or_default();
    format!("## {} ({key})\nNumbers: {numbers}\nRoutines:\n{routines}Own skills:\n{skills}Memory: {} chars, {} lines\n\n", s(b, "name"), memory.chars().count(), memory.lines().count())
}

/// The last pass `role` made: its time and its answer.
fn last_pass(app: &Shared, role: &str) -> String {
    let rows = app.query(
        "SELECT at, text FROM ceo_reviews WHERE role = ?1 ORDER BY id DESC LIMIT 1",
        [role],
        |r| {
            Ok(format!(
                "{}: {}",
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?
            ))
        },
    );
    rows.into_iter().next().unwrap_or_else(|| "none yet".into())
}

pub(super) fn brief(app: &Shared, role: &str, why: &str) -> String {
    let titles = titles(app);
    let bots: String = crate::bots::all(app)
        .iter()
        .filter(|b| b["key"] != role && b["show"] != false)
        .map(|b| bot_part(app, b, &titles))
        .collect();
    let shared_dir = app.home.join("bots");
    let shared: String = crate::bots::skills(&shared_dir.join("skills"), true)
        .iter()
        .map(|sk| skill_line(sk, "shared", &titles))
        .collect();
    let memory = std::fs::read_to_string(shared_dir.join("memory.md")).unwrap_or_default();
    format!("# Why this pass\n{why}\n\n# Your last pass\n{}\n\n# Bots\n{bots}# Shared skills\n{shared}\n# Shared memory\n{} chars, {} lines\n", last_pass(app, role), memory.chars().count(), memory.lines().count())
}
