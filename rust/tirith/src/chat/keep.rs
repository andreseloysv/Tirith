//! A conversation's longest life: reading it and setting it.
use super::{clear::to_phones, sweep::sweep, words::parse_keep, written::written};
use crate::app::{now_s, Shared};
use serde_json::{json, Value};

/// The words a conversation's life is kept in, as written ("" is none): a project's own `chat_keep`, else its bot role's.
pub(super) fn keep_words(app: &Shared, conv: &str) -> String {
    let cfg = app.cfg();
    cfg.project(conv)
        .map(|p| p.chat_keep.clone())
        .filter(|w| !w.is_empty())
        .or_else(|| cfg.factory.role::<String>(conv, "chat_keep"))
        .unwrap_or_default()
}

/// A conversation's longest life in seconds, when it has one that reads; words nothing can read (written by hand) limit nothing.
pub(super) fn keep_s(app: &Shared, conv: &str) -> Option<u64> {
    parse_keep(&keep_words(app, conv)).ok().flatten()
}

/// Gives one conversation its longest life, in the owner's words («siempre» takes the limit away): written into config.toml, the page and the phones hear it, and what is already older goes at once.
pub(super) fn set_keep(app: &Shared, project: &str, words: &str) -> Result<Value, String> {
    (project.is_empty() || app.conv(project) == project)
        .then_some(())
        .ok_or(format!("unknown project '{project}'"))?;
    let conv = app.conv(project).to_string();
    let secs = parse_keep(words)?;
    crate::projects::rewrite(app, |text| {
        written(
            text,
            app.cfg().project(&conv).is_some(),
            &conv,
            secs.map(|_| words.trim()),
        )
    })?;
    let now = keep_words(app, &conv);
    app.emit(
        "chat-keep",
        json!({"project": conv, "keep": now, "keep_s": secs.unwrap_or(0)}),
    );
    to_phones(app, &conv, json!({"v": 1, "keep_s": secs.unwrap_or(0)}));
    sweep(app, now_s());
    Ok(json!({"project": conv, "keep": now, "keep_s": secs.unwrap_or(0)}))
}

/// What a conversation's longest life is now, in the owner's words and in seconds (0: none).
pub(super) fn keep_of(app: &Shared, project: &str) -> Value {
    let conv = app.conv(project);
    json!({"project": conv, "keep": keep_words(app, conv), "keep_s": keep_s(app, conv).unwrap_or(0)})
}
