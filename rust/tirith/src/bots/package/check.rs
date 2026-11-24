//! A package checked before anything is shown or made: its shape, its limits, and each part held to what a bot's own are held to.
use super::{FIELDS, FORMAT, VERSION};
use crate::server::s;
use serde_json::Value;

const SHAPES: [&str; 13] = [
    "round", "cloud", "tile", "star", "clover", "blob", "flower", "drop", "pill", "tri", "dome",
    "burst", "hex",
];
const MAX_TEXT: usize = 40_000;

fn list<'a>(p: &'a Value, k: &str, max: usize) -> Result<&'a [Value], String> {
    let l = p[k].as_array().map(Vec::as_slice).unwrap_or_default();
    (l.len() <= max)
        .then_some(l)
        .ok_or(format!("a package holds at most {max} {k}"))
}

/// The routines as a bot keeps them, each checked like one made on the card.
pub(super) fn routines(p: &Value) -> Result<Vec<Value>, String> {
    list(p, "routines", 10)?
        .iter()
        .map(|r| {
            let mut out = serde_json::json!({"on": true});
            super::super::routine::set(&mut out, r)?;
            Ok(out)
        })
        .collect()
}

/// The skills as (name, text): a valid name, a text within the limit, no two alike.
pub(super) fn skills(p: &Value) -> Result<Vec<(String, String)>, String> {
    let mut out: Vec<(String, String)> = vec![];
    for k in list(p, "skills", 20)? {
        let (name, text) = (s(k, "name"), s(k, "text"));
        let fine = super::super::library::named(&name)
            && !text.trim().is_empty()
            && text.len() <= MAX_TEXT;
        fine.then_some(()).ok_or(format!("skill '{name}': a name of lowercase letters, digits and dashes, and a text up to {MAX_TEXT} bytes"))?;
        (!out.iter().any(|(n, _)| *n == name))
            .then_some(())
            .ok_or(format!("two skills are called '{name}'"))?;
        out.push((name, text));
    }
    Ok(out)
}

/// The package's name and colour, its face shape and its routines and skills, or why it cannot be a bot.
pub(in crate::bots) struct Checked {
    pub name: String,
    pub color: String,
    pub shape: String,
    pub routines: Vec<Value>,
    pub skills: Vec<(String, String)>,
}

pub(in crate::bots) fn checked(p: &Value) -> Result<Checked, String> {
    let known = p
        .as_object()
        .is_some_and(|o| o.keys().all(|k| FIELDS.contains(&k.as_str())));
    (known && (p["format"] == FORMAT || p["format"] == "openbot-bot") && p["v"] == VERSION)
        .then_some(())
        .ok_or(
            "not a neobot-bot package, version 1, or it carries fields a package does not have",
        )?;
    let face = &p["bot"]["face"];
    let (name, color) = super::super::card::checked(
        &serde_json::json!({"name": p["bot"]["name"], "color": face["color"]}),
    )?;
    SHAPES
        .contains(&s(face, "shape").as_str())
        .then_some(())
        .ok_or("the face shape is not one NeoBot draws")?;
    (s(&p["bot"], "description").len() <= 2_000 && s(&p["bot"], "instructions").len() <= MAX_TEXT)
        .then_some(())
        .ok_or("the description or the instructions are too long")?;
    Ok(Checked {
        name,
        color,
        shape: s(face, "shape"),
        routines: routines(p)?,
        skills: skills(p)?,
    })
}
