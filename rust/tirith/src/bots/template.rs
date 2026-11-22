//! A bot template read and checked: its name, face, routine, first words and skill.
use crate::app::App;
use crate::server::s;
use serde_json::{json, Value};

/// One template, ready to become a bot.
pub(super) struct Template {
    /// Also its skill's name, and that skill's folder.
    pub id: String,
    pub name: String,
    pub shape: String,
    pub color: String,
    /// As a card's routine is kept, with no id yet.
    pub routine: Value,
    /// What the bot says first: what it needs from the owner.
    pub start: String,
    pub skill: String,
}

impl Template {
    /// What the page shows of it.
    pub fn shown(&self) -> Value {
        json!({"id": self.id, "name": self.name, "shape": self.shape, "color": self.color, "routine": self.routine})
    }
}

/// The template `id` from its two texts, held to what a card and a routine are held to.
pub(super) fn parse(id: &str, bot: &str, skill: &str) -> Result<Template, String> {
    let bad = |e: String| format!("bot template '{id}': {e}");
    super::library::named(id)
        .then_some(())
        .ok_or_else(|| bad("its id is lowercase letters, digits and dashes".into()))?;
    let table: toml::Table = bot.parse().map_err(|e| bad(format!("{e}")))?;
    let t = json!(table);
    let card = json!({"name": t["name"], "color": t["face"]["color"]});
    let (name, color) = super::card::checked(&card).map_err(bad)?;
    let mut routine = json!({"on": true});
    super::routine::set(&mut routine, &t["routine"]).map_err(bad)?;
    Ok(Template {
        id: id.to_string(),
        name,
        shape: s(&t["face"], "shape"),
        color,
        routine,
        start: s(&t, "start"),
        skill: skill.to_string(),
    })
}

/// The factory's templates, by id.
pub(super) fn templates(app: &App) -> Result<Vec<Template>, String> {
    crate::factory::bot_templates(&app.cfg().raw)?
        .iter()
        .map(|(id, (bot, skill))| parse(id, bot, skill))
        .collect()
}
