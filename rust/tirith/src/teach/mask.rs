//! A recorded step as it is kept: its known fields only, cut short, a secret's value and an address's secrets masked.
use super::names::named;
use super::url::mask_url;
use crate::server::s;
use serde_json::{json, Value};
/// What a masked value shows.
pub(super) const MASK: &str = "•••";
/// Whether the field holds a password, a one-time code or a card: its type, its autocomplete, its name or label, or the extension's own mark.
pub(super) fn secret(step: &Value) -> bool {
    let auto = s(step, "autocomplete").to_lowercase();
    step["sensitive"] == true
        || s(step, "type").eq_ignore_ascii_case("password")
        || auto == "one-time-code"
        || auto.starts_with("cc-")
        || auto.contains("password")
        || named(&s(step, "name"))
        || named(&s(step, "id"))
        || named(&s(step, "label"))
}
fn cut(text: &str, most: usize) -> String {
    text.chars().take(most).collect()
}
/// The step kept: page (url, title), click (text, role, selector) or type (label, value), at the time `at` (the daemon's own clock).
pub(super) fn kept(step: &Value, at: &str) -> Option<Value> {
    let kind = s(step, "kind");
    ["page", "click", "type"]
        .contains(&kind.as_str())
        .then_some(())?;
    let field = |key: &str, most: usize| cut(&s(step, key), most);
    let value = if secret(step) {
        MASK.to_string()
    } else {
        field("value", 500)
    };
    Some(json!({
        "kind": kind, "at": at, "url": cut(&mask_url(&s(step, "url")), 500), "title": field("title", 200),
        "text": field("text", 200), "role": field("role", 40), "selector": field("selector", 200),
        "label": field("label", 200), "value": value, "masked": value == MASK,
    }))
}
