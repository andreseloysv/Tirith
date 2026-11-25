//! `neobot safari click <target> [<url>]` and `type <target> <text> [<url>]`: the owner lets the chat roles click
//! and type in their Safari without asking (their choice of 2026-11-23), and nobody turns on a Safari setting for
//! it: the daemon runs its own fixed script (assets/safari-ax-find.js, safari-ax-look.js and safari-ax.js,
//! JavaScript for Automation) through macOS Accessibility, the agent's words as the script's arguments, never as
//! code. `find [<words>] [<url>]` lists what can be pressed and typed into (the words those two take, also inside
//! frames, where a page's text is not read) and `wait <words> [<url>]` answers once something with the words is there. The first time, macOS asks the
//! person to let NeoBot use Accessibility. A URL first brings that tab forward with Safari's own scripting, as
//! Accessibility sees only the page a window shows. The script never reads cookies, storage or passwords.

/// How the commands that act are written.
pub const USAGE: &str = "neobot safari click <target> [<url>] | type <target> <text> [<url>] | find [<words>] [<url>] | wait <words> [<url>]";

/// The fixed script: finding the element the words name (WebKit's own search, not a walk of the whole page), saying what is there, then acting on it.
const AX: &str = concat!(
    include_str!("../../../assets/safari-ax-find.js"),
    "\n",
    include_str!("../../../assets/safari-ax-look.js"),
    "\n",
    include_str!("../../../assets/safari-ax.js")
);

/// The osascript arguments that run the fixed script for `cmd` with the agent's words.
pub fn act_args(cmd: &str, target: &str, text: &str, url: &str) -> Result<Vec<String>, String> {
    let control = |s: &str| s.chars().any(char::is_control);
    let named = !target.trim().is_empty();
    let fits = (matches!(cmd, "click" | "type" | "wait") && named || cmd == "find")
        && !control(target)
        && !control(url)
        && text.chars().all(|c| c == '\n' || !c.is_control());
    fits.then_some(()).ok_or_else(|| USAGE.to_string())?;
    Ok(["-l", "JavaScript", "-e", AX, cmd, target, text, url]
        .map(String::from)
        .to_vec())
}

/// The AppleScript that makes the first tab whose URL starts with `url` its window's current tab, that window first.
pub fn show_script(url: &str) -> String {
    let url = url.replace('\\', "\\\\").replace('"', "\\\"");
    format!("tell application \"Safari\"\nrepeat with w in windows\nrepeat with t in tabs of w\nif (URL of t) starts with \"{url}\" then\nset current tab of w to t\nset index of w to 1\nreturn \"shown\"\nend if\nend repeat\nend repeat\nerror \"no tab starts with that URL\"\nend tell")
}
