//! Fitting a capture in its size: a result's text, cut on a character to the room left.
use serde_json::{json, Value};

/// A result's text: a string, or the text blocks of a list.
pub(super) fn text_of(v: &Value) -> String {
    v.as_str().map(String::from).unwrap_or_else(|| {
        let blocks = v.as_array().into_iter().flatten();
        let texts: Vec<&str> = blocks.filter_map(|b| b["text"].as_str()).collect();
        texts.join("\n")
    })
}

/// `call` with as much of `text` as its result as fits in `room` bytes of JSON, and whether the text was cut; none when not even an empty result fits.
pub(super) fn fitted(call: &Value, text: &str, room: usize) -> (Option<Value>, bool) {
    let mut keep = text.len();
    loop {
        let t = upto(text, keep);
        let mut c = call.clone();
        c["result"] = json!(t);
        let n = c.to_string().len();
        if n <= room {
            return (Some(c), t.len() < text.len());
        }
        let Some(()) = (keep > 0).then_some(()) else {
            return (None, true);
        };
        keep = keep.saturating_sub((n - room).max(1));
    }
}

/// The longest start of `text` of at most `max` bytes that ends on a character.
fn upto(text: &str, max: usize) -> &str {
    let end = (0..=max.min(text.len()))
        .rev()
        .find(|i| text.is_char_boundary(*i));
    &text[..end.unwrap_or(0)]
}
