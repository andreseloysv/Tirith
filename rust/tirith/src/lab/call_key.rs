//! What a tool call asks for, so a replay answers only a call that asks for the same thing: the tool and its meaningful input (the URL, the query, the command with its spaces normalised), never its purely descriptive fields.
use serde_json::Value;

/// Fields that only describe a call (why it is made, how long to wait), whatever the tool.
const DESCRIPTIVE: [&str; 6] = [
    "description",
    "explanation",
    "reason",
    "justification",
    "timeout",
    "run_in_background",
];

/// The words of `text` with one space between them.
fn words(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The key of a call of `tool` (under the rules' names) with `input`: two calls with one key ask for the same thing.
pub(super) fn key(tool: &str, input: &Value) -> String {
    let text = |k: &str| input[k].as_str();
    let asked = if let Some(url) = text("url") {
        url.trim().trim_end_matches('/').to_string()
    } else if let Some(query) = text("query") {
        words(query)
    } else if let Some(command) = text("command").or(text("cmd")) {
        words(command)
    } else {
        let mut rest = input.clone();
        let fields = rest.as_object_mut();
        fields
            .into_iter()
            .for_each(|o| DESCRIPTIVE.iter().for_each(|k| drop(o.remove(*k))));
        rest.to_string()
    };
    format!("{tool} {asked}")
}
