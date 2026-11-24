//! A bot's look beyond its shape and colour: its material (solid, a gradient preset or one of two colours,
//! or glass), its eye colour, and each material's own settings. The card carries them flat, beside `shape`
//! and `color`; in config they live in the role's `face` table, and only when set (a bot without them is
//! solid in its colour with ink eyes, as every bot was). The preset lists are the page's face's own.
use crate::server::s;
use serde_json::{json, Value};
/// The gradient presets a card may name (`custom`: its two colours and angle instead).
const GRADIENTS: &str = "custom;electric-dusk;ocean-signal;warm-flare;mint-violet;midnight-plum;peach-sky;acid-lime;blue-hour;porcelain-bloom;lilac-breath;blue-milk;peach-haze";
/// The glass presets a card may name.
const GLASSES: &str = "iridescent-orb;prism;aurora;candy;opal";
/// The look's keys as the card and the stored bot carry them, beside `shape` and `color`.
pub(super) const KEYS: [&str; 6] = ["material", "eye", "gradient", "gstart", "gend", "glass"];
fn hex(x: &str) -> bool {
    x.is_empty()
        || (x.len() == 7 && x.starts_with('#') && x[1..].bytes().all(|c| c.is_ascii_hexdigit()))
}
fn one_of(list: &str, x: &str) -> bool {
    x.is_empty() || list.split(';').any(|p| p == x)
}
/// The card's look, when it is one a bot can have.
pub(super) fn look_ok(b: &Value) -> bool {
    matches!(
        s(b, "material").as_str(),
        "" | "solid" | "gradient" | "rainbow-glass"
    ) && hex(&s(b, "eye"))
        && one_of(GRADIENTS, &s(b, "gradient"))
        && one_of(GLASSES, &s(b, "glass"))
        && hex(&s(b, "gstart"))
        && hex(&s(b, "gend"))
        && (b["gangle"].is_null()
            || b["gangle"]
                .as_f64()
                .is_some_and(|a| (0.0..=360.0).contains(&a)))
}
/// The card's look laid over the bot: what it carries (even emptied) wins, what it does not carry stays.
pub(super) fn lay(bot: &mut Value, b: &Value) {
    for k in KEYS {
        if !b[k].is_null() {
            bot[k] = json!(s(b, k));
        }
    }
    if b.as_object().is_some_and(|o| o.contains_key("gangle")) {
        bot["gangle"] = b["gangle"].clone();
    }
}
/// A stored bot's look into its role's `face` table: a key only when set, so a plain bot's face stays two keys.
pub(super) fn face_into(role: &mut Value, b: &Value) {
    for k in KEYS {
        if !s(b, k).is_empty() {
            role["face"][k] = json!(s(b, k));
        }
    }
    if b["gangle"].is_number() {
        role["face"]["gangle"] = b["gangle"].clone();
    }
}
