//! `neobot chrome|edge open|read|find|click|type|wait` on Windows: the owner's own Chrome or Edge, the one they have open,
//! as Safari is on a Mac. The daemon runs its own fixed script (assets/win-browser.ps1) through Windows UI
//! Automation, out of sight; the agent's words go in as environment variables, never as code. Nothing is
//! installed in the browser, and no permission is asked: UI Automation is every Windows program's to use.
//! Here: the words of a command, checked, and what the script answered. `uia_run.rs` runs it.
use serde_json::Value;

/// The Windows process of the browser a command names, and how people call it.
pub fn browser(which: &str) -> Option<(&'static str, &'static str)> {
    match which {
        "chrome" => Some(("chrome", "Chrome")),
        "edge" => Some(("msedge", "Edge")),
        _ => None,
    }
}

/// The script's variables for one command, its words checked: an address to open is a web address in one piece.
pub fn vars(process: &str, b: &Value) -> Result<Vec<(&'static str, String)>, String> {
    let word = |k: &str| b[k].as_str().unwrap_or("").to_string();
    let (cmd, url, target, text) = (word("cmd"), word("url"), word("target"), word("text"));
    let control = |s: &str| s.chars().any(char::is_control);
    let web = url.starts_with("https://") || url.starts_with("http://");
    let fits = match cmd.as_str() {
        "open" => web && !url.chars().any(|c| c.is_whitespace() || c == '"'),
        "read" | "find" => true,
        "click" | "type" | "wait" => !target.trim().is_empty(),
        _ => false,
    };
    let clean =
        !control(&url) && !control(&target) && text.chars().all(|c| c == '\n' || !c.is_control());
    (fits && clean).then_some(()).ok_or_else(|| {
        "open <url> | read [<url>] | find [<words>] [<url>] | click <target> [<url>] | type <target> <text> [<url>] | wait <words> [<url>]"
            .to_string()
    })?;
    let named = [cmd, process.to_string(), url, target, text];
    let keys = [
        "NEOBOT_CMD",
        "NEOBOT_APP",
        "NEOBOT_URL",
        "NEOBOT_TARGET",
        "NEOBOT_TEXT",
    ];
    Ok(keys.into_iter().zip(named).collect())
}

/// What the script said, as the command's answer: its `ERROR …` is an error, named after the browser.
#[cfg(any(windows, test))]
pub fn answer(name: &str, ok: bool, out: &str, err: &str) -> Result<String, String> {
    let said = out.trim_end();
    match (ok, said.strip_prefix("ERROR ")) {
        (true, None) => Ok(said.to_string()),
        (true, Some(why)) => Err(format!("{name}: {why}")),
        (false, _) => Err(format!(
            "{name}: {}",
            err.lines()
                .next()
                .unwrap_or("it could not be reached")
                .trim()
        )),
    }
}
