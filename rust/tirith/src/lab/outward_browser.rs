//! Which browser and MCP calls of a shadow run act outward. A browser may look (open a page, read it, take a picture of it, find, wait) and click or type what does not send; a click on «Send», «Comprar», «Delete» or «Publicar», a typed text submitted, Enter, a form filled, an upload, a dialog accepted or code run in the page are refused. Another MCP tool whose name sends, writes or deletes is refused too.
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
/// The words of an MCP tool's name that change something outside.
const VERBS: [&str; 24] = [
    "send", "post", "reply", "publish", "delete", "remove", "create", "update", "write", "upload",
    "buy", "pay", "order", "share", "invite", "say", "message", "comment", "archive", "move",
    "trash", "submit", "edit", "forward",
];

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
        return action.split(['_', '-']).any(|w| VERBS.contains(&w));
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
