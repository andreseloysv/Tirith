//! A package imported: the bot it describes, made only for the hash the owner saw in the preview.
use super::check::checked;
use super::preview::preview;
use crate::app::Shared;
use crate::server::s;
use serde_json::{json, Value};

/// Instructions become one more skill of the bot, so every agent reads them where it reads skills.
const INSTRUCTIONS: &str = "instructions";

pub(in crate::bots) fn import(app: &Shared, p: &Value, hash: &str) -> Result<Value, String> {
    let shown = preview(p)?;
    (shown["hash"] == hash)
        .then_some(())
        .ok_or("the package is not the one that was previewed: preview it again")?;
    let c = checked(p)?;
    let mut skills = c.skills;
    let text = s(&p["bot"], "instructions");
    if !text.trim().is_empty() {
        let head = format!(
            "---\nname: {INSTRUCTIONS}\ndescription: What this bot's creator wrote for it\n---\n\n"
        );
        skills.retain(|(n, _)| n != INSTRUCTIONS);
        skills.push((INSTRUCTIONS.into(), format!("{head}{text}\n")));
    }
    let start = s(&p["bot"], "description");
    let bot = super::super::make::make(
        app,
        super::super::make::Parts {
            name: &c.name,
            shape: &c.shape,
            color: &c.color,
            routines: c.routines,
            skills,
            start: &start,
            origin: "package",
        },
    )?;
    Ok(json!({"bot": bot, "creator": shown["creator"], "to_grant": p["integrations"]}))
}
