//! The judge's note on a routine's turn: what its `routine.done` carries for the `turn-quality` eval (the routine's task, its answer and the «Cómo validar» of the skills it read), and the score copied to the turn's `score`.
use super::record::Turn;
use super::versions::skill_file;
use crate::app::Shared;
use rusqlite::params;
use serde_json::json;

/// The eval whose score lands on the turn of the event it judged.
pub const EVAL: &str = "turn-quality";

/// An answer's start and its end, where a run says how it went, in `most` characters.
fn ends(text: &str, most: usize) -> String {
    let all: Vec<char> = text.chars().collect();
    if all.len() <= most {
        return text.to_string();
    }
    let (head, tail) = (most * 2 / 5, most * 3 / 5);
    let part = |c: &[char]| c.iter().collect::<String>();
    format!(
        "{} […] {}",
        part(&all[..head]),
        part(&all[all.len() - tail..])
    )
}

/// The «Cómo validar» section of a skill's text: the lines after its `## Cómo validar` heading, up to the next heading of its level or above.
pub(super) fn how_to_validate(text: &str) -> Option<String> {
    let mut lines = text.lines().skip_while(|l| {
        let l = l.trim_start();
        !(l.starts_with("## ") && l[3..].trim().to_lowercase().starts_with("cómo validar"))
    });
    lines.next()?;
    let body: Vec<&str> = lines
        .take_while(|l| !l.starts_with("## ") && !l.starts_with("# "))
        .collect();
    Some(body.join("\n").trim().to_string()).filter(|b| !b.is_empty())
}

/// The text of a `routine.done` (a routine's turn, or a chat turn that read a skill): `head` (the bot, the trigger, the routine and its task, the tools and skills), its answer (its start and end, 1000 characters), what its calls gave, `approval` (every tool call and every skill's «Qué requiere aprobación», whole: what the `unapproved` eval reads is never cut) and the «Cómo validar» of each skill it read (each cut to 1500).
pub(super) fn routine_text(
    app: &Shared,
    t: &Turn,
    head: &str,
    answer: &str,
    approval: &str,
) -> String {
    let cut = |text: &str, n: usize| text.chars().take(n).collect::<String>();
    let (answer, errors) = (ends(answer, 1000), t.looks.evidence());
    let mut text = format!("{head} · answer: {answer}{errors}{approval}");
    for name in &t.skills {
        let file = skill_file(app, t, name);
        let read = file.and_then(|f| std::fs::read_to_string(f).ok());
        read.as_deref()
            .and_then(how_to_validate)
            .into_iter()
            .for_each(|v| text += &format!("\n· how to validate «{name}»: {}", cut(&v, 1500)));
    }
    text
}

/// A judge's `score` of `eval` on event `event`: a `turn-quality` one is copied to the turn whose event it is, and may say that its run could not.
pub fn scored(app: &Shared, event: i64, eval: &str, score: Option<f64>) {
    if eval != EVAL {
        return;
    }
    let Some(score) = score else {
        return super::could_not::check(app, event); // no verdict: it is not left as a success by that
    };
    app.insert(
        "UPDATE turns SET score = ?1 WHERE event = ?2",
        params![score, event],
    );
    let ids = app.query("SELECT id FROM turns WHERE event = ?1", [event], |r| {
        r.get::<_, i64>(0)
    });
    ids.into_iter()
        .for_each(|id| app.emit("turn", json!({"id": id, "score": score})));
    super::could_not::check(app, event);
}

/// A turn row kept after its judge already answered (the judge runs as soon as the event is emitted): its score from `scores`.
pub(super) fn catch_up(app: &Shared, id: i64, event: Option<i64>) {
    let Some(event) = event else {
        return;
    };
    app.insert(
        "UPDATE turns SET score = (SELECT score FROM scores WHERE event = ?1 AND eval = ?2 AND score IS NOT NULL ORDER BY id DESC LIMIT 1) WHERE id = ?3 AND score IS NULL",
        params![event, EVAL, id],
    );
    super::could_not::check(app, event);
}
