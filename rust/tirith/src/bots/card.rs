//! A bot's card saved: a new bot, or a change to one.
use super::store::{find, with};
use crate::app::Shared;
use crate::server::s;
use serde_json::{json, Value};
/// Each route is on an event of the closed list but `schedule` and `webhook` (those are its routines), as a turn or a note.
fn hears_ok(b: &Value) -> bool {
    b["routes"].is_null()
        || b["routes"].as_array().is_some_and(|rs| {
            rs.iter().all(|r| {
                crate::factory::EVENTS
                    .split(';')
                    .any(|e| e == s(r, "on") && e != "schedule" && e != "webhook")
                    && matches!(s(r, "mode").as_str(), "" | "turn" | "note")
            })
        })
}
/// The card's name and colour, when the card is one a bot can have.
fn checked(b: &Value) -> Result<(String, String), String> {
    let (name, color) = (s(b, "name").trim().to_string(), s(b, "color"));
    (!name.is_empty()
        && color.len() == 7
        && color.starts_with('#')
        && color[1..].bytes().all(|c| c.is_ascii_hexdigit()))
    .then_some(())
    .ok_or("a bot needs a name and a colour #rrggbb")?;
    (matches!(s(b, "session").as_str(), "" | "chat" | "turn") && matches!(s(b, "access").as_str(), "" | "read" | "write") && hears_ok(b))
        .then_some(())
        .ok_or("a bot's session is chat or turn, its access read or write, and each route is on an event of the closed list (not schedule) as a turn or a note")?;
    Ok((name, color))
}
/// The card `b` laid over the bot: what it carries (even emptied) wins, what it does not carry stays.
fn change(bot: &mut Value, b: &Value, name: String, color: String) {
    let keep = |k: &str| {
        Some(s(b, k))
            .filter(|x| !x.is_empty() || !b[k].is_null())
            .unwrap_or_else(|| s(bot, k))
    };
    let (session, access, routes) = (
        Some(keep("session"))
            .filter(|x| !x.is_empty())
            .unwrap_or_else(|| s(bot, "session")),
        keep("access"),
        if b["routes"].is_null() {
            bot["routes"].clone()
        } else {
            b["routes"].clone()
        },
    ); // a session never empties (a bot always runs somehow); an access does (back to writing)
    (
        bot["name"],
        bot["shape"],
        bot["color"],
        bot["session"],
        bot["access"],
        bot["routes"],
        bot["dirty"],
    ) = (
        json!(name),
        b["shape"].clone(),
        json!(color),
        json!(session),
        json!(access),
        routes,
        json!(true),
    ); // `dirty`: a system bot's change is written as a layer in config.toml
}
/// A new bot (no `id`), or a change to one's card: its name, shape and colour, its `session` and `access`, and its routes (each on an event of the closed list but `schedule` — those are its routines — as a turn or a note, with its chat line and phones as written). What the body does not carry stays as it was.
pub(crate) fn save(app: &Shared, b: Value) -> Result<Value, String> {
    let (name, color) = checked(&b)?;
    with(app, |bots| {
        let (id, fresh) = (
            s(&b, "id"),
            s(&b, "id")
                .is_empty()
                .then(|| format!("{:06x}", rand::random::<u32>() >> 8)),
        );
        bots.extend(
            fresh
                .iter()
                .map(|id| json!({"id": id, "routines": [], "routes": []})),
        );
        let bot = find(bots, fresh.as_deref().unwrap_or(&id))?;
        change(bot, &b, name, color);
        Ok(bot.clone())
    })
}
