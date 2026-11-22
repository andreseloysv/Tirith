//! Block and Unblock of a connector, written by the config writer (`projects::rewrite`, which reloads the config) so the next call of any open gateway is refused or runs again, with no restart.
use super::{block, kept};
use crate::app::Shared;
use serde_json::json;

/// Blocks (`on`) or unblocks connector `id`, for bot `bot` (its page id) or, when it is empty, for every bot; what was done.
pub fn change(app: &Shared, id: &str, bot: &str, on: bool) -> Result<String, String> {
    if id.is_empty() {
        return Err("name the connector".into());
    }
    if bot.is_empty() {
        crate::projects::rewrite(app, |t| block::blocked(t, id, on))?;
    } else if on {
        one(app, id, bot)?;
    } else {
        back(app, id, bot)?;
    }
    let whom = match bot {
        "" => "every bot".to_string(),
        b => format!("bot {b}"),
    };
    let said = format!(
        "connector {id} {} for {whom}",
        if on { "blocked" } else { "unblocked" }
    );
    app.event("", "", "connector-block", "owner", &said, "");
    app.emit("connectors", json!({}));
    Ok(said)
}

/// Takes bot `bot`'s grant of `id` out, remembering it.
fn one(app: &Shared, id: &str, bot: &str) -> Result<(), String> {
    let key = crate::bots::key_for(app, bot)
        .filter(|k| k.starts_with("bot-"))
        .ok_or_else(|| format!("no bot '{bot}' of the owner's"))?;
    let mut grant = None;
    crate::projects::rewrite(app, |t| {
        let (out, g) = block::taken(t, &key, id)?;
        grant = Some(g);
        Ok(out)
    })?;
    kept::keep(app, bot, id, grant.as_ref());
    Ok(())
}

/// Lays bot `bot`'s remembered grant of `id` back as it was; a bot that is gone has none to get back.
fn back(app: &Shared, id: &str, bot: &str) -> Result<(), String> {
    let grant = kept::kept(app, bot, id)
        .ok_or_else(|| format!("connector {id} is not blocked for bot {bot}"))?;
    let key = grant["key"].as_str().unwrap_or_default().to_string();
    let laid = crate::projects::rewrite(app, |t| block::laid(t, &key, id, &grant));
    if laid.as_ref().is_err_and(|e| e.contains("is gone")) {
        kept::keep(app, bot, id, None);
    }
    laid?;
    kept::keep(app, bot, id, None);
    Ok(())
}
