//! Which browser and MCP calls of a shadow run act outward. A browser may look (open a page, read it, take a picture of it, find, wait) and click or type what does not send; a click on «Send», «Comprar», «Delete» or «Publicar», a typed text submitted, Enter, a form filled, an upload, a dialog accepted or code run in the page are refused. Another MCP tool is refused unless its name says that it reads and nothing in it sends, writes or deletes.
use super::outward_words::{READ, VERBS};
use regex::Regex;
use serde_json::Value;
use std::sync::LazyLock;

/// What a browser does that only looks.
const LOOKS: [&str; 14] = [
    "navigate",
    "navigate_back",
    "snapshot",
    "take_screenshot",
    "screenshot",
    "find",
    "tabs",
    "wait_for",
    "network_requests",
    "network_request",
    "console_messages",
    "resize",
    "close",
    "emulate_media",
];
/// A control whose words send, buy, delete or publish, in English and Spanish.
static SENDS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(?:send|enviar|env[ií]a\w*|submit|buy|comprar?|purchase|pay|pagar?|checkout|order|pedir|delete|borrar?|elimina\w*|remove|quitar?|trash|papelera|publish|publica\w*|post|reply|responder?|forward|reenviar?|share|compartir|archive|archivar?|confirm|confirmar?|accept|aceptar?)\b").unwrap()
});

/// A name's words, «getUser» and «get_user» alike.
fn words(name: &str) -> Vec<String> {
    let mut spaced = String::new();
    for c in name.chars() {
        if c.is_uppercase() {
            spaced.push(' ');
        }
        spaced.extend(c.to_lowercase());
    }
    let parts = spaced.split(|c: char| !c.is_alphanumeric());
    parts.filter(|w| !w.is_empty()).map(String::from).collect()
}

/// True when an MCP tool called `name` is not clearly a read: no word of its name says it reads (unless its server declared it read-only), or one says it sends, writes or deletes.
pub fn name_acts(name: &str, declared_read: bool) -> bool {
    let said = words(name);
    let has = |list: &[&str]| said.iter().any(|w| list.contains(&w.as_str()));
    !(declared_read || has(&READ)) || has(&VERBS)
}

/// True for a browser or MCP call (`tool` as named) that acts outward.
pub fn acts(tool: &str, input: &Value) -> bool {
    let lower = tool.to_lowercase();
    let browser = lower.contains("browser");
    if !browser && !tool.starts_with("mcp__") {
        return false;
    }
    let action = lower.rsplit("__").next().unwrap_or(&lower);
    let action = action.strip_prefix("browser_").unwrap_or(action);
    if !browser {
        return name_acts(tool.rsplit("__").next().unwrap_or(tool), false);
    }
    let said = |k: &str| input[k].as_str().unwrap_or("").to_string();
    match action {
        a if LOOKS.contains(&a) => false,
        "hover" | "select_option" => false,
        "click" => SENDS.is_match(&format!("{} {}", said("element"), said("text"))),
        "type" => input["submit"] == true || SENDS.is_match(&said("element")),
        "press_key" => matches!(said("key").to_lowercase().as_str(), "enter" | "return"),
        "handle_dialog" => input["accept"] != false,
        _ => true, // a form filled, an upload, a drag or a drop, code run in the page, anything unknown
    }
}
