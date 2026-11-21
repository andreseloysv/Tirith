//! The owner's words while recording, and the turn at stop: the steps with those words under the step they follow, the skill template, and what to write and ask.
use super::state::{live, now_ms, push, read};
use crate::app::{App, Shared};
use crate::server::s;
use serde_json::{json, Value};
/// What the owner wrote in a bot's chat: kept with its recording when one is on (true), so it is the owner's voice for a step and starts no turn of its own.
pub fn heard(app: &App, conv: &str, text: &str) -> bool {
    live(&read(app, conv))
        && push(
            app,
            conv,
            json!({"kind": "said", "at": now_ms(), "text": text}),
        )
}
/// One entry as a line: a numbered step, or the owner's words under the step before.
fn line(n: &mut usize, e: &Value) -> String {
    let time = s(e, "at").get(11..19).unwrap_or("").to_string();
    let kind = s(e, "kind");
    if kind == "said" {
        return format!("   The owner wrote: «{}»", s(e, "text"));
    }
    *n += 1;
    let what = match kind.as_str() {
        "page" => format!("page «{}» {}", s(e, "title"), s(e, "url")),
        "click" => format!(
            "click «{}» ({}, {})",
            s(e, "text"),
            s(e, "role"),
            s(e, "selector")
        ),
        _ => format!("type «{}» = {}", s(e, "label"), s(e, "value")),
    };
    format!("{n}. {time} {what}")
}
/// The steps and words sorted by time (a step and words at the same moment keep the order they came in).
pub(super) fn lines(entries: &[Value]) -> String {
    let mut sorted: Vec<&Value> = entries.iter().collect();
    sorted.sort_by_key(|e| s(e, "at"));
    let mut n = 0;
    sorted
        .into_iter()
        .map(|e| line(&mut n, e))
        .collect::<Vec<_>>()
        .join("\n")
}
fn told(rec: &Value, template: &str) -> String {
    let entries = rec["steps"].as_array().cloned().unwrap_or_default();
    format!("[tirith] The owner just taught you a task: they did it once in their own Edge while Tirith's extension recorded each step (from {} to {}, times in UTC), and what they wrote in this chat meanwhile is under the step it follows. A value shown as ••• is a secret the extension masked: never ask for it or write it down.\n\n{}\n\nWrite this as a new skill of yours with the template below: .agents/skills/<name>/SKILL.md in your folder, with the line `draft: true` in its header after description. It stays a draft, used by no one, until the owner activates it in the Library. Do not run the task now. Then ask the owner here 3 to 5 short numbered questions about what one example cannot show: the decision rules (what to choose or skip, and when), what to do when a step fails, and which steps need their approval.\n\nThe template:\n{template}", s(rec, "since"), now_ms(), lines(&entries))
}
/// Hands the bot its recording at stop, in one turn; a recording with nothing in it only says so.
pub(super) fn over(app: &Shared, key: &str, rec: &Value) {
    let count = rec["steps"]
        .as_array()
        .map_or(0, |v| v.iter().filter(|e| e["kind"] != "said").count());
    if count == 0 {
        return app.chat("tirith", "Recording stopped with no steps.", key);
    }
    app.chat(
        "tirith",
        &format!("Recording stopped · {count} steps. Writing the draft skill and the questions."),
        key,
    );
    crate::turns::trigger(key, "teach", "");
    let template = app.cfg().factory.prompt("skill");
    crate::manager::deliver(app, key, &told(rec, template.trim()), true);
}
