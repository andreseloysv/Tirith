//! A routine's own model and effort, chosen on its card: kept in the bot's `routines.toml` (`tuning.rs`), the file
//! the Coach's experiments change too, tracked by the bot's repository. A quick routine on the cheap model need
//! not make the bot's every conversation cheap, and a careful one need not make every greeting slow.
use super::folder::folder;
use super::tuning::{self, Tuning};
use crate::app::Shared;
use crate::server::s;
use serde_json::{json, Value};

/// A model or an effort as a card may name it: one word of letters, digits, dots and dashes.
fn word(w: &str) -> bool {
    !w.is_empty()
        && w.len() <= 80
        && w.chars()
            .all(|c| c.is_ascii_alphanumeric() || "._-".contains(c))
}

/// The card's `model` and `effort` for a routine of the bot whose conversation is `key`, laid over what it `had`:
/// each a word, "" for the bot's own, absent for no change. The model must be one the bot's agent runs.
fn asked(app: &Shared, key: &str, b: &Value, had: Tuning) -> Result<Tuning, String> {
    let one = |k: &str, had: Option<String>| match &b[k] {
        Value::Null => Ok(had),
        Value::String(w) if w.is_empty() => Ok(None),
        Value::String(w) if word(w) => Ok(Some(w.clone())),
        _ => Err(format!(
            "a routine's {k} is one word, or empty for the bot's own"
        )),
    };
    let (effort, model) = (one("effort", had.0)?, one("model", had.1)?);
    let agent = crate::models::role(app, key).agent;
    let wrong = crate::models::mismatch(&app.cfg(), &agent, model.as_deref());
    wrong.map_or(Ok((effort, model)), Err)
}

/// A routine saved from its card (`routine.rs`), with what the card says of its own model and effort: checked
/// before anything is saved, kept once the routine has its id, and gone with a deleted routine.
pub(super) fn saved(app: &Shared, id: String, b: Value) -> Result<Value, String> {
    let key = super::key_for(app, &id).ok_or(format!("no bot '{id}'"))?;
    let named = !b["model"].is_null() || !b["effort"].is_null();
    asked(app, &key, &b, (None, None))?;
    let done = super::routine::routine(app, id, b.clone())?;
    let dir = folder(app, &key);
    let rid = Some(s(&done, "id")).filter(|r| !r.is_empty());
    let (rid, tuning) = match rid {
        Some(rid) if named => {
            let had = tuning::read(&dir, &rid).unwrap_or_default();
            (rid, asked(app, &key, &b, had)?)
        }
        None if b["delete"] == true && !s(&b, "id").is_empty() => (s(&b, "id"), (None, None)),
        _ => return Ok(done),
    };
    if tuning::write(&dir, &rid, &tuning)? {
        super::versioned(app, &key, "routine");
    }
    Ok(done)
}

/// Each routine of `bot` with its own `model` and `effort` ("" for the bot's own), as its card shows them.
pub(super) fn shown(app: &crate::app::App, mut bot: Value) -> Value {
    let dir = folder(app, &s(&bot, "key"));
    for r in bot["routines"].as_array_mut().into_iter().flatten() {
        let (effort, model) = tuning::read(&dir, &s(r, "id")).unwrap_or_default();
        r["effort"] = json!(effort.unwrap_or_default());
        r["model"] = json!(model.unwrap_or_default());
    }
    bot
}

#[cfg(test)]
#[path = "../../checks/routine_model.rs"]
mod tests;
