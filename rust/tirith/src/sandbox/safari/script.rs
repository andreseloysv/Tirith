//! The fixed AppleScript of `tirith safari open <url>` / `read [<url>]`.
use crate::app::Shared;

/// The daemon (outside the sandbox, where macOS accepts its Apple Events) runs it against Safari for a chat role — the managers and bots, never a lane or the CEO. The agent gives only a URL, escaped into the fixed text; it never sends a script. A read gives URL, title and text of the current tab, or of the first tab whose URL starts with the one given.
pub fn safari_script(
    app: &Shared,
    lane: &str,
    role: &str,
    cmd: &str,
    url: &str,
) -> Result<String, String> {
    chat_role(app, lane, role)?;
    let url = url.replace('\\', "\\\\").replace('"', "\\\"");
    match (cmd, url.is_empty(), url.chars().any(|c| c.is_control())) {
        ("open", false, false) => Ok(format!("tell application \"Safari\"\nactivate\nopen location \"{url}\"\nend tell")),
        ("read", true, false) => Ok("tell application \"Safari\" to return (URL of current tab of front window) & linefeed & (name of current tab of front window) & linefeed & (text of current tab of front window)".into()),
        ("read", false, false) => Ok(format!("tell application \"Safari\"\nrepeat with w in windows\nrepeat with t in tabs of w\nif (URL of t) starts with \"{url}\" then return (URL of t) & linefeed & (name of t) & linefeed & (text of t)\nend repeat\nend repeat\nerror \"no tab starts with that URL\"\nend tell")),
        _ => Err("tirith safari open <url> | tirith safari read [<url>]".into()),
    }
}

/// Only a chat role outside any lane may drive Safari.
pub(super) fn chat_role(app: &Shared, lane: &str, role: &str) -> Result<(), String> {
    let chat = lane.is_empty()
        && app.cfg().factory.role::<String>(role, "session").as_deref() == Some("chat");
    chat.then_some(()).ok_or(
        "tirith safari is for the managers and bots (the chat roles), not a lane or the CEO"
            .to_string(),
    )
}
