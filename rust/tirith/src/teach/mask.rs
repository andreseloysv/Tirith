//! A recorded step as it is kept: its known fields only, cut short, and a secret's value masked.
use crate::server::s;
use serde_json::{json, Value};
/// What a masked value shows.
pub(super) const MASK: &str = "•••";
/// Words in a field's name or label that say it holds a card, a one-time code or a PIN.
const WORDS: [&str; 7] = ["cvv", "cvc", "otp", "pin", "código", "codigo", "tarjeta"];
/// Whether the field holds a password, a one-time code or a card: its type, its autocomplete, its name or label, or the extension's own mark.
pub(super) fn secret(step: &Value) -> bool {
    let auto = s(step, "autocomplete").to_lowercase();
    let named = |text: String| {
        let text = text.to_lowercase();
        text.contains("card")
            || text.contains("password")
            || text
                .split(|c: char| !c.is_alphanumeric())
                .any(|w| WORDS.contains(&w))
    };
    step["sensitive"] == true
        || s(step, "type").eq_ignore_ascii_case("password")
        || auto == "one-time-code"
        || auto.starts_with("cc-")
        || named(s(step, "name"))
        || named(s(step, "label"))
}
fn cut(step: &Value, key: &str, most: usize) -> String {
    s(step, key).chars().take(most).collect()
}
/// The step kept: page (url, title), click (text, role, selector) or type (label, value), at the time the extension gave or now.
pub(super) fn kept(step: &Value, now: &str) -> Option<Value> {
    let kind = s(step, "kind");
    ["page", "click", "type"]
        .contains(&kind.as_str())
        .then_some(())?;
    let at = Some(s(step, "at"))
        .filter(|a| a.len() == 24 && a.ends_with('Z') && a.starts_with("20"))
        .unwrap_or_else(|| now.to_string());
    let value = if secret(step) {
        MASK.to_string()
    } else {
        cut(step, "value", 500)
    };
    Some(json!({
        "kind": kind, "at": at, "url": cut(step, "url", 500), "title": cut(step, "title", 200),
        "text": cut(step, "text", 200), "role": cut(step, "role", 40), "selector": cut(step, "selector", 200),
        "label": cut(step, "label", 200), "value": value, "masked": value == MASK,
    }))
}
